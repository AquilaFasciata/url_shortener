use askama::Template;
use base64::{engine::general_purpose, prelude::*};
use rand::{
    distributions::{Alphanumeric, DistString},
    prelude::*,
};
use sqlx::{postgres::PgQueryResult, FromRow};
use std::{result::Result, str};

#[derive(FromRow, Debug, Clone, Template)]
#[template(path = "url-table-row.html")]
#[allow(dead_code)]
pub struct UrlRow {
    // If fields are updated, update UrlRowIterator
    id: i64,
    shorturl: String,
    longurl: String,
    created_by: Option<i64>,
    clicks: i64,
}

#[derive(FromRow, Debug)]
#[allow(dead_code)]
pub struct UserRow {
    id: i64,
    username: String,
    hashed_pw: String,
    email: String,
}

#[allow(dead_code)]
impl UserRow {
    pub fn hashed_pw(&self) -> &String {
        &self.hashed_pw
    }
    pub fn hashed_pw_mut(&mut self) -> &mut String {
        &mut self.hashed_pw
    }
    pub fn username(&self) -> &String {
        &self.username
    }
    pub fn username_mut(&mut self) -> &mut String {
        &mut self.username
    }
    pub fn email(&self) -> &String {
        &self.email
    }
    pub fn email_mut(&mut self) -> &mut String {
        &mut self.email
    }
    pub fn id(&self) -> &i64 {
        &self.id
    }
    pub fn id_mut(&mut self) -> &mut i64 {
        &mut self.id
    }
    pub fn update_id(&mut self, new_id: i64) {
        self.id = new_id
    }
    pub fn new(id: i64, username: String, hashed_pw: String, email: String) -> UserRow {
        UserRow {
            id,
            username,
            hashed_pw,
            email,
        }
    }
}

impl UrlRow {
    pub fn id(&self) -> i64 {
        self.id
    }
    pub fn long_url(&self) -> &String {
        &self.longurl
    }
    pub fn short_url(&self) -> &String {
        &self.shorturl
    }
    pub fn clone_short_url(&self) -> String {
        self.shorturl.clone()
    }
    pub fn incr_click(&mut self) -> &Self {
        self.clicks += 1;
        self
    }
}

/// Creates a UrlRow, inserts it into the PostgreSQL databse, and returns the created UrlRow object
pub async fn create_url(
    long_url: &str,
    user_id: Option<i64>,
    connection_pool: &sqlx::PgPool,
    url_len: usize,
) -> Result<UrlRow, sqlx::Error> {
    let temp_long = gen_url_longword(long_url);
    let mut short_url = String::new();

    // Cycle through intil there is a window that is unused
    for keyword in temp_long.windows(url_len) {
        let keyword_str =
            str::from_utf8(keyword).expect("Error parsing str. This shouldn't be possible!");
        match retrieve_url(keyword_str, connection_pool).await {
            Ok(response) => {
                if response.is_empty() {
                    short_url = String::from_utf8(Vec::from(keyword))
                        .expect("Error iterpreting short url set")
                }
            }
            Err(_) => break,
        }
        if !short_url.is_empty() {
            break;
        }
    }

    // Checking if there is a successful URL generated from uuid and generating random if there are
    // collisions
    if short_url.is_empty() {
        loop {
            short_url = Alphanumeric.sample_string(&mut thread_rng(), url_len);
            let req_result = retrieve_url(&short_url, connection_pool).await;
            // If there is a response that is empty (no long url) or error (there is no applicable
            // row) then break from the loop (new url found isn't being used)
            if req_result.as_ref().is_ok_and(|res_str| res_str.is_empty()) || req_result.is_err() {
                break;
            }
        }
    }

    let mut new_row = UrlRow {
        id: -1,
        shorturl: short_url.clone(),
        longurl: long_url.to_string(),
        created_by: user_id,
        clicks: 0,
    };

    new_row.id = url_db_create(&new_row, connection_pool).await?;

    Ok(new_row)
}

/// Retrieves a Long Url from the database from a Short Url. This is a more efficient function than
/// retriving the object because the filtering is done on the PostgreSQL server.
pub async fn retrieve_url(
    url: &str,
    pool: &sqlx::PgPool,
) -> Result<std::string::String, sqlx::Error> {
    let response = sqlx::query_scalar("SELECT longurl FROM urls WHERE shorturl = $1")
        .bind(url)
        .fetch_one(pool)
        .await?;
    Ok(response)
}

pub async fn incr_url_clicks(row: &mut UrlRow, pool: &sqlx::PgPool) {
    row.incr_click();
    sqlx::query(
        "UPDATE urls
        SET clicks = clicks + 1
        WHERE id = $1",
    )
    .bind(row.id())
    .execute(pool)
    .await
    .unwrap();
}

