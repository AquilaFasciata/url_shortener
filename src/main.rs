use axum::{routing::get, Router, Html};
use tokio::fs;

#[tokio::main]
async fn main() {
    let router = Router::new();

    let app = router.route("/", get(root()));
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn root() -> Html< {
    let contents = fs::read("html/index.html");
    return Html(contents)
}
