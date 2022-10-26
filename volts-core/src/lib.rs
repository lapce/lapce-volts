pub mod db;
pub mod util;

#[macro_use]
extern crate diesel;

use db::models::ApiToken;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct MeUser {
    pub login: String,
}

#[derive(Serialize, Deserialize)]
pub struct NewSessionResponse {
    pub url: String,
    pub state: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ApiTokenList {
    pub api_tokens: Vec<ApiToken>,
}

#[derive(Serialize, Deserialize)]
pub struct EncodeApiToken {
    pub token: ApiToken,
    pub plaintext: String,
}

#[derive(Serialize, Deserialize)]
pub struct NewTokenPayload {
    pub name: String,
}
