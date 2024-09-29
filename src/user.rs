use super::{DBNAME, IPADDR, PASS, USER};
use sqlx::{postgres::PgQueryResult, prelude::*};

#[derive(FromRow)]
pub struct User {
    id: i64,
    username: String,
    hashed_pw: String,
    email: String,
}

pub async fn create_user(
    username: String,
    hashed_pw: String,
    email: String,
) -> Result<User, sqlx::Error> {
    let user = User {
        id: -1,
        username,
        hashed_pw,
        email,
    };

    return Ok(user);
}

async fn add_user_to_db(user: User, pool: &sqlx::PgPool) -> Result<i64, sqlx::Error> {
    let mut transaction = pool.begin().await?;

    sqlx::query(
        "INSERT INTO users (username, hashed_pw, email) VALUES ($1, $2, $3);
        SELECT currval('users_id_seq');",
    )
    .bind(user.username)
    .bind(user.hashed_pw)
    .bind(user.email)
    .execute(&mut *transaction)
    .await?;

    let id: i64 = sqlx::query_scalar("SELECT currval('users_id_seq')")
        .fetch_one(&mut *transaction)
        .await?;

    transaction.commit().await?;

    return Ok(id);
}
