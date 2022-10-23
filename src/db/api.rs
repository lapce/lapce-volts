use std::borrow::Cow;

use anyhow::Result;
use diesel::ExpressionMethods;
use diesel::QueryDsl;
use diesel::BelongingToDsl;
use diesel_async::RunQueryDsl;
use diesel_async::{pooled_connection::deadpool::Pool, AsyncPgConnection};

use super::models::User;
use super::schema::users;
use super::models::ApiToken;
use super::schema::api_tokens;

pub fn new_db_pool() -> Pool<AsyncPgConnection> {
    let manager = diesel_async::pooled_connection::AsyncDieselConnectionManager::new(
        std::env::var("DATABASE_URL").unwrap(),
    );
    Pool::builder(manager).build().unwrap()
}

/// Represents a new user record insertable to the `users` table
#[derive(Insertable, Debug, Default)]
#[table_name = "users"]
pub struct NewUser<'a> {
    pub gh_id: i32,
    pub gh_login: &'a str,
    pub gh_access_token: Cow<'a, str>,
}

impl<'a> NewUser<'a> {
    pub fn new(gh_id: i32, gh_login: &'a str, gh_access_token: &'a str) -> Self {
        NewUser {
            gh_id,
            gh_login,
            gh_access_token: Cow::Borrowed(gh_access_token),
        }
    }

    /// Inserts the user into the database, or updates an existing one.
    pub async fn create_or_update(&self, conn: &mut AsyncPgConnection) -> Result<User> {
        use crate::db::schema::users::dsl::*;
        use diesel::pg::upsert::excluded;

        let user: User = diesel::insert_into(users)
            .values(self)
            .on_conflict(gh_id)
            .do_update()
            .set((
                gh_login.eq(excluded(gh_login)),
                gh_access_token.eq(excluded(gh_access_token)),
            ))
            .get_result(conn)
            .await?;
        Ok(user)
    }
}

impl User {
    pub async fn find(conn: &mut AsyncPgConnection, id: i32) -> Result<User> {
        let user = users::table.find(id).first(conn).await?;
        Ok(user)
    }
}

impl ApiToken {
    pub async fn list(conn: &mut AsyncPgConnection, user: &User) -> Result<Vec<ApiToken>> {
        let tokens: Vec<ApiToken> = ApiToken::belonging_to(&user)
            .filter(api_tokens::revoked.eq(false))
            .order(api_tokens::created_at.desc())
            .load(conn)
            .await?;
        Ok(tokens)
    }
}
