use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

use crate::db::schema::{api_tokens, users};
use crate::util::rfc3339;

#[derive(
    Queryable, Debug, Identifiable, Associations, Serialize, Deserialize, Clone, PartialEq, Eq,
)]
#[diesel(belongs_to(User))]
pub struct ApiToken {
    pub id: i32,
    #[serde(skip)]
    pub user_id: i32,
    #[serde(skip)]
    pub token: Vec<u8>,
    pub name: String,
    #[serde(with = "rfc3339")]
    pub created_at: NaiveDateTime,
    #[serde(with = "rfc3339::option")]
    pub last_used_at: Option<NaiveDateTime>,
    #[serde(skip)]
    pub revoked: bool,
}

#[derive(Queryable, Debug, Identifiable)]
pub struct User {
    pub id: i32,
    pub gh_access_token: String,
    pub gh_login: String,
    pub gh_id: i32,
}
