use super::DEFAULT_URL_LEN;
use core::str;
use sqlx::FromRow;
use std::str::from_utf8;
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

pub fn create_url(
    long_url: &str,
    user_id: Option<i64>,
    connection_pool: sqlx::Pool,
) -> Result<&str, sqlx::Error> {
}

fn gen_url_longword(long_url: &str) -> &str {
    let mut buffer: [u8; 16] = [0; 16];
    let uuid = Uuid::new_v3(&Uuid::NAMESPACE_OID, &long_url.as_bytes());
    uuid.clone().as_simple().encode_lower(&mut buffer);
    let buff_str = str::from_utf8(&(buffer.clone())).unwrap();
    return buff_str;
}
