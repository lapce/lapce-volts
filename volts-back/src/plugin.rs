use std::collections::{HashMap, HashSet};

use anyhow::Result;
use axum::{
    body::Bytes,
    extract::{BodyStream, Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    BoxError, Json, TypedHeader,
};
use diesel::{BelongingToDsl, BoolExpressionMethods, ExpressionMethods, GroupedBy};
use diesel::{PgTextExpressionMethods, QueryDsl};
use diesel_async::RunQueryDsl;
use flate2::{read::GzDecoder, write::GzEncoder, Compression};
use futures::{FutureExt, Stream, TryStreamExt};
use headers::authorization::Bearer;
use lapce_rpc::plugin::VoltMetadata;
use s3::Bucket;
use serde::{Deserialize, Serialize};
use tar::Archive;
use tokio_util::io::StreamReader;
use toml_edit::easy as toml;
use volts_core::{
    db::{
        models::{Plugin, User, Version},
        schema::{plugins, users, versions},
    },
    EncodePlugin, PluginList,
};

use crate::db::{
    find_api_token, find_plugin, find_plugin_version, find_user, find_user_by_gh_login,
    modify_plugin_version_yank, DbPool, NewPlugin, NewVersion,
};

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

#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    q: Option<String>,
    sort: Option<String>,
    limit: Option<usize>,
    offset: Option<usize>,
}

pub async fn search(
    Query(query): Query<SearchQuery>,
    State(db_pool): State<DbPool>,
) -> impl IntoResponse {
    let limit = query.limit.unwrap_or(10).min(100);
    let offset = query.offset.unwrap_or(0);
    let mut conn = db_pool.read.get().await.unwrap();
    let mut sql_query = plugins::table
        .inner_join(users::dsl::users)
        .filter(diesel::expression::exists::exists(
            versions::table
                .filter(versions::plugin_id.eq(plugins::id))
                .filter(versions::yanked.eq(false)),
        ))
        .into_boxed();
    let mut total: i64 = 0;
    let mut had_query = false;
    if let Some(q) = query.q.as_ref() {
        if !q.is_empty() {
            let q = format!("%{q}%");

            let filter = plugins::name
                .ilike(q.clone())
                .or(plugins::description.ilike(q.clone()))
                .or(plugins::display_name.ilike(q));
            sql_query = sql_query.filter(filter.clone());

            had_query = true;
            total = plugins::table
                .filter(filter)
                .filter(diesel::expression::exists::exists(
                    versions::table
                        .filter(versions::plugin_id.eq(plugins::id))
                        .filter(versions::yanked.eq(false)),
                ))
                .count()
                .get_result(&mut conn)
                .await
                .unwrap();
        }
    }
    if !had_query {
        total = plugins::table
            .filter(diesel::expression::exists::exists(
                versions::table
                    .filter(versions::plugin_id.eq(plugins::id))
                    .filter(versions::yanked.eq(false)),
            ))
            .count()
            .get_result(&mut conn)
            .await
            .unwrap();
    }

    sql_query = sql_query.offset(offset as i64).limit(limit as i64);
    match query.sort.as_deref() {
        Some("created") => {
            sql_query = sql_query.order(plugins::created_at.desc());
        }
        Some("updated") => {
            sql_query = sql_query.order(plugins::updated_at.desc());
        }
        _ => {
            sql_query = sql_query.order(plugins::downloads.desc());
        }
    }
    let data: Vec<(Plugin, User)> = sql_query.load(&mut conn).await.unwrap();

    let plugins = data.iter().map(|(p, u)| p).collect::<Vec<&Plugin>>();

    let versions: Vec<Version> = Version::belonging_to(plugins.as_slice())
        .filter(versions::yanked.eq(false))
        .load(&mut conn)
        .await
        .unwrap();

    let versions = versions.grouped_by(&plugins).into_iter().map(|versions| {
        versions
            .into_iter()
            .filter_map(|v| Some((semver::Version::parse(&v.num).ok()?, v)))
            .max_by_key(|(v, _)| v.clone())
    });

    let plugins: Vec<EncodePlugin> = versions
        .zip(data)
        .filter_map(|(v, (p, u))| {
            let version = v?.1;
            Some(EncodePlugin {
                id: p.id,
                name: p.name,
                author: u.gh_login,
                version: version.num,
                display_name: p.display_name,
                description: p.description,
                downloads: p.downloads,
                repository: p.repository,
                updated_at_ts: p.updated_at.timestamp(),
                updated_at: p.updated_at.format("%Y-%m-%d %H:%M:%S").to_string(),
                released_at: version.created_at.format("%Y-%m-%d %H:%M:%S").to_string(),
                wasm: p.wasm,
            })
        })
        .collect();

    Json(PluginList {
        total,
        limit,
        offset,
        plugins,
    })
}

