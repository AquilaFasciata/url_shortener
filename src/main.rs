use std::{
    collections::{BTreeMap, HashMap},
    env, fs,
    net::SocketAddr,
    time::{self, UNIX_EPOCH},
};

use askama::Template;
use axum::{
    body::{Body, Bytes},
    extract::{Path, State},
    http::{
        header::{self, HeaderValue, CONTENT_LENGTH, CONTENT_TYPE, LOCATION, SET_COOKIE},
        response, HeaderMap, HeaderName, StatusCode,
    },
    response::{Html, IntoResponse, Response},
    routing::{get, post},
    Router,
};
use axum_server::tls_rustls::RustlsConfig;
use jsonwebtoken::{DecodingKey, EncodingKey};
use preferences::Preferences;
use regex::Regex;
use serde::Deserialize;
use sqlx::{postgres::PgPoolOptions, PgPool};
use tokio;
use tracing::{debug, info, Level};
use url_db::{UrlRow, UserRow};

mod preferences;
mod url_db;
mod user;

const AUTH_COOKIE_NAME: &str = "Bearer";

pub enum AuthenticationResponse {
    Authenticated(UserRow),
    NotAuthenticated,
    Error(AuthError),
}

pub enum AuthError {
    NoCookieHeader,
    InvalidCookieHeader,
    SqlError,
}

struct MasterState {
    pool: PgPool,
    prefs: Preferences,
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
}

impl MasterState {
    fn pool(&self) -> &PgPool {
        &self.pool
    }
    fn prefs(&self) -> &Preferences {
        &self.prefs
    }
    fn pool_and_prefs(&self) -> (&PgPool, &Preferences) {
        (&self.pool, &self.prefs)
    }

    fn encoding_key(&self) -> &EncodingKey {
        &self.encoding_key
    }

    fn decoding_key(&self) -> &DecodingKey {
        &self.decoding_key
    }
}

#[derive(Deserialize)]
struct LoginPayload<'a> {
    username: &'a str,
    password: &'a str,
}

impl<'a> LoginPayload<'a> {
    fn username(&self) -> &str {
        &self.username
    }
    fn password(&self) -> &str {
        &self.password
    }
}

#[tokio::main]
async fn main() -> Result<(), sqlx::Error> {
    /*****************
     * Initalization *
     *****************/
    let args: Vec<String> = env::args().collect();
    if args.contains(&String::from("config")) {
        preferences::create_default_config("config.toml");
        return Ok(());
    }
    let prefs = Preferences::load_config("config.toml").expect("Error loading configuration.");
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(Level::DEBUG)
        .compact()
        .with_thread_ids(true)
        .with_level(true)
        .finish();
    let _ = tracing::subscriber::set_global_default(subscriber)
        .map_err(|_err| eprintln!("Error setting subscriber!"));

    let cert = prefs.https_cert_path();
    let key = prefs.https_key_path();

    let router = Router::new();

    let url = format!(
        "postgres://{}:{}@{}/{}",
        prefs.db_user(),
        prefs.db_pass(),
        prefs.db_ip(),
        prefs.db_name()
    );
    // This pool is to be used throughout
    let pool_fut = PgPoolOptions::new()
        .max_connections(prefs.db_pool_size())
        .connect(url.as_str());
    let pool: Result<PgPool, sqlx::Error>;

    let mut tls_config: Option<RustlsConfig> = None;
    if cert.is_some() && key.is_some() {
        // If there are keys, populate them...
        let temp = RustlsConfig::from_pem_file(cert.as_ref().unwrap(), key.as_ref().unwrap());
        let conf_fut;
        (conf_fut, pool) = tokio::join!(temp, pool_fut); // ...and join the pool
        tls_config = Some(conf_fut.unwrap());
    } else {
        // Otherwise, just wait for the pool
        pool = pool_fut.await;
    }

    // JWT Token Keys
    let encoding_key = EncodingKey::from_secret(prefs.jwt_secret().as_bytes());
    let decoding_key = DecodingKey::from_secret(prefs.jwt_secret().as_bytes());

    let master_state = MasterState {
        pool: pool.expect("Error creating connection pool. {}"),
        prefs: prefs.clone(),
        encoding_key,
        decoding_key,
    };
    let box_master_state = Box::leak(Box::new(master_state));

    sqlx::migrate!("./migrations")
        .run(box_master_state.pool())
        .await
        .unwrap_or_else(|_| debug!("Migration already exists, skipping"));

    /*************
     * Main Loop *
     *************/

    let app = router
        .route("/", get(root))
        .route("/:extra", get(subdir_handler))
        .with_state(box_master_state)
        .route("/", post(post_new_url))
        .with_state(box_master_state)
        .route("/:extra/:extra", get(subdir_handler))
        .with_state(box_master_state);
    let address = SocketAddr::from(([127, 0, 0, 1], u16::try_from(prefs.port()).unwrap()));
    info!(
        "Listening on {}:{} for connections!",
        prefs.http_ip(),
        prefs.port()
    );

    if tls_config.is_some() {
        axum_server::bind_rustls(address, tls_config.unwrap())
            .serve(app.into_make_service())
            .await
            .unwrap();
    } else {
        let listener =
            tokio::net::TcpListener::bind(format!("{}:{}", prefs.http_ip(), prefs.port()).as_str())
                .await
                .unwrap();
        axum::serve(listener, app.into_make_service())
            .await
            .unwrap();
    }

    Ok(())
}

