use super::DEFAULT_URL_LEN;
use rand::{
    distributions::{Alphanumeric, DistString},
    prelude::*,
};
use sqlx::{postgres::PgQueryResult, FromRow};
use std::{result::Result, str};
use uuid::Uuid;

#[derive(FromRow, Debug)]
pub struct UrlRow {
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

    let new_row = UrlRow {
        id: -1,
        shorturl: short_url.clone(),
        longurl: long_url.to_string(),
        created_by: user_id,
        clicks: 0,
    };

    url_db_create(&new_row, &connection_pool).await?;

    return Ok(new_row);
}

pub async fn retrieve_url(
    url: &str,
    pool: &sqlx::PgPool,
) -> Result<std::string::String, sqlx::Error> {
    let response: UrlRow = sqlx::query_as("SELECT * FROM urls WHERE shorturl = $1")
        .bind(url)
        .fetch_one(pool)
        .await?;
    return Ok(response.longurl);
}

pub async fn retrieve_url_obj(url: &str, pool: &sqlx::PgPool) -> Result<UrlRow, sqlx::Error> {
    let response: UrlRow = sqlx::query_as("SELECT * FROM urls WHERE shorturl = $1")
        .bind(url)
        .fetch_one(pool)
        .await?;
    return Ok(response);
}

fn gen_url_longword(long_url: &str) -> Vec<u8> {
    let uuid = Uuid::new_v3(&Uuid::NAMESPACE_OID, &long_url.as_bytes());
    let mut return_buff = [0_u8; uuid::fmt::Simple::LENGTH];
    uuid.as_simple().encode_lower(&mut return_buff);
    return return_buff.to_vec();
}

async fn url_db_create(
    new_row: &UrlRow,
    pool: &sqlx::PgPool,
) -> Result<PgQueryResult, sqlx::Error> {
    sqlx::query("INSERT INTO urls (shorturl, longurl, created_by, clicks) VALUES ($1, $2, $3, 0)")
        .bind(new_row.shorturl.clone())
        .bind(new_row.longurl.clone())
        .bind(new_row.created_by)
        .execute(pool)
        .await
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
    const DEFAULT_URL_LEN: usize = 6;
    static mut TEST_SHORT: String = String::new();

    #[sqlx::test]
    async fn make_url() {
        let conn_url = format!("postgres://{USER}:{PASS}@172.17.0.2/testdb");
        let pool = PgPoolOptions::new()
            .max_connections(3)
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
            .max_connections(3)
            .connect(&conn_url)
            .await
            .unwrap();

        let url_row: UrlRow;
        unsafe {
            url_row = retrieve_url_obj(TEST_SHORT.as_str(), &pool).await.unwrap();
        }
        assert_eq!(url_row.longurl, "https://example.com");
        assert_eq!(url_row.created_by, None);
    }
}
