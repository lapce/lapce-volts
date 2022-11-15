use std::env;

use async_session::MemoryStore;
use axum::extract::FromRef;
use oauth2::{basic::BasicClient, AuthUrl, ClientId, ClientSecret, TokenUrl};
use s3::{creds::Credentials, Bucket, Region};

use crate::{db::DbPool, github::GithubClient};

const GITHUB_OAUTH_AUTHORIZE_ENDPOINT: &str = "https://github.com/login/oauth/authorize";
const GITHUB_OAUTH_TOKEN_ENDPOINT: &str = "https://github.com/login/oauth/access_token";

pub const SESSION_COOKIE_NAME: &str = "session";

#[derive(Clone)]
pub struct AppState {
    store: MemoryStore,
    /// The GitHub OAuth2 configuration
    pub github_oauth: BasicClient,
    github_client: GithubClient,
    db_pool: DbPool,
    bucket: Bucket,
}

impl FromRef<AppState> for MemoryStore {
    fn from_ref(state: &AppState) -> Self {
        state.store.clone()
    }
}

impl FromRef<AppState> for BasicClient {
    fn from_ref(state: &AppState) -> Self {
        state.github_oauth.clone()
    }
}

impl FromRef<AppState> for GithubClient {
    fn from_ref(state: &AppState) -> Self {
        state.github_client.clone()
    }
}

impl FromRef<AppState> for DbPool {
    fn from_ref(state: &AppState) -> Self {
        state.db_pool.clone()
    }
}

impl FromRef<AppState> for Bucket {
    fn from_ref(state: &AppState) -> Self {
        state.bucket.clone()
    }
}

impl Default for AppState {
    fn default() -> Self {
        AppState::new()
    }
}

impl AppState {
    pub fn new() -> Self {
        let github_client_id = ClientId::new(
            env::var("GITHUB_CLIENT_ID")
                .expect("Missing the GITHUB_CLIENT_ID environment variable."),
        );
        let github_client_secret = ClientSecret::new(
            env::var("GITHUB_CLIENT_SECRET")
                .expect("Missing the GITHUB_CLIENT_SECRET environment variable."),
        );
        let auth_url = AuthUrl::new(GITHUB_OAUTH_AUTHORIZE_ENDPOINT.to_string())
            .expect("Invalid authorization endpoint URL");
        let token_url = TokenUrl::new(GITHUB_OAUTH_TOKEN_ENDPOINT.to_string())
            .expect("Invalid token endpoint URL");

        // Set up the config for the Github OAuth2 process.
        let github_oauth = BasicClient::new(
            github_client_id,
            Some(github_client_secret),
            auth_url,
            Some(token_url),
        );
        let store = MemoryStore::new();
        let github_client = GithubClient::new();
        let db_pool = crate::db::DbPool::new();
        let bucket = Bucket::new(
            "lapce-plugins",
            Region::R2 {
                account_id: env::var("R2_ACCOUNT_ID").unwrap(),
            },
            Credentials::from_env().unwrap(),
        )
        .unwrap()
        .with_path_style();
        Self {
            store,
            github_oauth,
            github_client,
            db_pool,
            bucket,
        }
    }
}
