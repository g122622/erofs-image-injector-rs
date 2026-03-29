//! Task API handlers

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use tracing::{debug, info};

use crate::api::{ApiState, ErrorResponse};
use crate::types::*;

/// List all tasks
pub async fn list_tasks(
    State(state): State<ApiState>,
) -> Result<Json<Vec<Task>>, (StatusCode, Json<ErrorResponse>)> {
    debug!("Listing all tasks");

    let tasks = state.task_manager.list_tasks().await
        .map_err(|e| {
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse::from(e)))
        })?;

    Ok(Json(tasks))
}

/// Get a single task
pub async fn get_task(
    State(state): State<ApiState>,
    Path(id): Path<i64>,
) -> Result<Json<Task>, (StatusCode, Json<ErrorResponse>)> {
    debug!("Getting task {}", id);

    let task = state.task_manager.get_task(id).await
        .map_err(|e| {
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse::from(e)))
        })?
        .ok_or_else(|| {
            (StatusCode::NOT_FOUND, Json(ErrorResponse::from("Task not found")))
        })?;

    Ok(Json(task))
}

/// Create a new task
pub async fn create_task(
    State(state): State<ApiState>,
    Json(request): Json<CreateTaskRequest>,
) -> Result<Json<Task>, (StatusCode, Json<ErrorResponse>)> {
    info!("Creating task: {:?}", request.config);

    let task_id = state.task_manager.create_task(request.config).await
        .map_err(|e| {
            let status = if e.contains("not found") {
                StatusCode::BAD_REQUEST
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            };
            (status, Json(ErrorResponse::from(e)))
        })?;

    let task = state.task_manager.get_task(task_id).await
        .map_err(|e| {
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse::from(e)))
        })?
        .expect("Task should exist after creation");

    Ok(Json(task))
}

/// Start a task
pub async fn start_task(
    State(state): State<ApiState>,
    Path(id): Path<i64>,
) -> Result<Json<Task>, (StatusCode, Json<ErrorResponse>)> {
    info!("Starting task {}", id);

    state.task_manager.start_task(id).await
        .map_err(|e| {
            let status = if e.contains("not found") {
                StatusCode::NOT_FOUND
            } else if e.contains("not in a startable state") {
                StatusCode::BAD_REQUEST
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            };
            (status, Json(ErrorResponse::from(e)))
        })?;

    let task = state.task_manager.get_task(id).await
        .map_err(|e| {
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse::from(e)))
        })?
        .expect("Task should exist");

    Ok(Json(task))
}

/// Stop a task
pub async fn stop_task(
    State(state): State<ApiState>,
    Path(id): Path<i64>,
) -> Result<Json<Task>, (StatusCode, Json<ErrorResponse>)> {
    info!("Stopping task {}", id);

    state.task_manager.stop_task(id).await
        .map_err(|e| {
            let status = if e.contains("not found") {
                StatusCode::NOT_FOUND
            } else if e.contains("not running") {
                StatusCode::BAD_REQUEST
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            };
            (status, Json(ErrorResponse::from(e)))
        })?;

    let task = state.task_manager.get_task(id).await
        .map_err(|e| {
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse::from(e)))
        })?
        .expect("Task should exist");

    Ok(Json(task))
}

/// Pause a task
pub async fn pause_task(
    State(state): State<ApiState>,
    Path(id): Path<i64>,
) -> Result<Json<Task>, (StatusCode, Json<ErrorResponse>)> {
    info!("Pausing task {}", id);

    state.task_manager.pause_task(id).await
        .map_err(|e| {
            let status = if e.contains("not found") {
                StatusCode::NOT_FOUND
            } else if e.contains("not running") {
                StatusCode::BAD_REQUEST
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            };
            (status, Json(ErrorResponse::from(e)))
        })?;

    let task = state.task_manager.get_task(id).await
        .map_err(|e| {
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse::from(e)))
        })?
        .expect("Task should exist");

    Ok(Json(task))
}

/// Resume a paused task
pub async fn resume_task(
    State(state): State<ApiState>,
    Path(id): Path<i64>,
) -> Result<Json<Task>, (StatusCode, Json<ErrorResponse>)> {
    info!("Resuming task {}", id);

    state.task_manager.resume_task(id).await
        .map_err(|e| {
            let status = if e.contains("not found") {
                StatusCode::NOT_FOUND
            } else if e.contains("not paused") {
                StatusCode::BAD_REQUEST
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            };
            (status, Json(ErrorResponse::from(e)))
        })?;

    let task = state.task_manager.get_task(id).await
        .map_err(|e| {
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse::from(e)))
        })?
        .expect("Task should exist");

    Ok(Json(task))
}

/// Delete a task
pub async fn delete_task(
    State(state): State<ApiState>,
    Path(id): Path<i64>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    info!("Deleting task {}", id);

    state.task_manager.delete_task(id).await
        .map_err(|e| {
            let status = if e.contains("not found") {
                StatusCode::NOT_FOUND
            } else if e.contains("Cannot delete a running task") {
                StatusCode::BAD_REQUEST
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            };
            (status, Json(ErrorResponse::from(e)))
        })?;

    Ok(StatusCode::NO_CONTENT)
}

/// Get task statistics
pub async fn get_stats(
    State(state): State<ApiState>,
) -> Result<Json<TaskStats>, (StatusCode, Json<ErrorResponse>)> {
    debug!("Getting stats");

    let stats = state.task_manager.get_stats().await
        .map_err(|e| {
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse::from(e)))
        })?;

    Ok(Json(stats))
}
