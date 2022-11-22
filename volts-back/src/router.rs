use async_session::{MemoryStore, Session, SessionStore};
use axum::{
    extract::{Query, State},
    http::{header::SET_COOKIE, HeaderMap, StatusCode},
    response::{IntoResponse, Redirect},
    routing::{delete, get, post, put},
    Json, Router, TypedHeader,
};
use oauth2::{
    basic::BasicClient, reqwest::async_http_client, AuthorizationCode, Scope, TokenResponse,
};
use serde::Deserialize;
use volts_core::{db::models::User, MeUser, NewSessionResponse};

use crate::{
    db::{find_user, DbPool, NewUser},
    github::GithubClient,
    plugin,
    state::{AppState, SESSION_COOKIE_NAME},
    token,
};

pub fn build_router() -> Router<AppState> {
    let state = AppState::new();

    let private_routes = Router::with_state(state.clone())
        .route("/session", get(new_session))
        .route("/session", delete(logout))
        .route("/session/authorize", get(session_authorize));

    let user_routes = Router::with_state(state.clone())
        .route("/", get(me))
        .route("/tokens", get(token::list))
        .route("/tokens", post(token::new))
        .route("/tokens/:id", delete(token::revoke));

    let plugins_routes = Router::with_state(state.clone())
        .route("/", get(plugin::search))
        .route("/new", put(plugin::publish))
        .route("/:author/:name/:version", get(plugin::meta))
        .route("/:author/:name/:version/yank", post(plugin::yank))
        .route("/:author/:name/:version/unyank", post(plugin::unyank))
        .route("/:author/:name/:version/download", get(plugin::download))
        .route("/:author/:name/:version/readme", get(plugin::readme))
        .route("/:author/:name/:version/icon", get(plugin::icon));

    let v1 = Router::with_state(state.clone())
        .nest("/me", user_routes)
        .nest("/plugins", plugins_routes);

    let api = Router::with_state(state.clone())
        .nest("/private", private_routes)
        .nest("/v1", v1);

    Router::with_state(state).nest("/api", api)
}

async fn me(
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
    Json(MeUser {
        login: user.gh_login,
    })
}

async fn new_session(
    State(store): State<MemoryStore>,
    State(github_oauth): State<BasicClient>,
) -> impl IntoResponse {
    let (url, state) = github_oauth
        .authorize_url(oauth2::CsrfToken::new_random)
        .add_scope(Scope::new("read:user".to_string()))
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
        Json(NewSessionResponse {
            url: url.as_str().to_string(),
            state,
        }),
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
    State(db_pool): State<DbPool>,
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

    let mut conn = db_pool.write.get().await.unwrap();

    let user = NewUser::new(ghuser.id, &ghuser.login, token.secret())
        .create_or_update(&mut conn)
        .await
        .unwrap();

    session.insert("user_id", user.id).unwrap();

    println!("redirect to home page");
    Redirect::temporary("/account/").into_response()
}

async fn logout(
    State(store): State<MemoryStore>,
    TypedHeader(cookies): TypedHeader<headers::Cookie>,
) -> impl IntoResponse {
    let cookie = cookies.get(SESSION_COOKIE_NAME).unwrap();
    let mut session = store
        .load_session(cookie.to_string())
        .await
        .unwrap()
        .unwrap();
    session.remove("user_id");
}

pub async fn authenticated_user(
    State(store): State<MemoryStore>,
    State(db_pool): State<DbPool>,
    TypedHeader(cookies): TypedHeader<headers::Cookie>,
) -> Option<User> {
    let cookie = cookies.get(SESSION_COOKIE_NAME)?;
    let session = store.load_session(cookie.to_string()).await.ok()??;
    let user_id: i32 = session.get("user_id")?;
    let mut conn = db_pool.read.get().await.ok()?;
    find_user(&mut conn, user_id).await.ok()
}
