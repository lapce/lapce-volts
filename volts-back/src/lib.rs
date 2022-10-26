use std::net::SocketAddr;

pub(crate) mod db;
pub mod github;
pub(crate) mod plugin;
pub mod router;
pub mod state;
pub mod token;
pub mod util;

#[macro_use]
extern crate diesel;

pub async fn start_server() {
    dotenvy::dotenv().ok();
    let router = crate::router::build_router();
    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    axum::Server::bind(&addr)
        .serve(router.into_make_service())
        .await
        .unwrap();
}
