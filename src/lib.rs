use argon2::{
    password_hash::{PasswordHash, SaltString},
    Argon2, PasswordHasher, PasswordVerifier,
};
use axum::{
    body::Body,
    extract::{Path, Request},
    http::{header, HeaderMap, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response, Result},
    routing::{delete, get, patch, post},
    Extension, Json, Router,
};
use axum_extra::{
    extract::{
        cookie::{Cookie, SameSite},
        CookieJar,
    },
    headers::{authorization::Bearer, Authorization, HeaderMapExt},
};
use chrono::{DateTime, Utc};
use dotenvy::dotenv;
use dotenvy_macro::dotenv;
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::prelude::FromRow;
use std::{collections::HashSet, error::Error};
use tower_http::cors::{Any, CorsLayer};
use validator::Validate;

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

    let cors = CorsLayer::new().allow_origin(Any);

    let app = Router::new()
        // Auth
        .route("/auth/users/:id", delete(delete_user))
        .route("/auth/register", post(register_user))
        .route("/auth/login", post(login_user))
        .route(
            "/auth/logout",
            get(logout_user).route_layer(middleware::from_fn(authentication)),
        )
        .route(
            "/tasks",
            get(list_tasks).route_layer(middleware::from_fn(authentication)),
        )
        // Create
        .route(
            "/tasks",
            post(create_task).route_layer(middleware::from_fn(authentication)),
        )
        // Read
        .route(
            "/tasks/:id",
            get(read_task).route_layer(middleware::from_fn(authentication)),
        )
        // Update
        .route(
            "/tasks/:id",
            patch(update_task).route_layer(middleware::from_fn(authentication)),
        )
        // Delete
        .route(
            "/tasks/:id",
            delete(delete_task).route_layer(middleware::from_fn(authentication)),
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

#[derive(Deserialize, Validate)]
struct UserRegisterRequest {
    #[validate(length(max = 64))]
    username: String,
    #[validate(length(max = 64))]
    password: String,
}

#[derive(Serialize, Validate)]
struct UserRegisterResponse {
    id: i32,
    username: String,
}

async fn register_user(
    Extension(pool): Extension<sqlx::Pool<sqlx::Postgres>>,
    Json(user): Json<UserRegisterRequest>,
) -> Result<Json<UserRegisterResponse>, StatusCode> {
    let exists =
        sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT * FROM users WHERE username = $1)")
            .bind(&user.username)
            .fetch_one(&pool)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if exists {
        return Err(StatusCode::CONFLICT);
    }

    let salt = SaltString::generate(&mut OsRng);
    let hashed_password = Argon2::default()
        .hash_password(user.password.as_bytes(), &salt)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .to_string();

    let user = sqlx::query_as::<_, User>(
        "INSERT INTO users (username, password) VALUES ($1, $2) RETURNING *",
    )
    .bind(&user.username)
    .bind(&hashed_password)
    .fetch_one(&pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(UserRegisterResponse {
        id: user.id,
        username: user.username,
    }))
}

#[derive(Deserialize, Validate)]
struct UserLoginRequest {
    username: String,
    password: String,
}

async fn login_user(
    Extension(pool): Extension<sqlx::Pool<sqlx::Postgres>>,
    Json(login_user): Json<UserLoginRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE username = $1")
        .bind(&login_user.username)
        .fetch_optional(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::BAD_REQUEST)?;

    if !PasswordHash::new(&user.password).is_ok_and(|parsed_hash| {
        Argon2::default()
            .verify_password(login_user.password.as_bytes(), &parsed_hash)
            .is_ok()
    }) {
        return Err(StatusCode::BAD_REQUEST);
    }

    let claims = Claims {
        sub: user.id.to_string(),
    };

    let token = jsonwebtoken::encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(JWT_SECRET),
    )
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let cookie = Cookie::build(("token", token.clone()))
        .path("/")
        .same_site(SameSite::Lax)
        .http_only(true);

    let mut response = Response::new(json!({ "token": token }).to_string());
    response.headers_mut().insert(
        header::SET_COOKIE,
        cookie
            .to_string()
            .parse()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    );

    Ok(response)
}

