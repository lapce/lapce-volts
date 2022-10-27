use std::path::Path;

use anyhow::Result;
use axum::{
    body::Bytes,
    extract::{BodyStream, State},
    http::StatusCode,
    response::IntoResponse,
    BoxError, TypedHeader,
};
use diesel_async::{pooled_connection::deadpool::Pool, AsyncPgConnection};
use flate2::read::GzDecoder;
use futures::{Stream, TryStreamExt};
use headers::authorization::Bearer;
use lapce_rpc::plugin::VoltMetadata;
use tar::Archive;
use tokio_util::io::StreamReader;
use toml_edit::easy as toml;

use crate::db::{find_api_token, find_user, DbPool};

pub async fn publish(
    State(db_pool): State<DbPool>,
    TypedHeader(token): TypedHeader<headers::Authorization<Bearer>>,
    body: BodyStream,
) -> impl IntoResponse {
    let api_token = {
        let mut conn = db_pool.read.get().await.unwrap();
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
    let tar_gz = dir.path().join("plugin.tar.gz");
    stream_to_file(&tar_gz, body).await.unwrap();

    let dir_path = dir.path().to_path_buf();
    tokio::task::spawn_blocking(move || {
        let tar_gz = std::fs::File::open(tar_gz).unwrap();
        let tar = GzDecoder::new(tar_gz);
        let mut archive = Archive::new(tar);
        archive.unpack(dir_path).unwrap();
    })
    .await
    .unwrap();

    let dir_path = dir.path().to_path_buf();
    let entries = tokio::task::spawn_blocking(move || {
        std::fs::read_dir(dir_path)
            .unwrap()
            .map(|res| res.map(|e| e.path()))
            .collect::<Result<Vec<_>, std::io::Error>>()
            .unwrap()
    })
    .await
    .unwrap();

    let volt_path = dir.path().join("volt.toml");
    if !volt_path.exists() {
        return (StatusCode::BAD_REQUEST, "volt.toml doens't exist").into_response();
    }

    let s = tokio::fs::read_to_string(volt_path).await.unwrap();
    let volt: VoltMetadata = match toml::from_str(&s) {
        Ok(volt) => volt,
        Err(_) => return (StatusCode::BAD_REQUEST, "volt.tmol format invalid").into_response(),
    };

    println!("entries {entries:?}");

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
