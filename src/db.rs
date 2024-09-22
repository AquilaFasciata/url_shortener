use super::DEFAULT_URL_LEN;
use core::str;
use sqlx::FromRow;
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
    connection_pool: sqlx::Pool,
) -> Result<&str, sqlx::Error> {
    let mut uuid: &str;
    loop {}
}

pub async fn retrieve_url(url: &str, pool: &sqlx::PgPool) -> Result<UrlRow, sqlx::Error> {
    sqlx::query_as("SELECT $1 FROM urls")
        .bind(url)
        .fetch_one(pool)
        .await
}

fn gen_url_longword(long_url: &str) -> (&str, [u8; uuid::fmt::Simple::LENGTH]) {
    let uuid = Uuid::new_v3(&Uuid::NAMESPACE_OID, &long_url.as_bytes());
    let mut return_buff = [0_u8; uuid::fmt::Simple::LENGTH];
    let return_str = uuid.as_simple().encode_lower(&mut return_buff);
    return (return_str, return_buff);
}
