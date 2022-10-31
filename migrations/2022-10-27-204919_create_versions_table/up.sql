-- Your SQL goes here
create table versions (
    id                SERIAL PRIMARY KEY,
    plugin_id         INTEGER NOT NULL,
    updated_at        timestamp NOT NULL DEFAULT CURRENT_TIMESTAMP,
    created_at        timestamp NOT NULL DEFAULT CURRENT_TIMESTAMP,
    num               VARCHAR NOT NULL,
    yanked            bool NOT NULL DEFAULT false,
    CONSTRAINT "versions_plugin_id_fkey" FOREIGN KEY ("plugin_id") REFERENCES "public"."plugins"("id")
);

CREATE UNIQUE INDEX versions_plugin_id_num ON versions (plugin_id, num);
