use sqlx::FromRow;
use rand::SeedableRng;

#[derive(FromRow)]
pub struct UrlRow {
    id: i64,
    shorturl: String,
    longurl: String,
    created_by: Option<i64>,
    clicks: i64
}

#[derive(FromRow)]
pub struct UserRow {
    id: i64,
    username: String,
    hashed_pw: String,
    email: String
}

pub fn create_url(long_url: &str, user_id: Option<i64>, connection_pool: sqlx::Pool) -> Result<&str, sqlx::Error> {
     
}

fn gen_url_keyword(long_url: &str) {
    let alphabet_arr = [
        ['a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j'],
        ['k', 'l', 'm', 'n', 'o', 'p', 'q', 'r', 's', 't'],
        ['u', 'v', 'w', 'x', 'y', 'z', 'a', 'b', 'c', 'd']
    ];
}