pub async fn meta(
    State(bucket): State<Bucket>,
    State(db_pool): State<DbPool>,
    Path((author, name, version)): Path<(String, String, String)>,
) -> impl IntoResponse {
    let mut conn = db_pool.read.get().await.unwrap();
    let user = find_user_by_gh_login(&mut conn, &author).await.unwrap();
    let name = name.to_lowercase();
    let plugin = find_plugin(&mut conn, &user, &name).await.unwrap();

    let version = if version == "latest" {
        let versions: Vec<Version> = Version::belonging_to(&plugin)
            .filter(versions::yanked.eq(false))
            .load(&mut conn)
            .await
            .unwrap();

        let max = versions
            .into_iter()
            .filter_map(|v| {
                semver::Version::parse(&v.num)
                    .ok()
                    .map(|version| (v, version))
            })
            .max_by_key(|(_, version)| version.clone());
        max.unwrap().0
    } else {
        find_plugin_version(&mut conn, &plugin, &version)
            .await
            .unwrap()
    };

    Json(EncodePlugin {
        id: plugin.id,
        name,
        author,
        version: version.num,
        display_name: plugin.display_name,
        description: plugin.description,
        downloads: plugin.downloads,
        repository: plugin.repository,
        updated_at_ts: plugin.updated_at.timestamp(),
        updated_at: plugin.updated_at.format("%Y-%m-%d %H:%M:%S").to_string(),
        released_at: version.created_at.format("%Y-%m-%d %H:%M:%S").to_string(),
        wasm: plugin.wasm,
    })
}

pub async fn download(
    State(bucket): State<Bucket>,
    State(db_pool): State<DbPool>,
    Path((author, name, version)): Path<(String, String, String)>,
) -> impl IntoResponse {
    let mut conn = db_pool.read.get().await.unwrap();
    let user = find_user_by_gh_login(&mut conn, &author).await.unwrap();
    let name = name.to_lowercase();
    let plugin = find_plugin(&mut conn, &user, &name).await.unwrap();
    let version = find_plugin_version(&mut conn, &plugin, &version)
        .await
        .unwrap();
    {
        let mut conn = db_pool.write.get().await.unwrap();
        diesel::update(plugins::dsl::plugins.find(plugin.id))
            .set(plugins::downloads.eq(plugins::downloads + 1))
            .execute(&mut conn)
            .await
            .unwrap();
        diesel::update(versions::dsl::versions.find(version.id))
            .set(versions::downloads.eq(versions::downloads + 1))
            .execute(&mut conn)
            .await
            .unwrap();
    }

    let s3_path = format!("{}/{}/{}/volt.tar.gz", user.gh_login, name, version.num);
    bucket.presign_get(&s3_path, 60, None).unwrap()
}

pub async fn readme(
    State(bucket): State<Bucket>,
    State(db_pool): State<DbPool>,
    Path((author, name, version)): Path<(String, String, String)>,
) -> impl IntoResponse {
    let mut conn = db_pool.read.get().await.unwrap();
    let user = find_user_by_gh_login(&mut conn, &author).await.unwrap();
    let name = name.to_lowercase();
    let plugin = find_plugin(&mut conn, &user, &name).await.unwrap();
    let version = find_plugin_version(&mut conn, &plugin, &version)
        .await
        .unwrap();
    let s3_path = format!("{}/{}/{}/readme", user.gh_login, name, version.num);
    let result = bucket.get_object(&s3_path).await;
    let resp = match result {
        Ok(resp) => resp,
        Err(_) => {
            return (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "can't download readme",
            )
                .into_response();
        }
    };
    if resp.status_code() != 200 {
        return (
            axum::http::StatusCode::from_u16(resp.status_code()).unwrap(),
            "can't download readme",
        )
            .into_response();
    }

    resp.bytes().to_vec().into_response()
}

