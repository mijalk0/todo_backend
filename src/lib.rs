use axum::{
    extract::{Path, Query},
    http::{header::USER_AGENT, HeaderMap, HeaderName, HeaderValue},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};

pub async fn run() {
    // build our application with a single route
    let app = Router::new()
        .route("/", get(hello_world))
        .route("/extract_body", post(extract_body))
        .route("/extract_json", post(extract_json))
        .route("/extract_path_variable/:id", get(extract_path_variable))
        .route("/extract_query_params", get(extract_query_params))
        .route("/extract_standard_header", get(extract_standard_header))
        .route("/extract_custom_header", get(extract_custom_header));

    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn hello_world() -> String {
    "Hello, World!".into()
}

async fn extract_body(body: String) -> String {
    body
}

#[derive(Serialize, Deserialize)]
struct JsonRequest {
    message: String,
}

#[derive(Serialize, Deserialize)]
struct JsonResponse {
    message: String,
    added: String,
}

async fn extract_json(Json(json): Json<JsonRequest>) -> Json<JsonResponse> {
    Json(JsonResponse {
        message: json.message,
        added: "Added response".into(),
    })
}

async fn extract_path_variable(Path(id): Path<i32>) -> String {
    id.to_string()
}

#[derive(Serialize, Deserialize)]
struct QueryParams {
    message: String,
    id: i32,
}

async fn extract_query_params(Query(query): Query<QueryParams>) -> Json<QueryParams> {
    Json(query)
}

async fn extract_standard_header(headers: HeaderMap) -> String {
    headers
        .get(USER_AGENT)
        .map(HeaderValue::to_str)
        .unwrap_or(Ok(""))
        .unwrap()
        .into()
}

const CUSTOM_HEADER: HeaderName = HeaderName::from_static("x-custom-header");

async fn extract_custom_header(headers: HeaderMap) -> String {
    headers
        .get(CUSTOM_HEADER)
        .map(HeaderValue::to_str)
        .unwrap_or(Ok(""))
        .unwrap()
        .into()
}
