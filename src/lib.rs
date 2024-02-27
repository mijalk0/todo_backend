use axum::{
    async_trait,
    extract::{FromRequest, Path, Query, Request},
    http::{header::USER_AGENT, HeaderMap, HeaderName, HeaderValue, Method, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response, Result},
    routing::{get, post},
    Extension, Json, Router,
};
use serde::{Deserialize, Serialize};
use tower_http::cors::{Any, CorsLayer};

pub async fn run() {
    let cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST])
        .allow_origin(Any);

    // build our application with a single route
    let app = Router::new()
        .route("/", get(hello_world))
        .route("/extract_body", post(extract_body))
        .route("/extract_json", post(extract_json))
        .route("/extract_path_variable/:id", get(extract_path_variable))
        .route("/extract_query_params", get(extract_query_params))
        .route("/extract_standard_header", get(extract_standard_header))
        .route("/extract_custom_header", get(extract_custom_header))
        .route("/extract_shared_data", get(extract_shared_data))
        .layer(cors)
        .layer(Extension(String::from("Shared Data")))
        .route(
            "/read_custom_middleware_data",
            get(read_custom_middleware_data),
        )
        .layer(middleware::from_fn(write_custom_middleware_data))
        .route("/error", get(error))
        .route("/code", get(code))
        .route("/validate_json", post(validate_json));

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

async fn extract_shared_data(Extension(shared_data): Extension<String>) -> String {
    shared_data
}

async fn read_custom_middleware_data(
    Extension(custom_middleware_data): Extension<CustomMiddlewareData>,
) -> String {
    custom_middleware_data.0
}

#[derive(Clone)]
struct CustomMiddlewareData(String);

async fn write_custom_middleware_data(mut request: Request, next: Next) -> Response {
    let extensions = request.extensions_mut();
    extensions.insert(CustomMiddlewareData("Custom".into()));
    let response = next.run(request).await;
    response
}

async fn error() -> Result<(), StatusCode> {
    Err(StatusCode::UNAUTHORIZED)
}

async fn code() -> Response {
    (StatusCode::NO_CONTENT, ()).into_response()
}

#[derive(Deserialize)]
struct ValidatedJson {
    larger: i32,
    smaller: i32,
}

#[async_trait]
impl<S> FromRequest<S> for ValidatedJson
where
    Json<ValidatedJson>: FromRequest<S>,
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let Json(maybe_validated_json) = Json::<ValidatedJson>::from_request(req, state)
            .await
            .map_err(IntoResponse::into_response)?;

        if maybe_validated_json.larger > maybe_validated_json.smaller {
            Ok(maybe_validated_json)
        } else {
            return Err((StatusCode::BAD_REQUEST, "").into_response());
        }
    }
}

async fn validate_json(validated_json: ValidatedJson) -> String {
    format!("{} > {}", validated_json.larger, validated_json.smaller)
}