async fn post_new_url(State(pool_and_prefs): State<&MasterState>, body: Bytes) -> Response<Body> {
    let prefs = pool_and_prefs.prefs();
    let longurl: HashMap<String, String> =
        serde_html_form::from_bytes(&body).expect("Error deserializing form response");
    let new_url = url_db::create_url(
        &longurl["url"],
        None,
        pool_and_prefs.pool(),
        pool_and_prefs
            .prefs
            .url_len()
            .try_into()
            .expect("Error converting url_len to usize. {}"),
    )
    .await
    .unwrap();
    let rendered = new_url.render().unwrap();
    let rendered = rendered.split_once(new_url.short_url()).unwrap();

    let replaced_second = rendered.1.replace(
        new_url.short_url(),
        format!("{}/{}", prefs.domain_name(), new_url.short_url()).as_str(),
    );
    format!("{}{}{}", rendered.0, new_url.short_url(), replaced_second).into_response()
}

async fn root() -> Response {
    let contents = fs::read("html/index.html").unwrap();
    let html = Html::from(contents);
    html.into_response()
}

#[forbid(unsafe_code)]
async fn derivative(Path(extra): Path<String>) -> Response {
    // TODO Seperate Html and CSS responses
    let mut path = String::from("html/");
    if extra.contains("..") {
        return StatusCode::FORBIDDEN.into_response();
    }
    path.push_str(extra.as_str());
    let contents = match fs::read(&path) {
        Ok(content) => content,
        Err(_) => return not_found_handler().await,
    };

    let file_ext_regex: Regex = Regex::new(r"\.\w+$").expect("Error creating Regex match");
    let file_ext = match file_ext_regex.find(path.as_str()) {
        Some(strtype) => strtype,
        None => return StatusCode::NOT_FOUND.into_response(),
    };
    match file_ext.as_str() {
        ".html" => Html::from(contents).into_response(),
        ".css" => content_response(contents, HeaderValue::from_static("text/css")),
        ".jpg" => image_load(path.as_str(), file_ext.as_str()),
        ".webp" => image_load(path.as_str(), file_ext.as_str()),
        // Icos don't have a content type that matches the file ext.
        ".ico" => {
            let image = match fs::read(path) {
                Ok(img) => img,
                Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
            };
            response::Builder::new()
                .status(StatusCode::OK)
                .header(CONTENT_TYPE, "image/x-icon")
                .header(CONTENT_LENGTH, image.len())
                .body(image.into())
                .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR.into_response())
        }
        _ => not_found_handler().await,
    }
}

async fn consume_short_url(Path(url): Path<String>, State(pool): State<&PgPool>) -> Response {
    let mut url_row: UrlRow = match url_db::retrieve_url_obj(url.as_str(), &pool).await {
        Ok(row) => row,
        Err(_) => return not_found_handler().await,
    };

    url_db::incr_url_clicks(&mut url_row, pool).await;

    let long = if url_row.long_url().starts_with("http") || url_row.long_url().starts_with("https")
    {
        url_row.long_url()
    } else {
        &format!("http://{}", url_row.long_url())
    };

    Response::builder()
        .status(301) // Status 301: Moved permanently
        .header(header::LOCATION, long)
        .body(Body::empty())
        .unwrap()
}

