use axum::response::{IntoResponse, Response};

pub async fn is_valid_url(input: String) -> Response {
    let body = r###"
				<div id="input-box-div">
					<input type="text" name="url" id="long_url_input" hx-post="/verify/url" hx-target="#input-box-div"
						placeholder="https://example.com">
				</div>
        "###;
}
