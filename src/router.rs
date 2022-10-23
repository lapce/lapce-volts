use async_session::{MemoryStore, Session, SessionStore};
use axum::{
    extract::{Query, State},
    http::{header::SET_COOKIE, HeaderMap, StatusCode},
    response::{IntoResponse, Redirect},
    routing::get,
    Json, Router, TypedHeader,
};
use diesel_async::{pooled_connection::deadpool::Pool, AsyncPgConnection};
use oauth2::{
    basic::BasicClient, reqwest::async_http_client, AuthorizationCode, Scope, TokenResponse,
};
use serde::Deserialize;
use serde_json::json;

use crate::{
    db::api::NewUser,
    github::GithubClient,
    state::{AppState, SESSION_COOKIE_NAME},
    token,
};

pub fn build_router() -> Router<AppState> {
    let state = AppState::new();
    Router::with_state(state)
        .route("/api/private/session", get(new_session))
        .route("/api/private/session/authorize", get(session_authorize))
        .route("/api/v1/me/tokens", get(token::list))
}

async fn new_session(
    State(store): State<MemoryStore>,
    State(github_oauth): State<BasicClient>,
) -> impl IntoResponse {
    let (url, state) = github_oauth
        .authorize_url(oauth2::CsrfToken::new_random)
        .add_scope(Scope::new("read:org".to_string()))
        .url();
    let state = state.secret().to_string();

    let mut session = Session::new();
    let _ = session.insert("github_oauth_state", state.clone());
    let cookie = store.store_session(session).await.unwrap().unwrap();
    let cookie = format!("{SESSION_COOKIE_NAME}={cookie}; Path=/");

    let mut headers = HeaderMap::new();
    headers.insert(SET_COOKIE, cookie.parse().unwrap());

    (
        headers,
        Json(json!({
            "url": url,
            "state": state,
        })),
    )
}

#[derive(Debug, Deserialize)]
struct AuthRequest {
    code: String,
    state: String,
}

async fn session_authorize(
    Query(query): Query<AuthRequest>,
    State(store): State<MemoryStore>,
    State(github_oauth): State<BasicClient>,
    State(github_client): State<GithubClient>,
    State(db_pool): State<Pool<AsyncPgConnection>>,
    TypedHeader(cookies): TypedHeader<headers::Cookie>,
) -> impl IntoResponse {
    let cookie = cookies.get(SESSION_COOKIE_NAME).unwrap();
    let mut session = store
        .load_session(cookie.to_string())
        .await
        .unwrap()
        .unwrap();
    let session_state = session.get("github_oauth_state");
    println!("session state is {session_state:?}");
    session.remove("github_oauth_state");
    if session_state != Some(query.state) {
        return (StatusCode::BAD_REQUEST, "invalid state parameter").into_response();
    }

    // Fetch the access token from GitHub using the code we just got
    let code = AuthorizationCode::new(query.code);
    let token = github_oauth
        .exchange_code(code)
        .request_async(async_http_client)
        .await
        .unwrap();
    let token = token.access_token();

    let ghuser = github_client.current_user(token).await.unwrap();

    let mut conn = db_pool.get().await.unwrap();

    let user = NewUser::new(ghuser.id, &ghuser.login, token.secret())
        .create_or_update(&mut conn)
        .await
        .unwrap();

    session.insert("user_id", user.id).unwrap();

    Redirect::to("/").into_response()
}