async fn logout_user() -> Result<impl IntoResponse, StatusCode> {
    let cookie = Cookie::build(("token", ""))
        .path("/")
        .same_site(SameSite::Lax)
        .http_only(true);

    let mut response = Response::<Body>::default();
    response.headers_mut().insert(
        header::SET_COOKIE,
        cookie
            .to_string()
            .parse()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    );

    Ok(response)
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

pub async fn authentication(
    cookie_jar: CookieJar,
    headers: HeaderMap,
    Extension(pool): Extension<sqlx::Pool<sqlx::Postgres>>,
    mut req: Request,
    next: Next,
) -> Result<impl IntoResponse, StatusCode> {
    let token = cookie_jar
        .get("token")
        .map(|cookie| cookie.value().to_string())
        .or_else(|| {
            headers
                .typed_get::<Authorization<Bearer>>()
                .map(|auth| auth.token().into())
        })
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let mut validation = Validation::default();
    validation.required_spec_claims = HashSet::default();
    validation.validate_exp = false;

    let claims =
        jsonwebtoken::decode::<Claims>(&token, &DecodingKey::from_secret(JWT_SECRET), &validation)
            .map_err(|_| StatusCode::UNAUTHORIZED)?
            .claims;

    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
        .bind(
            &claims
                .sub
                .parse::<i32>()
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
        )
        .fetch_optional(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::UNAUTHORIZED)?;

    req.extensions_mut().insert(user);

    Ok(next.run(req).await)
}

async fn list_tasks(
    Extension(pool): Extension<sqlx::Pool<sqlx::Postgres>>,
    Extension(user): Extension<User>,
) -> Result<impl IntoResponse, StatusCode> {
    let tasks: Vec<Task> = sqlx::query_as::<_, Task>("SELECT * FROM tasks WHERE user_id = $1")
        .bind(&user.id)
        .fetch_all(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(tasks))
}

#[derive(Deserialize, Validate)]
struct CreateTaskRequest {
    title: String,
    description: Option<String>,
    completed: bool,
}

async fn create_task(
    Extension(pool): Extension<sqlx::Pool<sqlx::Postgres>>,
    Extension(user): Extension<User>,
    Json(task): Json<CreateTaskRequest>,
) -> Result<Json<Task>, StatusCode> {
    let task: Task = sqlx::query_as::<_, Task>(
        "INSERT INTO tasks (user_id, title, description, completed) VALUES ($1, $2, $3, $4) RETURNING *",
    )
    .bind(&user.id)
    .bind(&task.title)
    .bind(&task.description)
    .bind(&task.completed)
    .fetch_one(&pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(task))
}

async fn read_task(
    Path(id): Path<i32>,
    Extension(pool): Extension<sqlx::Pool<sqlx::Postgres>>,
    Extension(user): Extension<User>,
) -> Result<Json<Task>, StatusCode> {
    if let Some(task) =
        sqlx::query_as::<_, Task>("SELECT * FROM tasks WHERE id = $1 AND user_id = $2")
            .bind(&id)
            .bind(&user.id)
            .fetch_optional(&pool)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    {
        Ok(Json(task))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

#[derive(Deserialize)]
struct UpdateTaskRequest {
    title: Option<String>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "serde_with::rust::double_option"
    )]
    description: Option<Option<String>>,
    completed: Option<bool>,
}

async fn update_task(
    Path(id): Path<i32>,
    Extension(pool): Extension<sqlx::Pool<sqlx::Postgres>>,
    Extension(user): Extension<User>,
    Json(task_updated): Json<UpdateTaskRequest>,
) -> Result<Json<Task>, StatusCode> {
    let mut txn = pool
        .begin()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut task = sqlx::query_as::<_, Task>("SELECT * FROM tasks WHERE id = $1 AND user_id = $2")
        .bind(&id)
        .bind(&user.id)
        .fetch_one(&mut *txn)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if let Some(title) = task_updated.title {
        task.title = title;
    }
    if let Some(description) = task_updated.description {
        task.description = description;
    }
    if let Some(completed) = task_updated.completed {
        task.completed = completed;
    }

    let task = sqlx::query_as::<_, Task>(
        "UPDATE tasks
         SET title = $1, description = $2, completed = $3
         WHERE id = $4 AND user_id = $5
         RETURNING *",
    )
    .bind(&task.title)
    .bind(&task.description)
    .bind(&task.completed)
    .bind(&id)
    .bind(&user.id)
    .fetch_one(&mut *txn)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    txn.commit()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(task))
}

async fn delete_task(
    Path(id): Path<i32>,
    Extension(pool): Extension<sqlx::Pool<sqlx::Postgres>>,
    Extension(user): Extension<User>,
) -> StatusCode {
    match sqlx::query_as::<_, Task>("DELETE FROM tasks WHERE id = $1 AND user_id = $2 RETURNING *")
        .bind(&id)
        .bind(&user.id)
        .fetch_optional(&pool)
        .await
    {
        Ok(Some(_)) => StatusCode::NO_CONTENT,
        Ok(None) => StatusCode::NOT_FOUND,
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}
