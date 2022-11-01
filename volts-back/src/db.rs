use std::borrow::Cow;

use anyhow::Result;
use diesel::BelongingToDsl;
use diesel::ExpressionMethods;
use diesel::NullableExpressionMethods;
use diesel::QueryDsl;
use diesel_async::RunQueryDsl;
use diesel_async::{pooled_connection::deadpool::Pool, AsyncPgConnection};
use volts_core::db::models::Plugin;
use volts_core::db::models::{ApiToken, User, Version};
use volts_core::db::schema::{api_tokens, plugins, users, versions};
use volts_core::EncodeApiToken;

#[derive(Clone)]
pub struct DbPool {
    pub write: Pool<AsyncPgConnection>,
    pub read: Pool<AsyncPgConnection>,
}

impl Default for DbPool {
    fn default() -> Self {
        Self::new()
    }
}

impl DbPool {
    pub fn new() -> Self {
        let manager = diesel_async::pooled_connection::AsyncDieselConnectionManager::new(
            std::env::var("DATABASE_URL").unwrap(),
        );
        let write = Pool::builder(manager).build().unwrap();

        let manager = diesel_async::pooled_connection::AsyncDieselConnectionManager::new(
            std::env::var("READ_DATABASE_URL")
                .unwrap_or_else(|_| std::env::var("DATABASE_URL").unwrap()),
        );
        let read = Pool::builder(manager).build().unwrap();

        Self { write, read }
    }
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

pub async fn find_user_by_gh_login(conn: &mut AsyncPgConnection, gh_login: &str) -> Result<User> {
    let user = users::table
        .filter(users::gh_login.eq(gh_login))
        .first(conn)
        .await?;
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

#[derive(Insertable, Debug, Default)]
#[diesel(table_name = plugins)]
pub struct NewPlugin<'a> {
    pub name: &'a str,
    pub user_id: i32,
    pub display_name: &'a str,
    pub description: &'a str,
    pub downloads: i32,
}

impl<'a> NewPlugin<'a> {
    pub fn new(name: &'a str, user_id: i32, display_name: &'a str, description: &'a str) -> Self {
        NewPlugin {
            name,
            user_id,
            display_name,
            description,
            downloads: 0,
        }
    }

    pub async fn create_or_update(&self, conn: &mut AsyncPgConnection) -> Result<Plugin> {
        use diesel::pg::upsert::excluded;
        use volts_core::db::schema::plugins::dsl::*;

        let plugin: Plugin = diesel::insert_into(plugins)
            .values(self)
            .on_conflict((user_id, name))
            .do_update()
            .set((
                display_name.eq(excluded(display_name)),
                description.eq(excluded(description)),
                updated_at.eq(chrono::Utc::now().naive_utc()),
            ))
            .get_result(conn)
            .await?;
        Ok(plugin)
    }
}

#[derive(Insertable, Debug, Default)]
#[diesel(table_name = versions)]
pub struct NewVersion<'a> {
    pub plugin_id: i32,
    pub num: &'a str,
    pub yanked: bool,
}

impl<'a> NewVersion<'a> {
    pub fn new(plugin_id: i32, num: &'a str) -> Self {
        NewVersion {
            plugin_id,
            num,
            yanked: false,
        }
    }

    pub async fn create_or_update(&self, conn: &mut AsyncPgConnection) -> Result<Version> {
        use volts_core::db::schema::versions::dsl::*;

        let version: Version = diesel::insert_into(versions)
            .values(self)
            .on_conflict((plugin_id, num))
            .do_update()
            .set(updated_at.eq(chrono::Utc::now().naive_utc()))
            .get_result(conn)
            .await?;
        Ok(version)
    }
}

pub async fn find_plugin(conn: &mut AsyncPgConnection, user: &User, name: &str) -> Result<Plugin> {
    let plugin = Plugin::belonging_to(user)
        .filter(plugins::name.eq(name))
        .get_result(conn)
        .await?;
    Ok(plugin)
}

pub async fn find_plugin_version(
    conn: &mut AsyncPgConnection,
    plugin: &Plugin,
    num: &str,
) -> Result<Version> {
    let version = Version::belonging_to(plugin)
        .filter(versions::num.eq(num))
        .get_result(conn)
        .await?;
    Ok(version)
}

pub async fn modify_plugin_version_yank(
    conn: &mut AsyncPgConnection,
    plugin: &Plugin,
    num: &str,
    is_yanked: bool,
) -> Result<Version> {
    let version = diesel::update(Version::belonging_to(plugin).filter(versions::num.eq(num)))
        .set((
            versions::yanked.eq(is_yanked),
            versions::updated_at.eq(chrono::Utc::now().naive_utc()),
        ))
        .get_result(conn)
        .await?;
    Ok(version)
}
