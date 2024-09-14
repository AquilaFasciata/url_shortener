use axum::{
    body::Body,
    extract::Path,
    http::{
        header::{self, HeaderValue},
        StatusCode,
    },
    response::{Html, IntoResponse, Response},
    routing::get,
    Router,
};
use regex::Regex;
use tokio::fs;
use sqlx::postgres::PgPoolOptions;

mod db;

// This is only for development -- will move out to env variable or conf file.
const USER: &str = "postgres";
const PASS: &str = env!("db_pass", "Please set db_pass env variable \
    with your PostgreSQL password");

#[tokio::main]
async fn main() -> Result<(), sqlx::Error> {
    let router = Router::new();
    let url = format!("postgres://{USER}:{PASS}@172.17.0.2/shortener");
    println!("Connecting to {}", &url);
    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(url.as_str()).await?;

    let row: (i64, ) = sqlx::query_as("SELECT $1")
        .bind(150_i64)
        .fetch_one(&pool).await?;

    println!("{}", row.0);

    let app = router
        .route("/", get(root))
        .route("/:extra", get(derivative));
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await.unwrap();
    axum::serve(listener, app).await.unwrap();

    Ok(())
}

async fn root() -> Response {
    let contents = fs::read("html/index.html").await.unwrap();
    let html = Html::from(contents);
    return html.into_response();
}

#[forbid(unsafe_code)]
async fn derivative(Path(extra): Path<String>) -> Response {
    // TODO Seperate Html and CSS responses
    let mut path = String::from("html/");
    if extra.contains("..") {
        return StatusCode::FORBIDDEN.into_response();
    }
    println!("Retriving file {}", &extra);
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
        ".css" => return content_response(contents, HeaderValue::from_static("text/css")),
        _ => return not_found_handler().await,
    }
}

fn content_response(contents: Vec<u8>, content_type: HeaderValue) -> Response {
    let mut resp = Body::from(contents).into_response();
    resp.headers_mut()
        .insert(header::CONTENT_TYPE, content_type);
    return resp;
}

async fn not_found_handler() -> Response {
    let content = fs::read("html/404.html").await.unwrap();
    Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body(Body::from(content))
        .expect("Failed to build 404 response")
}
