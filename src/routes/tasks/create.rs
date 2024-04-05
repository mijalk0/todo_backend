use crate::{Task, User};
use axum::{http::StatusCode, response::Result, Extension, Json};
use serde::Deserialize;
use validator::Validate;

#[derive(Deserialize, Validate)]
pub struct CreateTaskRequest {
    #[validate(length(min = 1))]
    title: String,
    description: Option<String>,
}

pub async fn create(
    Extension(pool): Extension<sqlx::Pool<sqlx::Postgres>>,
    Extension(user): Extension<User>,
    Json(task): Json<CreateTaskRequest>,
) -> Result<Json<Task>, StatusCode> {
    if task.validate().is_err() {
        return Err(StatusCode::BAD_REQUEST);
    }

    let task: Task = sqlx::query_as::<_, Task>(
        "INSERT INTO tasks (user_id, title, description) VALUES ($1, $2, $3) RETURNING *",
    )
    .bind(user.id)
    .bind(&task.title)
    .bind(&task.description)
    .fetch_one(&pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(task))
}
