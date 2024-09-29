use core::str;

use rand::{distributions::Alphanumeric, Rng, SeedableRng};
use rand_chacha::ChaChaRng;
use sha2::{Digest, Sha512};
use sqlx::prelude::*;
use zeroize::Zeroizing;

#[derive(FromRow)]
pub struct User {
    id: i64,
    username: String,
    hashed_pw: String,
    email: String,
}

pub async fn create_user_db(
    username: String,
    plain_pw: String,
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

fn hash_password(password: Zeroizing<String>) -> String {
    let rng_gen = ChaChaRng::from_entropy();
    let mut hash_fun = Sha512::new();
    let salt: String = rng_gen
        .sample_iter(&Alphanumeric)
        .take(15)
        .map(char::from)
        .collect();
    let pass_with_salt: Zeroizing<String> =
        Zeroizing::new([salt.as_str(), password.as_str()].join(""));

    hash_fun.update(pass_with_salt);
    let hashed_pw = hash_fun.finalize();
    let hashed_pw = str::from_utf8(&hashed_pw).unwrap();
    let mut password_to_store = salt;
    password_to_store.push('#');
    password_to_store.push_str(hashed_pw);
    return password_to_store;
}
