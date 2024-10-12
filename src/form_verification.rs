use std::collections::HashMap;

use axum::response::Response;

const TLDS: HashMap<String, i32> = HashMap::from(include_str!("tlds-alpha-by-domain.txt"));

fn is_valid_url(input: String) -> Response {}
