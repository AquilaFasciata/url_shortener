use sqlx::{Executor, query::Query};

pub fn create_url(long_url: &str, user_id: Option<i64>, connection_pool: sqlx::Pool) -> Result {
    let query = 
}
