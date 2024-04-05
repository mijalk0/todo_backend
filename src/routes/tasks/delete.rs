use crate::{Task, User};
use axum::{extract::Path, http::StatusCode, Extension};

pub async fn delete(
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
