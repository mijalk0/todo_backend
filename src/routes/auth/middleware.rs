use crate::{Claims, User, JWT_SECRET};
use axum::{
    extract::Request,
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::{IntoResponse, Result},
    Extension,
};
use axum_extra::{
    extract::CookieJar,
    headers::{authorization::Bearer, Authorization, HeaderMapExt},
};
use jsonwebtoken::{DecodingKey, Validation};
use std::collections::HashSet;

pub async fn middleware(
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
