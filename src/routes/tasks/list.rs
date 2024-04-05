use crate::{Task, User};
use axum::{
    http::StatusCode,
    response::{IntoResponse, Result},
    Extension, Json,
};

pub async fn list(
    Extension(pool): Extension<sqlx::Pool<sqlx::Postgres>>,
    Extension(user): Extension<User>,
) -> Result<impl IntoResponse, StatusCode> {
    let tasks: Vec<Task> = sqlx::query_as::<_, Task>(
        "SELECT * FROM tasks WHERE user_id = $1 ORDER BY created_at DESC",
    )
    .bind(&user.id)
    .fetch_all(&pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(tasks))
}
