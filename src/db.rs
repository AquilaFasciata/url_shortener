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

pub fn create_url(
    long_url: &str,
    user_id: Option<i64>,
    connection_pool: sqlx::Pool,
) -> Result<&str, sqlx::Error> {
}

fn gen_url_keyword(long_url: &str) -> &str {
    let uuid = Uuid::new_v3(&Uuid::NAMESPACE_OID, &long_url.as_bytes());
}

fn does_keyword_exist(keyword: &str, pool: sqlx::PgPool) -> bool {}
