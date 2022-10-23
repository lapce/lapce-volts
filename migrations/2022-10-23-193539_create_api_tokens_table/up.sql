-- Your SQL goes here
CREATE TABLE "public"."api_tokens" (
    "id" SERIAL PRIMARY KEY,
    "user_id" INTEGER NOT NULL,
    "token" bytea NOT NULL,
    "name" varchar NOT NULL,
    "created_at" timestamp NOT NULL DEFAULT now(),
    "last_used_at" timestamp,
    "revoked" bool NOT NULL DEFAULT false,
    CONSTRAINT "api_tokens_user_id_fkey" FOREIGN KEY ("user_id") REFERENCES "public"."users"("id")
);