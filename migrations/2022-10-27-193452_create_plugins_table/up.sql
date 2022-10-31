-- Your SQL goes here
create table plugins (
    id                SERIAL PRIMARY KEY,
    name              VARCHAR NOT NULL,
    user_id           INTEGER NOT NULL,
    updated_at        timestamp NOT NULL DEFAULT CURRENT_TIMESTAMP,
    created_at        timestamp NOT NULL DEFAULT CURRENT_TIMESTAMP,
    display_name      VARCHAR NOT NULL,
    description       VARCHAR NOT NULL,
    downloads         INTEGER NOT NULL DEFAULT 0,
    repository        VARCHAR,
    CONSTRAINT "plugins_user_id_fkey" FOREIGN KEY ("user_id") REFERENCES "public"."users"("id")
);

CREATE UNIQUE INDEX plugins_user_id_name ON plugins (user_id, name);
