use crate::{Task, User};
use axum::{extract::Path, http::StatusCode, response::Result, Extension, Json};

pub async fn read(
    Path(id): Path<i32>,
    Extension(pool): Extension<sqlx::Pool<sqlx::Postgres>>,
    Extension(user): Extension<User>,
) -> Result<Json<Task>, StatusCode> {
    if let Some(task) =
        sqlx::query_as::<_, Task>("SELECT * FROM tasks WHERE id = $1 AND user_id = $2")
            .bind(id)
            .bind(user.id)
            .fetch_optional(&pool)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    {
        Ok(Json(task))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}
