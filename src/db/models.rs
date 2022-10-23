use chrono::NaiveDateTime;
use serde::Serialize;

use crate::db::schema::{api_tokens, users};

#[derive(Queryable, Debug, Identifiable, Associations, Serialize)]
#[belongs_to(User)]
pub struct ApiToken {
    pub id: i32,
    pub user_id: i32,
    pub token: Vec<u8>,
    pub name: String,
    pub created_at: NaiveDateTime,
    pub last_used_at: Option<NaiveDateTime>,
    pub revoked: bool,
}

#[derive(Queryable, Debug, Identifiable)]
pub struct User {
    pub id: i32,
    pub gh_access_token: String,
    pub gh_login: String,
    pub gh_id: i32,
}
