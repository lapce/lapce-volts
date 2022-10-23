use std::net::SocketAddr;

pub mod db;
pub mod github;
pub mod router;
pub mod state;
pub mod token;
pub mod util;

#[macro_use]
extern crate diesel;

pub async fn start_server() {
    dotenvy::dotenv().ok();
    let router = crate::router::build_router();
    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    axum::Server::bind(&addr)
        .serve(router.into_make_service())
        .await
        .unwrap();
}