/// This theoretically handles all of the incoming requests. If it matches a file extention (html
/// and css at the moment) then it returns that from the server. Otherwise, it will assume it is a
/// short url and send it to the handler.
async fn subdir_handler(Path(path): Path<String>, State(pool): State<&MasterState>) -> Response {
    const FILE_EXTENTIONS: [&str; 10] = [
        "html",
        "css",
        "ico",
        "png",
        "jpg",
        "webp",
        "xml",
        "csv",
        "webmanifest",
        "wasm",
    ];
    let split = match path.split('.').last() {
        Some(ext) => ext,
        None => return not_found_handler().await,
    };
    debug!("The file extention is {split}");
    if FILE_EXTENTIONS.contains(&split) {
        debug!("Loading file at {path}");
        return derivative(Path(path)).await;
    } else {
        debug!("Redirecting user based on db result for {path}");
        return consume_short_url(Path(path), State(&pool.pool())).await;
    }
}

fn content_response(contents: Vec<u8>, content_type: HeaderValue) -> Response {
    let mut resp = Body::from(contents).into_response();
    resp.headers_mut()
        .insert(header::CONTENT_TYPE, content_type);
    resp
}

async fn not_found_handler() -> Response {
    let content = fs::read("html/404.html").unwrap();
    Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body(Body::from(content))
        .expect("Failed to build 404 response")
}

fn image_load(path: &str, ext: &str) -> Response {
    let ext = ext.trim_start_matches('.');
    let image = match fs::read(path) {
        Ok(img) => img,
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };
    response::Builder::new()
        .status(StatusCode::OK)
        .header(CONTENT_TYPE, format!("image/{ext}"))
        .header(CONTENT_LENGTH, image.len())
        .body(image.into())
        .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR.into_response())
}

async fn authenticate_request(
    State(pools_and_prefs): State<&MasterState>,
    headers: &HeaderMap,
) -> AuthenticationResponse {
    let prefs = pools_and_prefs.prefs();
    let header_str = match headers.get(HeaderName::from_static("Cookie")) {
        Some(val) => val.to_str().unwrap_or(""),
        None => return AuthenticationResponse::Error(AuthError::NoCookieHeader),
    };

    let mut cookie_map: BTreeMap<&str, &str> = BTreeMap::new();
    let cookie_vec: Vec<&str> = header_str.split_terminator(';').collect();
    for pair in cookie_vec {
        let tup = match pair.trim().split_once('=') {
            Some(val) => val,
            None => return AuthenticationResponse::Error(AuthError::InvalidCookieHeader),
        };

        cookie_map.insert(tup.0, tup.1);
    }

    let token = match cookie_map.get(AUTH_COOKIE_NAME) {
        Some(v) => v,
        None => return AuthenticationResponse::Error(AuthError::InvalidCookieHeader),
    };
    let (token, orig_hash) = match Jwt::from_str_secret(token, prefs.jwt_secret()) {
        Ok(v) => v,
        Err(_) => return AuthenticationResponse::Error(AuthError::InvalidCookieHeader),
    };

    if orig_hash.is_empty() {
        return AuthenticationResponse::NotAuthenticated;
    };
    if token.signature().unwrap_or(String::new()) == orig_hash {
        let user =
            match user::retrieve_user_by_id(token.payload().sub(), pools_and_prefs.pool()).await {
                Ok(v) => v,
                Err(_) => return AuthenticationResponse::Error(AuthError::SqlError), // TODO: Make
                                                                                     // this more
                                                                                     // explicit
            };
        return AuthenticationResponse::Authenticated(user);
    }
    return AuthenticationResponse::NotAuthenticated;
}

async fn attempt_login(State(master_state): State<&MasterState>, body: Bytes) -> Response {
    let (pool, prefs) = master_state.pool_and_prefs();
    let current_time = time::SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let login_data: LoginPayload = match serde_html_form::from_bytes(&body) {
        Ok(parsed) => parsed,
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };

    let user: UserRow = match user::retrieve_user_by_name(login_data.username(), pool).await {
        Ok(user) => user,
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };

    if user::verify_pw(login_data.password(), &user).await {
        let token = Jwt::new(
            JwtHeader::new(SigAlgo::HS256, String::from("JWT")),
            JwtPayload::new(
                *user.id(),
                user.username().to_string(),
                user.email().to_string(),
                current_time,
            ),
        );
        let token_str = format!("__Host-jwt={}; Secure", token.finalize(prefs.jwt_secret()));
        return Response::builder()
            .header(SET_COOKIE, token_str)
            .status(StatusCode::TEMPORARY_REDIRECT)
            .header(LOCATION, "/loggedin.html")
            .body(Body::empty())
            .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR.into_response());
    } else {
        return StatusCode::OK.into_response();
    }
}
