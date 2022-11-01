// @generated automatically by Diesel CLI.

diesel::table! {
    api_tokens (id) {
        id -> Int4,
        user_id -> Int4,
        token -> Bytea,
        name -> Varchar,
        created_at -> Timestamp,
        last_used_at -> Nullable<Timestamp>,
        revoked -> Bool,
    }
}

diesel::table! {
    plugins (id) {
        id -> Int4,
        name -> Varchar,
        user_id -> Int4,
        updated_at -> Timestamp,
        created_at -> Timestamp,
        display_name -> Varchar,
        description -> Varchar,
        downloads -> Int4,
        repository -> Nullable<Varchar>,
    }
}

diesel::table! {
    users (id) {
        id -> Int4,
        gh_access_token -> Varchar,
        gh_login -> Varchar,
        gh_id -> Int4,
    }
}

diesel::table! {
    versions (id) {
        id -> Int4,
        plugin_id -> Int4,
        updated_at -> Timestamp,
        created_at -> Timestamp,
        num -> Varchar,
        downloads -> Int4,
        yanked -> Bool,
    }
}

diesel::joinable!(api_tokens -> users (user_id));
diesel::joinable!(plugins -> users (user_id));
diesel::joinable!(versions -> plugins (plugin_id));

diesel::allow_tables_to_appear_in_same_query!(
    api_tokens,
    plugins,
    users,
    versions,
);
