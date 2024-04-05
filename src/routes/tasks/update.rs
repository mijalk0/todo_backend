use crate::{Task, User};
use axum::{extract::Path, http::StatusCode, response::Result, Extension, Json};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct UpdateTaskRequest {
    title: Option<String>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "serde_with::rust::double_option"
    )]
    description: Option<Option<String>>,
    completed: Option<bool>,
}

pub async fn update(
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
        .bind(id)
        .bind(user.id)
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
    .bind(task.completed)
    .bind(id)
    .bind(user.id)
    .fetch_one(&mut *txn)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    txn.commit()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(task))
}
