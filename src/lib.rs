use axum::{
    extract::Path,
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
        .route("/extract_path_variable/:id", get(extract_path_variable));

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
