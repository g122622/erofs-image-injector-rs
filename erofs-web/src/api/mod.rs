//! REST API handlers

mod tasks;
mod crashes;

use axum::{
    routing::{get, post, delete, Router},
    Json,
};
use serde::{Deserialize, Serialize};
use tower_http::cors::{Any, CorsLayer};

use crate::db::Database;
use crate::task_manager::TaskManager;

pub use tasks::*;
pub use crashes::*;

/// API state shared by handlers
#[derive(Debug, Clone)]
pub struct ApiState {
    pub db: Database,
    pub task_manager: TaskManager,
}

/// Create the API router (without state - state is applied later)
pub fn create_router() -> Router<ApiState> {
    Router::new()
        // Task endpoints
        .route("/api/tasks", get(list_tasks).post(create_task))
        .route("/api/tasks/:id", get(get_task).delete(delete_task))
        .route("/api/tasks/:id/start", post(start_task))
        .route("/api/tasks/:id/stop", post(stop_task))
        .route("/api/tasks/:id/pause", post(pause_task))
        .route("/api/tasks/:id/resume", post(resume_task))
        // Crash endpoints
        .route("/api/crashes", get(list_crashes))
        .route("/api/crashes/:id", get(get_crash))
        .route("/api/crashes/:id/image", get(get_crash_image))
        .route("/api/crashes/:id/repro", get(get_crash_repro))
        // Stats endpoint
        .route("/api/stats", get(get_stats))
        // Health check
        .route("/api/health", get(health_check))
        // CORS
        .layer(CorsLayer::new().allow_origin(Any).allow_methods(Any).allow_headers(Any))
}

/// Health check response
#[derive(Debug, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
}

/// Health check endpoint
pub async fn health_check() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

/// Error response
#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: String,
}

impl From<String> for ErrorResponse {
    fn from(error: String) -> Self {
        Self { error }
    }
}

impl From<&str> for ErrorResponse {
    fn from(error: &str) -> Self {
        Self { error: error.to_string() }
    }
}
