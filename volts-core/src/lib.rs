pub mod db;
pub mod util;

#[macro_use]
extern crate diesel;

use chrono::NaiveDateTime;
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

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone)]
pub struct EncodePlugin {
    pub name: String,
    pub author: String,
    pub version: String,
    pub display_name: String,
    pub description: String,
    pub downloads: i32,
    pub repository: Option<String>,
    pub updated_at: NaiveDateTime,
}

#[derive(Serialize, Deserialize)]
pub struct PluginList {
    pub total: i64,
    pub limit: usize,
    pub page: usize,
    pub plugins: Vec<EncodePlugin>,
}