pub async fn icon(
    State(bucket): State<Bucket>,
    State(db_pool): State<DbPool>,
    Path((author, name, version)): Path<(String, String, String)>,
) -> axum::response::Response {
    let mut conn = db_pool.read.get().await.unwrap();
    let user = find_user_by_gh_login(&mut conn, &author).await.unwrap();
    let name = name.to_lowercase();
    let plugin = find_plugin(&mut conn, &user, &name).await.unwrap();
    let version = find_plugin_version(&mut conn, &plugin, &version)
        .await
        .unwrap();
    let s3_path = format!("{}/{}/{}/icon", user.gh_login, name, version.num);
    let content_type = match bucket.head_object(&s3_path).await {
        Ok((head, _)) => head.content_type,
        Err(_) => {
            return (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "can't download icon",
            )
                .into_response();
        }
    };

    let result = bucket.get_object(&s3_path).await;
    let resp = match result {
        Ok(resp) => resp,
        Err(_) => {
            return (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "can't download icon",
            )
                .into_response();
        }
    };
    if resp.status_code() != 200 {
        return (
            axum::http::StatusCode::from_u16(resp.status_code()).unwrap(),
            "can't download icon",
        )
            .into_response();
    }

    let mut res = axum::body::Full::from(resp.bytes().to_vec()).into_response();
    res.headers_mut().insert(
        axum::http::header::CONTENT_TYPE,
        axum::http::header::HeaderValue::from_str(
            &content_type.unwrap_or_else(|| "image/*".to_string()),
        )
        .unwrap(),
    );
    res.headers_mut().insert(
        axum::http::header::CACHE_CONTROL,
        axum::http::header::HeaderValue::from_static("public, max-age=86400"),
    );
    res
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
    let volt = match toml::from_str::<VoltMetadata>(&s) {
        Ok(mut volt) => {
            volt.author = user.gh_login.clone();
            volt.name = volt.name.to_lowercase();
            volt
        }
        Err(_) => return (StatusCode::BAD_REQUEST, "volt.tmol format invalid").into_response(),
    };

    if semver::Version::parse(&volt.version).is_err() {
        return (StatusCode::BAD_REQUEST, "version isn't valid").into_response();
    }

    {
        let dest_volt_path = dest.path().join("volt.toml");
        tokio::fs::write(
            dest_volt_path,
            toml_edit::ser::to_string_pretty(&volt).unwrap(),
        )
        .await
        .unwrap();
    }

    let s3_folder = format!("{}/{}/{}", user.gh_login, volt.name, volt.version);

    let mut is_wasm = false;
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
        is_wasm = true;
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

                let icon_content = tokio::fs::read(&icon_path).await.unwrap();
                bucket
                    .put_object(
                        &format!("{}/{}/{}/icon", user.gh_login, volt.name, volt.version),
                        &icon_content,
                    )
                    .await
                    .unwrap();

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
            let content_type = match icon_path.extension().and_then(|s| s.to_str()) {
                Some("png") => "image/png",
                Some("jpg") | Some("jpeg") => "image/jpeg",
                Some("svg") => "image/svg+xml",
                _ => "image/*",
            };
            let icon_content = tokio::fs::read(&icon_path).await.unwrap();
            bucket
                .put_object_with_content_type(
                    &format!("{}/{}/{}/icon", user.gh_login, volt.name, volt.version),
                    &icon_content,
                    content_type,
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

    let mut conn = db_pool.write.get().await.unwrap();

    let result: Result<()> = conn
        .build_transaction()
        .run(|conn| {
            async move {
                let new_plugin = NewPlugin::new(
                    &volt.name,
                    user.id,
                    &volt.display_name,
                    &volt.description,
                    volt.repository.as_deref(),
                    is_wasm,
                );
                let plugin = new_plugin.create_or_update(conn).await?;
                let new_version = NewVersion::new(plugin.id, &volt.version);
                new_version.create_or_update(conn).await?;
                Ok(())
            }
            .boxed()
        })
        .await;
    result.unwrap();

    ().into_response()
}

async fn stream_to_file<S, E>(path: &std::path::Path, stream: S) -> Result<()>
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

pub async fn yank(
    TypedHeader(token): TypedHeader<headers::Authorization<Bearer>>,
    State(db_pool): State<DbPool>,
    Path((name, version)): Path<(String, String)>,
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

    if let Err(e) = modify_yank(&user, State(db_pool), Path((name, version)), true).await {
        return (axum::http::StatusCode::BAD_REQUEST, e.to_string()).into_response();
    }

    ().into_response()
}

pub async fn unyank(
    TypedHeader(token): TypedHeader<headers::Authorization<Bearer>>,
    State(db_pool): State<DbPool>,
    Path((name, version)): Path<(String, String)>,
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

    if let Err(e) = modify_yank(&user, State(db_pool), Path((name, version)), false).await {
        return (axum::http::StatusCode::BAD_REQUEST, e.to_string()).into_response();
    }

    ().into_response()
}

async fn modify_yank(
    user: &User,
    State(db_pool): State<DbPool>,
    Path((name, version)): Path<(String, String)>,
    yanked: bool,
) -> Result<()> {
    let plugin = {
        let mut conn = db_pool.read.get().await?;
        find_plugin(&mut conn, user, &name).await?
    };

    {
        let mut conn = db_pool.write.get().await?;
        modify_plugin_version_yank(&mut conn, &plugin, &version, yanked).await?;
    }

    Ok(())
}
