use axum::{
    extract::Path,
    http::{HeaderMap, StatusCode},
    response::{Html, IntoResponse, Response},
    routing::get,
    Router,
};
use tokio::fs;

#[tokio::main]
async fn main() {
    let router = Router::new();

    let app = router
        .route("/", get(root))
        .route("/:extra", get(derivative));
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn root() -> Response {
    let contents = fs::read("html/index.html").await.unwrap();
    let html = Html::from(contents);
    return html.into_response();
}

async fn derivative(Path(extra): Path<String>) -> Response {
    // TODO Seperate Html and CSS responses
    let mut path = String::from("html/");
    let mut extra = extra;
    if extra.starts_with("..") {
        extra.drain(0..2);
    }
    println!("Retriving file {}", &extra);
    path.push_str(extra.as_str());
    let contents = match fs::read(path).await {
        Ok(content) => content,
        Err(_) => return StatusCode::NOT_FOUND.into_response(),
    };
    let html = Html::from(contents);
    return html.into_response();
}
