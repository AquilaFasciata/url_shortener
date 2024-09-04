use axum::{
    http::HeaderMap,
    response::{Html, IntoResponse, Response},
    routing::get,
    Router,
};
use tokio::fs;

#[tokio::main]
async fn main() {
    let router = Router::new();

    let app = router.route("/", get(root));
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn root() -> Response {
    let contents = fs::read("html/index.html").await.unwrap();
    let html = Html::from(contents);
    return html.into_response();
}
