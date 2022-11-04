use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

use crate::db::schema::{api_tokens, plugins, users, versions};
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

#[derive(Queryable, Debug, Identifiable, Associations)]
#[diesel(belongs_to(User))]
pub struct Plugin {
    pub id: i32,
    pub name: String,
    pub user_id: i32,
    pub updated_at: NaiveDateTime,
    pub created_at: NaiveDateTime,
    pub display_name: String,
    pub description: String,
    pub downloads: i32,
    pub repository: Option<String>,
    pub wasm: bool,
}

#[derive(Queryable, Debug, Identifiable, Associations)]
#[diesel(belongs_to(Plugin))]
pub struct Version {
    pub id: i32,
    pub plugin_id: i32,
    pub updated_at: NaiveDateTime,
    pub created_at: NaiveDateTime,
    pub num: String,
    pub yanked: bool,
    pub downloads: i32,
}
