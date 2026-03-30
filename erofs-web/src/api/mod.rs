//! REST API handlers

mod tasks;
mod crashes;
mod strategies;
mod seeds;

use axum::{
    routing::{get, post, put, delete, Router},
    Json,
};
use serde::{Deserialize, Serialize};
use tower_http::cors::{Any, CorsLayer};

use crate::db::Database;
use crate::task_manager::TaskManager;
use crate::strategy::StrategyStorage;

pub use tasks::*;
pub use crashes::*;
pub use strategies::*;
pub use seeds::*;

/// API state shared by handlers
#[derive(Debug, Clone)]
pub struct ApiState {
    pub db: Database,
    pub task_manager: TaskManager,
    pub strategy_storage: StrategyStorage,
}

/// Create the API router (without state - state is applied later)
pub fn create_router() -> Router<ApiState> {
    Router::new()
        // Task endpoints
        .route("/api/tasks", get(list_tasks).post(create_task))
        .route("/api/tasks/batch/stop", post(batch_stop_tasks))
        .route("/api/tasks/batch/delete", post(batch_delete_tasks))
        .route("/api/tasks/:id", get(get_task).delete(delete_task))
        .route("/api/tasks/:id/start", post(start_task))
        .route("/api/tasks/:id/stop", post(stop_task))
        .route("/api/tasks/:id/pause", post(pause_task))
        .route("/api/tasks/:id/resume", post(resume_task))
        // Crash endpoints
        .route("/api/crashes", get(list_crashes))
        .route("/api/crashes/:id", get(get_crash))
        .route("/api/crashes/:id/image", get(get_crash_image))
        .route("/api/crashes/:id/log", get(get_crash_log))
        .route("/api/crashes/:id/repro", get(get_crash_repro))
        // Strategy endpoints
        .route("/api/strategies", get(list_strategies).post(create_strategy))
        .route("/api/strategies/:id", get(get_strategy).put(update_strategy).delete(delete_strategy))
        .route("/api/strategies/:id/duplicate", post(duplicate_strategy))
        .route("/api/strategies/:id/export", get(export_strategy))
        .route("/api/strategies/import", post(import_strategy))
        .route("/api/strategies/import-file", post(import_strategy_file))
        // Seed endpoints
        .route("/api/seeds", get(list_seeds))
        .route("/api/seeds/generate", post(generate_seeds))
        .route("/api/seeds/upload", post(upload_seed))
        .route("/api/seeds/templates", get(list_templates))
        .route("/api/seeds/templates/:id", get(get_template))
        .route("/api/seeds/:id", get(get_seed).delete(delete_seed))
        .route("/api/seeds/:id/download", get(download_seed))
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