/// Deletes a url entry in the databse by id. Returns a sqlx::PgQueryResult on success and
/// sqlx::Error on failure
pub async fn delete_url(id: i64, pool: &sqlx::PgPool) -> Result<PgQueryResult, sqlx::Error> {
    sqlx::query("DELETE FROM urls WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await
}

/// Retrieve a UrlRow object WHERE shorturl = $url
/// This will return a UrlRow, or a sqlx::Error upon failure
pub async fn retrieve_url_obj(url: &str, pool: &sqlx::PgPool) -> Result<UrlRow, sqlx::Error> {
    let response: UrlRow = sqlx::query_as("SELECT * FROM urls WHERE shorturl = $1")
        .bind(url)
        .fetch_one(pool)
        .await?;
    Ok(response)
}

/// Creates a long string from which we can use to create a short url
fn gen_url_longword(long_url: &str) -> Vec<u8> {
    let long_word = general_purpose::STANDARD_NO_PAD.encode(long_url.as_bytes());
    return Vec::from(long_word.as_bytes());
}

/// Creates the UrlRow object in the PostgreSQL database and returns the id of the newly created
/// row
async fn url_db_create(new_row: &UrlRow, pool: &sqlx::PgPool) -> Result<i64, sqlx::Error> {
    sqlx::query("INSERT INTO urls (shorturl, longurl, created_by, clicks) VALUES ($1, $2, $3, 0)")
        .bind(new_row.shorturl.clone())
        .bind(new_row.longurl.clone())
        .bind(new_row.created_by)
        .execute(pool)
        .await?;

    let new_id = retrieve_url_obj(new_row.shorturl.as_str(), pool).await?.id;
    Ok(new_id)
}

#[cfg(test)]
mod tests {
    use std::env;

    use sqlx::{postgres::PgPoolOptions, PgPool};

    use crate::preferences::Preferences;

    use super::*;

    async fn pool_init() -> (PgPool, Preferences) {
        eprintln!("Current dir: {:#?}", env::current_dir().unwrap());
        let prefs =
            Preferences::load_config("./config.toml").expect("Error loading config from TOML");
        let conn_url = format!(
            "postgres://{}:{}@172.17.0.2/{}",
            prefs.db_user(),
            prefs.db_pass(),
            prefs.db_name(),
        );
        let pool = PgPoolOptions::new()
            .max_connections(prefs.db_pool_size())
            .connect(&conn_url)
            .await
            .expect("Couldn't create connection pool. Are your credentials correct?");

        sqlx::migrate!("./migrations").run(&pool).await.unwrap();

        return (pool, prefs);
    }

    async fn test_make_url() -> UrlRow {
        let (pool, prefs) = pool_init().await;
        let short_row: UrlRow = create_url("https://example.com", None, &pool, prefs.url_len())
            .await
            .unwrap();

        println!("{:#?}", short_row);

        assert_eq!(short_row.longurl, "https://example.com");
        assert_eq!(short_row.created_by, None);
        assert_eq!(short_row.clicks, 0);
        return short_row;
    }

    async fn test_retrieve_url(test_short: UrlRow) {
        let (pool, _) = pool_init().await;

        let url_row: UrlRow = test_short;
        assert_eq!(url_row.longurl, "https://example.com");
        assert_eq!(url_row.created_by, None);
        let url_row: String = retrieve_url(url_row.short_url().as_str(), &pool)
            .await
            .unwrap();
        assert_eq!(url_row, "https://example.com");
    }

    #[sqlx::test]
    async fn test_make_and_retrieve() {
        let row = test_make_url().await;
        test_retrieve_url(row).await;
    }

    #[sqlx::test]
    async fn test_delete_url() {
        let (pool, _) = pool_init().await;

        sqlx::query("INSERT INTO urls (id, shorturl, longurl, created_by, clicks) VALUES (1, 'test', 'https://example.com', NULL, 0);")
            .execute(&pool)
            .await
            .unwrap();

        delete_url(1, &pool).await.expect("Error deleting row");
    }

    #[sqlx::test]
    async fn test_delete_nonexistand_url() {
        let (pool, _) = pool_init().await;

        delete_url(1, &pool).await.expect("Error deleting row");
    }

    #[sqlx::test]
    async fn test_retrieve_enonexistant_url() {
        let (pool, _) = pool_init().await;

        let url = "247eadf89a518526cd34fd24aaaaaaaaaa";

        retrieve_url_obj(url, &pool)
            .await
            .expect_err("This url shouldn't exist");
    }
}
