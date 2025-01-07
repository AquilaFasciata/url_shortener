use std::{
    collections::HashMap,
    fs,
    net::SocketAddr,
    sync::Arc,
    time::{self, UNIX_EPOCH},
};

use askama::Template;
use axum::{
    body::{Body, Bytes},
    extract::{Path, State},
    http::{
        header::{self, HeaderValue, CONTENT_LENGTH, CONTENT_TYPE},
        response, StatusCode,
    },
    response::{Html, IntoResponse, Response},
    routing::{get, post},
    Router,
};
use axum_server::tls_rustls::RustlsConfig;
use preferences::Preferences;
use regex::Regex;
use serde::Deserialize;
use sqlx::{postgres::PgPoolOptions, PgPool};
use tracing::{debug, info, Level};
use url_db::{UrlRow, UserRow};
use user::jwt::{self, Jwt, JwtHeader, JwtPayload, SigAlgo};

mod preferences;
mod url_db;
mod user;

struct PoolAndPrefs {
    pool: PgPool,
    prefs: Preferences,
}

impl PoolAndPrefs {
    fn pool(&self) -> &PgPool {
        &self.pool
    }
    fn prefs(&self) -> &Preferences {
        &self.prefs
    }
    fn both(&self) -> (&PgPool, &Preferences) {
        (&self.pool, &self.prefs)
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

    let mut config: Option<RustlsConfig> = None;
    if cert.is_some() && key.is_some() {
        let temp = RustlsConfig::from_pem_file(cert.as_ref().unwrap(), key.as_ref().unwrap());
        let conf_fut;
        (conf_fut, pool) = tokio::join!(temp, pool_fut);
        config = Some(conf_fut.unwrap());
    } else {
        pool = pool_fut.await;
    }

    let pool_and_prefs = PoolAndPrefs {
        pool: pool.expect("Error creating connection pool. {}"),
        prefs: prefs.clone(),
    };

    let arc_pool_prefs: Arc<PoolAndPrefs> = Arc::new(pool_and_prefs);

    let app = router
        .route("/", get(root))
        .route("/:extra", get(subdir_handler))
        .with_state(arc_pool_prefs.clone())
        .route("/", post(post_new_url))
        .with_state(arc_pool_prefs.clone())
        .route("/:extra/:extra", get(subdir_handler))
        .with_state(arc_pool_prefs.clone());
    let address = SocketAddr::from(([127, 0, 0, 1], u16::try_from(prefs.port()).unwrap()));
    info!(
        "Listening on {}:{} for connections!",
        prefs.http_ip(),
        prefs.port()
    );

    if config.is_some() {
        axum_server::bind_rustls(address, config.unwrap())
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

async fn post_new_url(
    State(pool_and_prefs): State<Arc<PoolAndPrefs>>,
    body: Bytes,
) -> Response<Body> {
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
async fn subdir_handler(
    Path(path): Path<String>,
    State(pool): State<Arc<PoolAndPrefs>>,
) -> Response {
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

async fn attempt_login(
    State(pool_and_prefs): State<Arc<PoolAndPrefs>>,
    body: Bytes,
) -> Response<Body> {
    let (pool, prefs) = pool_and_prefs.both();
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
    } else {
        todo!()
    }

    return StatusCode::OK.into_response();
}
