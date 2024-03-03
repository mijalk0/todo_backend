use axum::{
    extract::Path,
    http::StatusCode,
    response::{IntoResponse, Response, Result},
    routing::{delete, get, patch, put},
    Extension, Json, Router,
};
use dotenvy::dotenv;
use dotenvy_macro::dotenv;
use serde::{Deserialize, Serialize};
use sqlx::prelude::FromRow;
use std::error::Error;
use tower_http::cors::{Any, CorsLayer};
use validator::Validate;

#[derive(FromRow, Serialize, Deserialize)]
struct User {
    id: i32,
    username: String,
    password: String,
    token: Option<String>,
}

const DATABASE_URL: &'static str = dotenv!("DATABASE_URL");

pub async fn run() -> Result<(), Box<dyn Error>> {
    dotenv().ok();
    let pool = sqlx::postgres::PgPool::connect(DATABASE_URL).await?;
    sqlx::migrate!("./migrations").run(&pool).await?;

    let cors = CorsLayer::new().allow_origin(Any);

    let app = Router::new()
        .route("/users", put(create_user))
        .route("/users/:id", get(read_user))
        .route("/users/:id", patch(update_user))
        .route("/users/:id", delete(delete_user))
        .layer(cors)
        .layer(Extension(pool));

    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();
    axum::serve(listener, app).await.unwrap();
    Ok(())
}

#[derive(Deserialize, Validate)]
struct UserRequested {
    #[validate(length(max = 64))]
    username: String,
    #[validate(length(max = 64))]
    password: String,
    token: Option<String>,
}

async fn create_user(
    Extension(pool): Extension<sqlx::Pool<sqlx::Postgres>>,
    Json(user): Json<UserRequested>,
) -> Response {
    if let Ok(user) = sqlx::query_as::<_, User>(
        "INSERT INTO users (username, password, token) VALUES ($1, $2, $3) RETURNING *",
    )
    .bind(&user.username)
    .bind(&user.password)
    .bind(&user.token)
    .fetch_one(&pool)
    .await
    {
        (StatusCode::CREATED, Json(user)).into_response()
    } else {
        (StatusCode::INTERNAL_SERVER_ERROR, ()).into_response()
    }
}

async fn read_user(
    Path(id): Path<i32>,
    Extension(pool): Extension<sqlx::Pool<sqlx::Postgres>>,
) -> Result<Json<User>, StatusCode> {
    sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
        .bind(&id)
        .fetch_one(&pool)
        .await
        .map(|user| Json(user))
        .map_err(|_| StatusCode::NOT_FOUND)
}

async fn update_user(
    Path(id): Path<i32>,
    Extension(pool): Extension<sqlx::Pool<sqlx::Postgres>>,
    Json(user): Json<UserRequested>,
) -> Result<Json<User>, StatusCode> {
    sqlx::query_as::<_, User>(
        "UPDATE users SET username = $2, password = $3, token = $4 WHERE id = $1 RETURNING *",
    )
    .bind(&id)
    .bind(&user.username)
    .bind(&user.password)
    .bind(&user.token)
    .fetch_one(&pool)
    .await
    .map(|user| Json(user))
    .map_err(|_| StatusCode::NOT_FOUND)
}

async fn delete_user(
    Path(id): Path<i32>,
    Extension(pool): Extension<sqlx::Pool<sqlx::Postgres>>,
) -> StatusCode {
    if sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(&id)
        .execute(&pool)
        .await
        .is_ok()
    {
        StatusCode::NO_CONTENT
    } else {
        StatusCode::NOT_FOUND
    }
}
