use rand::{distributions::Alphanumeric, Rng, SeedableRng};
use rand_chacha::ChaChaRng;
use sha2::{Digest, Sha512};
use tracing::{debug, instrument};
use zeroize::Zeroizing;

use crate::url_db::UserRow;

/// Creates a new user from user, pass, and email, inserts into DB, and returns the created row or
/// sql error
pub async fn new_user(
    username: String,
    plain_pw: String,
    email: String,
    pool: &sqlx::PgPool,
) -> Result<UserRow, sqlx::Error> {
    let mut new_user = create_user_for_db(username, plain_pw, email).await?;
    let new_user_id = add_user_to_db(&new_user, &pool).await?;

    new_user.update_id(new_user_id);

    Ok(new_user)
}

pub async fn create_user_for_db(
    username: String,
    plain_pw: String,
    email: String,
) -> Result<UserRow, sqlx::Error> {
    let hashed_pw = hash_unsalted_password(Zeroizing::new(plain_pw));
    let user = UserRow::new(-1, username, hashed_pw, email);

    Ok(user)
}

async fn add_user_to_db(user: &UserRow, pool: &sqlx::PgPool) -> Result<i64, sqlx::Error> {
    let mut transaction = pool.begin().await?;

    sqlx::query("INSERT INTO users (username, hashed_pw, email) VALUES ($1, $2, $3);")
        .bind(user.username())
        .bind(user.hashed_pw())
        .bind(user.email())
        .execute(&mut *transaction)
        .await?;

    let id: i64 = sqlx::query_scalar("SELECT currval('users_id_seq')")
        .fetch_one(&mut *transaction)
        .await?;

    transaction.commit().await?;

    Ok(id)
}

pub async fn retrieve_user_by_id(id: i64, pool: &sqlx::PgPool) -> Result<UserRow, sqlx::Error> {
    sqlx::query_as("SELECT * FROM users WHERE id=$1 LIMIT 1")
        .bind(id)
        .fetch_one(pool)
        .await
}

fn hash_unsalted_password(password: Zeroizing<String>) -> String {
    let mut hash_fun = Sha512::new();

    let (password, salt) = salt_password(password);
    hash_fun.update(password);
    let hashed_pw = hash_fun.finalize();
    let hashed_pw = hex::encode(hashed_pw);
    let mut password_to_store = salt;
    password_to_store.push('#');
    password_to_store.push_str(hashed_pw.as_str());
    password_to_store
}

fn hash_salted_password(password: Zeroizing<String>) -> String {
    let mut hash_fun = Sha512::new();

    hash_fun.update(password);
    let hashed_pw = hash_fun.finalize();
    let hashed_pw = hex::encode(hashed_pw);
    hashed_pw
}

/// Used to salt a plain password. Returns a tuple with (hashed_pw, salt)
fn salt_password(password: Zeroizing<String>) -> (Zeroizing<String>, String) {
    let rng_gen = ChaChaRng::from_entropy();
    let salt: String = rng_gen
        .sample_iter(&Alphanumeric)
        .take(15)
        .map(char::from)
        .collect();
    let pass_with_salt: Zeroizing<String> =
        Zeroizing::new([salt.as_str(), password.as_str()].join(""));
    (pass_with_salt, salt)
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
    let hashed_pw = hash_salted_password(salted_password);
    debug!("Whole hash in db is {}", user.hashed_pw());
    let stored_hash = user.hashed_pw().as_str().split_at(delimiter_index).1;
    debug!(
        "Comparing passwords --
         Input hash: {hashed_pw}
        Stored hash: {stored_hash}"
    );
    return hashed_pw == stored_hash;
}

pub async fn delete_user_from_db(id: i64, pool: &sqlx::PgPool) -> Result<u64, sqlx::Error> {
    let result = sqlx::query("DELETE FROM users WHERE id=$1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected())
}

#[cfg(test)]
mod tests {
    use sqlx::postgres::PgPoolOptions;
    use tracing::Level;
    use zeroize::Zeroizing;

    use super::*;

    const USER: &str = "postgres";
    const PASS: &str = env!(
        "db_pass",
        "Please set db_pass env variable \
        with your PostgreSQL password"
    );
    const MAX_CONN: u32 = 10;

    async fn pool_init() -> sqlx::PgPool {
        let conn_url = format!("postgres://{USER}:{PASS}@172.17.0.2/testdb");
        let pool = PgPoolOptions::new()
            .max_connections(MAX_CONN)
            .connect(&conn_url)
            .await
            .expect("Couldn't create connection pool. Are your credentials correct?");

        return pool;
    }

    impl UserRow {
        fn user_with_pass(pass: String) -> UserRow {
            UserRow::new(-1, String::from("test"), pass, String::from("test"))
        }
    }

    #[sqlx::test]
    fn verify_matching_pw() {
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

    #[sqlx::test]
    async fn create_user() {
        let pool = pool_init().await;

        let user = new_user(
            String::from("test"),
            String::from("Test"),
            String::from("email"),
            &pool,
        )
        .await
        .unwrap();

        let returned_user = retrieve_user_by_id(*user.id(), &pool).await.unwrap();
        assert_eq!(format!("{:?}", user), format!("{:?}", returned_user));
    }
}
