use axum::{
    middleware::{self},
    response::Result,
    routing::{delete, get, patch, post},
    Extension, Router,
};
use chrono::{DateTime, Utc};
use dotenvy::dotenv;
use dotenvy_macro::dotenv;
use serde::{Deserialize, Serialize};
use sqlx::prelude::FromRow;
use std::error::Error;
use tower_http::cors::CorsLayer;

mod routes;
use routes::*;

#[derive(Clone, FromRow, Serialize, Deserialize)]
struct User {
    id: i32,
    username: String,
    password: String,
    token: Option<String>,
}

#[derive(Clone, FromRow, Serialize, Deserialize)]
struct Task {
    id: i32,
    user_id: i32,
    title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    completed: bool,
    #[serde(rename = "createdAt")]
    created_at: DateTime<Utc>,
    #[serde(rename = "updatedAt")]
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: String,
}

const DATABASE_URL: &'static str = dotenv!("DATABASE_URL");
const JWT_SECRET: &'static [u8] = dotenv!("JWT_SECRET").as_bytes();

pub async fn run() -> Result<(), Box<dyn Error>> {
    dotenv().ok();
    let pool = sqlx::postgres::PgPool::connect(DATABASE_URL).await?;
    sqlx::migrate!("./migrations").run(&pool).await?;

    let cors = CorsLayer::very_permissive();

    let app = Router::new()
        // Auth
        .route("/auth/register", post(auth::register))
        .route("/auth/login", post(auth::login))
        .route(
            "/auth/logout",
            get(auth::logout).route_layer(middleware::from_fn(auth::middleware)),
        )
        .route(
            "/tasks",
            get(tasks::list).route_layer(middleware::from_fn(auth::middleware)),
        )
        // Create
        .route(
            "/tasks",
            post(tasks::create).route_layer(middleware::from_fn(auth::middleware)),
        )
        // Read
        .route(
            "/tasks/:id",
            get(tasks::read).route_layer(middleware::from_fn(auth::middleware)),
        )
        // Update
        .route(
            "/tasks/:id",
            patch(tasks::update).route_layer(middleware::from_fn(auth::middleware)),
        )
        // Delete
        .route(
            "/tasks/:id",
            delete(tasks::delete).route_layer(middleware::from_fn(auth::middleware)),
        )
        .layer(Extension(pool))
        .layer(cors);

    // Run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();
    axum::serve(listener, app).await.unwrap();
    Ok(())
}
