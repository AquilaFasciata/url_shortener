use core::str;

use hex::ToHex;
use rand::{distributions::Alphanumeric, Rng, SeedableRng};
use rand_chacha::ChaChaRng;
use sha2::{Digest, Sha512};
use tracing::{debug, instrument};
use zeroize::Zeroizing;

use crate::url_db::UserRow;

enum PasswordResult {
    Match,
    NoMatch,
    NoUser,
}

pub async fn create_user_db(
    username: String,
    plain_pw: String,
    email: String,
) -> Result<UserRow, sqlx::Error> {
    let hashed_pw = hash_password(Zeroizing::new(plain_pw));
    let user = UserRow::new(-1, username, hashed_pw, email);

    return Ok(user);
}

async fn add_user_to_db(user: UserRow, pool: &sqlx::PgPool) -> Result<i64, sqlx::Error> {
    let mut transaction = pool.begin().await?;

    sqlx::query(
        "INSERT INTO users (username, hashed_pw, email) VALUES ($1, $2, $3);
        SELECT currval('users_id_seq');",
    )
    .bind(user.username())
    .bind(user.hashed_pw())
    .bind(user.email())
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
    let hashed_pw = hex::encode(hashed_pw);
    let mut password_to_store = salt;
    password_to_store.push('#');
    password_to_store.push_str(hashed_pw.as_str());
    return password_to_store;
}

#[instrument]
pub async fn verify_pw(password: Zeroizing<String>, user: &UserRow) -> bool {
    let mut salted_password = Zeroizing::new(String::new());
    salted_password.reserve(14);

    // Get salt from hashed_pw
    let mut delimiter_index: usize = 0;
    for (i, letter) in user.hashed_pw().as_bytes().iter().enumerate() {
        if *letter != b'#' {
            let letter = char::from(*letter);
            salted_password.push(letter);
            continue;
        }
        delimiter_index = i + 1;
        break;
    }

    salted_password.push_str(password.as_str());
    let hashed_pw = hash_password(salted_password);
    debug!("Whole hash in db is {}", user.hashed_pw());
    let stored_hash = user.hashed_pw().as_str().split_at(delimiter_index).1;
    debug!("Comparing passwords -- Input hash: {hashed_pw}     Stored hash: {stored_hash}");
    if hashed_pw == stored_hash {
        return true;
    } else {
        return false;
    }
}

#[cfg(test)]
mod tests {
    use tracing::Level;
    use zeroize::Zeroizing;

    use super::*;

    impl UserRow {
        fn user_with_pass(pass: String) -> UserRow {
            UserRow::new(-1, String::from("test"), pass, String::from("test"))
        }
    }

    #[sqlx::test]
    fn verify_verify_pw() {
        let subscriber = tracing_subscriber::FmtSubscriber::builder()
            .with_level(true)
            .with_max_level(Level::DEBUG)
            .pretty()
            .finish();

        tracing::subscriber::set_global_default(subscriber)
            .expect("Couldn't set subscriber for tracing");
        let hashed_pass: Zeroizing<String> = Zeroizing::new(String::from("12#4c3fdfe4efb17076577bfedcb6e1fbfff4d14abfdb8f0fc81c9a66fc5ed6a98d0b6e17b1b7175a29a5c4654743bef584feb48655a7701a7a31f8d7bf98e3222d"));
        let user = UserRow::user_with_pass(hashed_pass.clone().to_string());
        let clear_pass = Zeroizing::new(String::from("test"));
        assert!(super::verify_pw(clear_pass, &user).await);
    }
}
