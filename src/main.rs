use axum::{
    body::Body,
    extract::Path,
    http::{
        header::{self, HeaderValue},
        StatusCode,
    },
    response::{Html, IntoResponse, Redirect, Response},
    routing::get,
    Router,
};
use regex::Regex;
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
        Err(_) => return StatusCode::NOT_FOUND.into_response(),
    };

    let file_ext_regex: Regex = Regex::new(r"\.\w+$").expect("Error creating Regex match");
    let file_ext = match file_ext_regex.find(path.as_str()) {
        Some(strtype) => strtype,
        None => return StatusCode::NOT_FOUND.into_response(),
    };
    match file_ext.as_str() {
        ".html" => Html::from(contents).into_response(),
        ".css" => return css_response_builder(contents),
        _ => return Redirect::to("/index.html").into_response(),
    }
}

fn css_response_builder(contents: Vec<u8>) -> Response {
    let mut resp = Body::from(contents).into_response();
    resp.headers_mut()
        .insert(header::CONTENT_TYPE, HeaderValue::from_static("text/css"));
    return resp;
}
