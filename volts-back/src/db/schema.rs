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
    users (id) {
        id -> Int4,
        gh_access_token -> Varchar,
        gh_login -> Varchar,
        gh_id -> Int4,
    }
}

diesel::joinable!(api_tokens -> users (user_id));

diesel::allow_tables_to_appear_in_same_query!(
    api_tokens,
    users,
);
