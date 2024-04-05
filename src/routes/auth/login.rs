use crate::{Claims, User, JWT_SECRET};
use argon2::{password_hash::PasswordHash, Argon2, PasswordVerifier};
use axum::{
    http::StatusCode,
    response::{IntoResponse, Result},
    Extension, Json,
};
use axum_extra::extract::{
    cookie::{Cookie, SameSite},
    CookieJar,
};
use jsonwebtoken::{EncodingKey, Header};
use serde::Deserialize;
use serde_json::json;
use validator::Validate;

#[derive(Deserialize, Validate)]
pub struct UserLoginRequest {
    username: String,
    password: String,
    #[serde(rename = "rememberMe")]
    remember_me: bool,
}

pub async fn login(
    Extension(pool): Extension<sqlx::Pool<sqlx::Postgres>>,
    jar: CookieJar,
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

    let mut cookie = Cookie::build(("token", token.clone()))
        .path("/")
        .same_site(SameSite::Lax)
        .secure(true)
        .build();

    if login_user.remember_me {
        cookie.make_permanent();
    } else {
        cookie.set_expires(None);
    }

    Ok((jar.add(cookie), Json(json!({ "token": token }))))
}
