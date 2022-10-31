use std::{
    collections::{HashMap, HashSet},
    path::Path,
};

use anyhow::Result;
use axum::{
    body::Bytes,
    extract::{BodyStream, State},
    http::StatusCode,
    response::IntoResponse,
    BoxError, TypedHeader,
};
use flate2::{read::GzDecoder, write::GzEncoder, Compression};
use futures::{Stream, TryStreamExt};
use headers::authorization::Bearer;
use lapce_rpc::plugin::VoltMetadata;
use s3::Bucket;
use serde::{Deserialize, Serialize};
use tar::Archive;
use tokio_util::io::StreamReader;
use toml_edit::easy as toml;

use crate::db::{find_api_token, find_user, DbPool};

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
struct IconTheme {
    pub icon_theme: IconThemeConfig,
}

#[derive(Serialize, Deserialize)]
struct IconThemeConfig {
    pub ui: HashMap<String, String>,
    pub foldername: HashMap<String, String>,
    pub filename: HashMap<String, String>,
    pub extension: HashMap<String, String>,
}

pub async fn publish(
    State(db_pool): State<DbPool>,
    State(bucket): State<Bucket>,
    TypedHeader(token): TypedHeader<headers::Authorization<Bearer>>,
    body: BodyStream,
) -> impl IntoResponse {
    let api_token = {
        let mut conn = db_pool.write.get().await.unwrap();
        match find_api_token(&mut conn, token.token()).await {
            Ok(api_token) => api_token,
            Err(_) => {
                return (axum::http::StatusCode::UNAUTHORIZED, "API Token Invalid").into_response()
            }
        }
    };

    let user = {
        let mut conn = db_pool.read.get().await.unwrap();
        find_user(&mut conn, api_token.user_id).await.unwrap()
    };

    let dir = tempfile::TempDir::new().unwrap();
    let dest = tempfile::TempDir::new().unwrap();
    let tar_gz = dir.path().join("volt.tar.gz");
    stream_to_file(&tar_gz, body).await.unwrap();

    {
        let tar_gz = tar_gz.clone();
        let dir_path = dir.path().to_path_buf();
        tokio::task::spawn_blocking(move || {
            let tar_gz = std::fs::File::open(tar_gz).unwrap();
            let tar = GzDecoder::new(tar_gz);
            let mut archive = Archive::new(tar);
            archive.unpack(dir_path).unwrap();
        })
        .await
        .unwrap();
    }

    let volt_path = dir.path().join("volt.toml");
    if !volt_path.exists() {
        return (StatusCode::BAD_REQUEST, "volt.toml doens't exist").into_response();
    }

    let s = tokio::fs::read_to_string(&volt_path).await.unwrap();
    let volt: VoltMetadata = match toml::from_str(&s) {
        Ok(volt) => volt,
        Err(_) => return (StatusCode::BAD_REQUEST, "volt.tmol format invalid").into_response(),
    };

    let s3_folder = format!("{}/{}/{}", user.gh_login, volt.name, volt.version);

    let dest_volt_path = dest.path().join("volt.toml");
    tokio::fs::copy(volt_path, dest_volt_path).await.unwrap();

    if let Some(wasm) = volt.wasm.as_ref() {
        let wasm_path = dir.path().join(wasm);
        if !wasm_path.exists() {
            return (StatusCode::BAD_REQUEST, "wasm {wasm} not found").into_response();
        }

        let dest_wasm = dest.path().join(wasm);
        tokio::fs::create_dir_all(dest_wasm.parent().unwrap())
            .await
            .unwrap();
        tokio::fs::copy(wasm_path, dest_wasm).await.unwrap();
    } else if let Some(themes) = volt.color_themes.as_ref() {
        if themes.is_empty() {
            return (StatusCode::BAD_REQUEST, "no color theme provided").into_response();
        }
        for theme in themes {
            let theme_path = dir.path().join(theme);
            if !theme_path.exists() {
                return (
                    StatusCode::BAD_REQUEST,
                    format!("color theme {theme} not found"),
                )
                    .into_response();
            }

            let dest_theme = dest.path().join(theme);
            tokio::fs::create_dir_all(dest_theme.parent().unwrap())
                .await
                .unwrap();
            tokio::fs::copy(theme_path, dest_theme).await.unwrap();
        }
    } else if let Some(themes) = volt.icon_themes.as_ref() {
        if themes.is_empty() {
            return (StatusCode::BAD_REQUEST, "no icon theme provided").into_response();
        }
        for theme in themes {
            let theme_path = dir.path().join(theme);
            if !theme_path.exists() {
                return (
                    StatusCode::BAD_REQUEST,
                    format!("icon theme {theme} not found"),
                )
                    .into_response();
            }

            let dest_theme = dest.path().join(theme);
            tokio::fs::create_dir_all(dest_theme.parent().unwrap())
                .await
                .unwrap();
            tokio::fs::copy(&theme_path, &dest_theme).await.unwrap();

            let s = tokio::fs::read_to_string(&theme_path).await.unwrap();
            let theme_config: IconTheme = match toml::from_str(&s) {
                Ok(config) => config,
                Err(_) => {
                    return (
                        StatusCode::BAD_REQUEST,
                        format!("icon theme {theme} format invalid"),
                    )
                        .into_response();
                }
            };

            let mut icons = HashSet::new();
            icons.extend(theme_config.icon_theme.ui.values());
            icons.extend(theme_config.icon_theme.filename.values());
            icons.extend(theme_config.icon_theme.foldername.values());
            icons.extend(theme_config.icon_theme.extension.values());

            let cwd = theme_path.parent().unwrap();
            let dest_cwd = dest_theme.parent().unwrap();

            for icon in icons {
                let icon_path = cwd.join(icon);
                if !icon_path.exists() {
                    return (StatusCode::BAD_REQUEST, format!("icon {icon} not found"))
                        .into_response();
                }
                let dest_icon = dest_cwd.join(icon);
                tokio::fs::create_dir_all(dest_icon.parent().unwrap())
                    .await
                    .unwrap();
                tokio::fs::copy(icon_path, dest_icon).await.unwrap();
            }
        }
    } else {
        return (StatusCode::BAD_REQUEST, "not a valid plugin").into_response();
    }

    let readme_path = dir.path().join("README.md");
    if readme_path.exists() {
        let readme = tokio::fs::read(&readme_path).await.unwrap();
        bucket
            .put_object(
                &format!("{}/{}/{}/readme", user.gh_login, volt.name, volt.version),
                &readme,
            )
            .await
            .unwrap();
        tokio::fs::copy(readme_path, dest.path().join("README.md"))
            .await
            .unwrap();
    }

    if let Some(icon) = volt.icon.as_ref() {
        let icon_path = dir.path().join(icon);
        if icon_path.exists() {
            let icon_content = tokio::fs::read(&icon_path).await.unwrap();
            bucket
                .put_object(
                    &format!("{}/{}/{}/icon", user.gh_login, volt.name, volt.version),
                    &icon_content,
                )
                .await
                .unwrap();

            let dest_icon = dest.path().join(icon);
            tokio::fs::create_dir_all(dest_icon.parent().unwrap())
                .await
                .unwrap();
            tokio::fs::copy(icon_path, dest_icon).await.unwrap();
        }
    }

    let dest_tar_gz_dir = tempfile::TempDir::new().unwrap();
    let dest_tar_gz = dest_tar_gz_dir.path().join("volt.tar.gz");
    {
        let tar_gz = dest_tar_gz.clone();
        tokio::task::spawn_blocking(move || {
            let tar_gz = std::fs::File::create(tar_gz).unwrap();
            let encoder = GzEncoder::new(tar_gz, Compression::default());
            let mut tar = tar::Builder::new(encoder);
            tar.append_dir_all(".", dest.path()).unwrap();
        })
        .await
        .unwrap();
    }

    let volt_content = tokio::fs::read(&dest_tar_gz).await.unwrap();
    bucket
        .put_object(format!("{}/volt.tar.gz", s3_folder), &volt_content)
        .await
        .unwrap();

    ().into_response()
}

async fn stream_to_file<S, E>(path: &Path, stream: S) -> Result<()>
where
    S: Stream<Item = Result<Bytes, E>>,
    E: Into<BoxError>,
{
    let body_with_io_error =
        stream.map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, err));
    let body_reader = StreamReader::new(body_with_io_error);
    futures::pin_mut!(body_reader);

    let mut tar_gz = tokio::fs::File::create(path).await?;
    tokio::io::copy(&mut body_reader, &mut tar_gz).await?;
    Ok(())
}
