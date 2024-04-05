use axum::{
    body::Body,
    http::{header, StatusCode},
    response::{IntoResponse, Response, Result},
};
use axum_extra::extract::cookie::{Cookie, SameSite};

pub async fn logout() -> Result<impl IntoResponse, StatusCode> {
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
