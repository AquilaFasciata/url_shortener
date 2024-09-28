use super::DEFAULT_URL_LEN;
use base64::{engine::general_purpose, prelude::*};
use rand::{
    distributions::{Alphanumeric, DistString},
    prelude::*,
};
use sqlx::{postgres::PgQueryResult, FromRow};
use std::{result::Result, str};

#[derive(FromRow, Debug)]
pub struct UrlRow {
    // If fields are updated, update UrlRowIterator
    id: i64,
    shorturl: String,
    longurl: String,
    created_by: Option<i64>,
    clicks: i64,
}

#[derive(FromRow)]
pub struct UserRow {
    id: i64,
    username: String,
    hashed_pw: String,
    email: String,
}

/// Creates a UrlRow, inserts it into the PostgreSQL databse, and returns the created UrlRow object
pub async fn create_url(
    long_url: &str,
    user_id: Option<i64>,
    connection_pool: &sqlx::PgPool,
) -> Result<UrlRow, sqlx::Error> {
    let temp_long = gen_url_longword(long_url);
    let mut short_url = String::new();

    // Cycle through intil there is a window that is unused
    for keyword in temp_long.windows(DEFAULT_URL_LEN) {
        let keyword_str =
            str::from_utf8(keyword).expect("Error parsing str. This shouldn't be possible!");
        match retrieve_url(keyword_str, &connection_pool).await {
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
        let mut rng = thread_rng();
        loop {
            short_url = Alphanumeric.sample_string(&mut rng, DEFAULT_URL_LEN);
            let req_result = retrieve_url(&short_url, &connection_pool).await;
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

    new_row.id = url_db_create(&new_row, &connection_pool).await?;

    return Ok(new_row);
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
    return Ok(response);
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
    return Ok(response);
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

    let new_id = retrieve_url_obj(new_row.shorturl.as_str(), &pool).await?.id;
    return Ok(new_id);
}

#[cfg(test)]
mod tests {
    use sqlx::postgres::PgPoolOptions;

    use super::*;

    const USER: &str = "postgres";
    const PASS: &str = env!(
        "db_pass",
        "Please set db_pass env variable \
        with your PostgreSQL password"
    );
    const MAX_CONN: u32 = 10;
    static mut TEST_SHORT: String = String::new();

    #[sqlx::test]
    async fn make_url() {
        let conn_url = format!("postgres://{USER}:{PASS}@172.17.0.2/testdb");
        let pool = PgPoolOptions::new()
            .max_connections(MAX_CONN)
            .connect(&conn_url)
            .await
            .unwrap();

        let short_row: UrlRow = create_url("https://example.com", None, &pool)
            .await
            .unwrap();

        unsafe {
            TEST_SHORT = short_row.shorturl.clone();
        }

        println!("{:#?}", short_row);

        assert_eq!(short_row.longurl, "https://example.com");
        assert_eq!(short_row.created_by, None);
        assert_eq!(short_row.clicks, 0);
    }

    #[sqlx::test]
    async fn test_retrieve_url() {
        let conn_url = format!("postgres://{USER}:{PASS}@172.17.0.2/testdb");
        let pool = PgPoolOptions::new()
            .max_connections(MAX_CONN)
            .connect(&conn_url)
            .await
            .unwrap();

        let url_row: UrlRow;
        unsafe {
            url_row = retrieve_url_obj(TEST_SHORT.as_str(), &pool).await.unwrap();
            println!("Short url is: {}", TEST_SHORT);
        }
        assert_eq!(url_row.longurl, "https://example.com");
        assert_eq!(url_row.created_by, None);
        let url_row: String;
        unsafe {
            url_row = retrieve_url(TEST_SHORT.as_str(), &pool).await.unwrap();
            println!("Short url is: {}", TEST_SHORT);
        }
        assert_eq!(url_row, "https://example.com");
    }
}
