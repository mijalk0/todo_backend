use crate::User;
use argon2::{password_hash::SaltString, Argon2, PasswordHasher};
use axum::{http::StatusCode, response::Result, Extension, Json};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Deserialize, Validate)]
pub struct UserRegisterRequest {
    #[validate(length(min = 1, max = 64))]
    username: String,
    #[validate(length(min = 1, max = 64))]
    password: String,
}

#[derive(Serialize, Validate)]
pub struct UserRegisterResponse {
    id: i32,
    username: String,
}

pub async fn register(
    Extension(pool): Extension<sqlx::Pool<sqlx::Postgres>>,
    Json(user): Json<UserRegisterRequest>,
) -> Result<Json<UserRegisterResponse>, StatusCode> {
    if user.validate().is_err() {
        return Err(StatusCode::BAD_REQUEST);
    }

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
