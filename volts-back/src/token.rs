use async_session::{MemoryStore, SessionStore};
use axum::{
    extract::{Path, State},
    response::IntoResponse,
    Json, TypedHeader,
};
use volts_core::{ApiTokenList, NewTokenPayload};

use crate::{
    db::{find_user, insert_token, list_tokens, revoke_token, DbPool},
    router::authenticated_user,
    state::SESSION_COOKIE_NAME,
};

pub async fn list(
    State(store): State<MemoryStore>,
    State(db_pool): State<DbPool>,
    TypedHeader(cookies): TypedHeader<headers::Cookie>,
) -> impl IntoResponse {
    let cookie = cookies.get(SESSION_COOKIE_NAME).unwrap();
    let session = store
        .load_session(cookie.to_string())
        .await
        .unwrap()
        .unwrap();
    let user_id: i32 = session.get("user_id").unwrap();

    let mut conn = db_pool.read.get().await.unwrap();
    let user = find_user(&mut conn, user_id).await.unwrap();
    let tokens = list_tokens(&mut conn, &user).await.unwrap();
    Json(ApiTokenList { api_tokens: tokens })
}

pub async fn new(
    State(store): State<MemoryStore>,
    State(db_pool): State<DbPool>,
    TypedHeader(cookies): TypedHeader<headers::Cookie>,
    Json(payload): Json<NewTokenPayload>,
) -> impl IntoResponse {
    let user = authenticated_user(State(store), State(db_pool.clone()), TypedHeader(cookies))
        .await
        .unwrap();
    let mut conn = db_pool.write.get().await.unwrap();

    let token = insert_token(&mut conn, &user, &payload.name).await.unwrap();
    Json(token)
}

pub async fn revoke(
    State(store): State<MemoryStore>,
    State(db_pool): State<DbPool>,
    TypedHeader(cookies): TypedHeader<headers::Cookie>,
    Path(id): Path<i32>,
) -> impl IntoResponse {
    let user = authenticated_user(State(store), State(db_pool.clone()), TypedHeader(cookies))
        .await
        .unwrap();

    let mut conn = db_pool.write.get().await.unwrap();
    revoke_token(&mut conn, &user, id).await.unwrap();
}
