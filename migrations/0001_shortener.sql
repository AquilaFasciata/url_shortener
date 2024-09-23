CREATE TABLE "users"(
    "id" bigserial NOT NULL,
    "username" TEXT NOT NULL,
    "hashed_pw" TEXT NOT NULL,
    "email" TEXT NOT NULL
);
ALTER TABLE
    "users" ADD PRIMARY KEY("id");
CREATE TABLE "urls"(
    "id" bigserial NOT NULL,
    "shorturl" TEXT NOT NULL,
    "longurl" TEXT NOT NULL,
    "created_by" BIGINT NULL,
    "clicks" BIGINT NOT NULL
);
CREATE INDEX "urls_shorturl_index" ON
    "urls"("shorturl");
ALTER TABLE
    "urls" ADD PRIMARY KEY("id");
ALTER TABLE
    "urls" ADD CONSTRAINT "urls_shorturl_unique" UNIQUE("shorturl");
ALTER TABLE
    "urls" ADD CONSTRAINT "urls_created_by_foreign" FOREIGN KEY("created_by") REFERENCES "users"("id");
