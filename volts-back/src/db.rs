use std::borrow::Cow;

use anyhow::Result;
use diesel::BelongingToDsl;
use diesel::ExpressionMethods;
use diesel::NullableExpressionMethods;
use diesel::QueryDsl;
use diesel_async::RunQueryDsl;
use diesel_async::{pooled_connection::deadpool::Pool, AsyncPgConnection};
use volts_core::db::models::{ApiToken, User};
use volts_core::db::schema::{api_tokens, users};
use volts_core::EncodeApiToken;

pub fn new_db_pool() -> Pool<AsyncPgConnection> {
    let manager = diesel_async::pooled_connection::AsyncDieselConnectionManager::new(
        std::env::var("DATABASE_URL").unwrap(),
    );
    Pool::builder(manager).build().unwrap()
}

/// Represents a new user record insertable to the `users` table
#[derive(Insertable, Debug, Default)]
#[diesel(table_name = users)]
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
        use diesel::pg::upsert::excluded;
        use volts_core::db::schema::users::dsl::*;

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

pub async fn find_user(conn: &mut AsyncPgConnection, id: i32) -> Result<User> {
    let user = users::table.find(id).first(conn).await?;
    Ok(user)
}

pub async fn list_tokens(conn: &mut AsyncPgConnection, user: &User) -> Result<Vec<ApiToken>> {
    let tokens: Vec<ApiToken> = ApiToken::belonging_to(&user)
        .filter(api_tokens::revoked.eq(false))
        .order(api_tokens::created_at.desc())
        .load(conn)
        .await?;
    Ok(tokens)
}

pub async fn insert_token(
    conn: &mut AsyncPgConnection,
    user: &User,
    name: &str,
) -> Result<EncodeApiToken> {
    let token = crate::util::SecureToken::new_token();

    let model: ApiToken = diesel::insert_into(api_tokens::table)
        .values((
            api_tokens::user_id.eq(user.id),
            api_tokens::name.eq(name),
            api_tokens::token.eq(token.token()),
        ))
        .get_result(conn)
        .await?;

    Ok(EncodeApiToken {
        token: model,
        plaintext: token.plaintext().into(),
    })
}

pub async fn revoke_token(conn: &mut AsyncPgConnection, user: &User, id: i32) -> Result<()> {
    diesel::update(ApiToken::belonging_to(&user).find(id))
        .set(api_tokens::revoked.eq(true))
        .execute(conn)
        .await?;
    Ok(())
}

pub async fn find_api_token(conn: &mut AsyncPgConnection, api_token: &str) -> Result<ApiToken> {
    use diesel::{dsl::now, update};
    use volts_core::db::schema::api_tokens::dsl::*;

    let token_ = crate::util::SecureToken::parse(api_token);

    let tokens = api_tokens
        .filter(revoked.eq(false))
        .filter(token.eq(token_.token()));

    let token_ = update(tokens)
        .set(last_used_at.eq(now.nullable()))
        .get_result(conn)
        .await?;

    Ok(token_)
}
