struct UrlRow {
    id: i64,
    shorturl: String,
    longurl: String,
    created_by: Option<i64>,
    clicks: i64
}

struct UserRow {
    id: i64,
    username: String,
    hashed_pw: String,
    email: String
}

pub fn create_url(long_url: &str, user_id: Option<i64>, connection_pool: sqlx::Pool) -> Result<&str, sqlx::Error> {

}
