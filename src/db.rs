use super::DEFAULT_URL_LEN;
use rand::{
    distributions::{Alphanumeric, DistString},
    prelude::*,
};
use sqlx::{postgres::PgQueryResult, FromRow};
use std::{result::Result, str};
use uuid::Uuid;

#[derive(FromRow)]
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
    connection_pool: sqlx::PgPool,
) -> Result<String, sqlx::Error> {
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
            if retrieve_url(&short_url, &connection_pool)
                .await
                .unwrap()
                .is_empty()
            {
                break;
            }
        }
    }

    let new_row = UrlRow {
        id: -1,
        shorturl: short_url.clone(),
        longurl: long_url.to_string(),
        created_by: user_id,
        clicks: -1,
    };

    url_db_create(new_row, &connection_pool).await?;

    return Ok(short_url);
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

fn gen_url_longword(long_url: &str) -> Vec<u8> {
    let uuid = Uuid::new_v3(&Uuid::NAMESPACE_OID, &long_url.as_bytes());
    let mut return_buff = [0_u8; uuid::fmt::Simple::LENGTH];
    uuid.as_simple().encode_lower(&mut return_buff);
    return return_buff.to_vec();
}

async fn url_db_create(new_row: UrlRow, pool: &sqlx::PgPool) -> Result<PgQueryResult, sqlx::Error> {
    sqlx::query("INSERT INTO url (shorturl, longurl, created_by, clicks) VALUES ($1, $2, $3, 0)")
        .bind(new_row.shorturl)
        .bind(new_row.longurl)
        .bind(new_row.created_by)
        .execute(pool)
        .await
}

#[cfg(test)]
mod tests {
    use sqlx::postgres::PgPoolOptions;

    use super::*;
    use super::super::{PASS, USER};

    #[test]
    fn make_url() {
        let url = format!("postgres://{USER}:{PASS}@172.17.0.2/shortener");
        let pool = PgPoolOptions::new()
            .max_connections(2)
            .connect(url.as_str())
        let url = create_url("https://example.com", None, )
    }
}
