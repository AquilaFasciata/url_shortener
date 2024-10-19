use std::collections::HashMap;

use askama::Template;
use axum::{
    body::{Body, Bytes},
    debug_handler,
    extract::{Path, State},
    http::{
        header::{self, HeaderValue},
        StatusCode,
    },
    response::{Html, IntoResponse, Response},
    routing::{get, post},
    Router,
};
use preferences::Preferences;
use regex::Regex;
use sqlx::{postgres::PgPoolOptions, PgPool};
use std::sync::OnceLock;
use tokio::fs;
use tracing::{debug, Level};

mod preferences;
mod url_db;
mod user;

// This is only for development -- will move out to env variable or conf file.
const USER: &str = "postgres";
const PASS: &str = env!(
    "db_pass",
    "Please set db_pass env variable \
    with your PostgreSQL password"
);
const MAX_CONN: u32 = 10;
#[allow(dead_code)]
const DEFAULT_URL_LEN: usize = 6;
const DBNAME: &str = "shortener";
const IPADDR: &str = "172.17.0.2";

#[tokio::main]
async fn main() -> Result<(), sqlx::Error> {
    let prefs = Preferences::load_config(".");
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(Level::DEBUG)
        .compact()
        .with_thread_ids(true)
        .with_level(true)
        .finish();
    let _ = tracing::subscriber::set_global_default(subscriber)
        .map_err(|_err| eprintln!("Error setting subscriber!"));

    let router = Router::new();
    let url = format!("postgres://{USER}:{PASS}@{IPADDR}/{DBNAME}");
    // This pool is to be used throughout
    let pool = PgPoolOptions::new()
        .max_connections(MAX_CONN)
        .connect(url.as_str())
        .await?;

    let app = router
        .route("/", get(root))
        .route("/:extra", get(subdir_handler))
        .with_state(pool.clone())
        .route("/", post(post_new_url))
        .with_state(pool.clone());
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await.unwrap();
    axum::serve(listener, app).await.unwrap();

    Ok(())
}

#[debug_handler]
async fn post_new_url(State(pool): State<sqlx::PgPool>, body: Bytes) -> Response<Body> {
    let longurl: HashMap<String, String> =
        serde_html_form::from_bytes(&body).expect("Error deserializing form response");
    let new_url = url_db::create_url(&longurl["url"], None, &pool)
        .await
        .unwrap();
    new_url.render().unwrap().into_response()
}

async fn root() -> Response {
    let contents = fs::read("html/index.html").await.unwrap();
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
    let contents = match fs::read(&path).await {
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
        _ => not_found_handler().await,
    }
}

async fn consume_short_url(Path(url): Path<String>, State(pool): State<PgPool>) -> Response {
    let url_row = match url_db::retrieve_url_obj(url.as_str(), &pool).await {
        Ok(row) => row,
        Err(_) => return not_found_handler().await,
    };

    Response::builder()
        .status(301) // Status 301: Moved permanently
        .header(header::LOCATION, url_row.long_url())
        .body(Body::empty())
        .unwrap()
}

/// This theoretically handles all of the incoming requests. If it matches a file extention (html
/// and css at the moment) then it returns that from the server. Otherwise, it will assume it is a
/// short url and send it to the handler.
async fn subdir_handler(Path(path): Path<String>, State(pool): State<PgPool>) -> Response {
    const FILE_EXTENTIONS: [&str; 9] = [
        "html",
        "css",
        "ico",
        "png",
        "jpg",
        "webp",
        "xml",
        "csv",
        "webmanifest",
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
        return consume_short_url(Path(path), State(pool)).await;
    }
}

fn content_response(contents: Vec<u8>, content_type: HeaderValue) -> Response {
    let mut resp = Body::from(contents).into_response();
    resp.headers_mut()
        .insert(header::CONTENT_TYPE, content_type);
    resp
}

async fn not_found_handler() -> Response {
    let content = fs::read("html/404.html").await.unwrap();
    Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body(Body::from(content))
        .expect("Failed to build 404 response")
}
