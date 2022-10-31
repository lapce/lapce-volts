-- Your SQL goes here
create table users (
    id                SERIAL PRIMARY KEY,
    gh_access_token   VARCHAR NOT NULL,
    gh_login          VARCHAR NOT NULL,
    gh_id             INTEGER NOT NULL
);

CREATE UNIQUE INDEX users_gh_id ON users (gh_id);
