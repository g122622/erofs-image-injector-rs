//! Strategy API handlers
//!
//! HTTP handlers for strategy template management.

use axum::{
    extract::{Multipart, Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

use crate::api::ApiState;
use crate::strategy::StrategyError;
use crate::strategy_types::{CreateStrategyRequest, StrategyTemplate, UpdateStrategyRequest};

/// List all strategy templates
pub async fn list_strategies(
    State(state): State<ApiState>,
) -> Result<Json<Vec<StrategyTemplate>>, (StatusCode, Json<ErrorResponse>)> {
    debug!("Listing all strategy templates");

    let templates = state.strategy_storage.list().await;
    Ok(Json(templates))
}

/// Get a single strategy template
pub async fn get_strategy(
    State(state): State<ApiState>,
    Path(id): Path<i64>,
) -> Result<Json<StrategyTemplate>, (StatusCode, Json<ErrorResponse>)> {
    debug!("Getting strategy template {}", id);

    state.strategy_storage.get(id).await
        .ok_or_else(|| {
            (StatusCode::NOT_FOUND, Json(ErrorResponse::from("Strategy template not found")))
        })
        .map(Json)
}

/// Create a new strategy template
pub async fn create_strategy(
    State(state): State<ApiState>,
    Json(request): Json<CreateStrategyRequest>,
) -> Result<Json<StrategyTemplate>, (StatusCode, Json<ErrorResponse>)> {
    info!("Creating strategy template: {}", request.name);

    state.strategy_storage.create(request).await
        .map_err(|e| {
            let status = match &e {
                StrategyError::Invalid(_) => StatusCode::BAD_REQUEST,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            };
            (status, Json(ErrorResponse::from(e.to_string())))
        })
        .map(Json)
}

/// Update a strategy template
pub async fn update_strategy(
    State(state): State<ApiState>,
    Path(id): Path<i64>,
    Json(request): Json<UpdateStrategyRequest>,
) -> Result<Json<StrategyTemplate>, (StatusCode, Json<ErrorResponse>)> {
    info!("Updating strategy template {}", id);

    state.strategy_storage.update(id, request).await
        .map_err(|e| {
            let status = match &e {
                StrategyError::NotFound(_) => StatusCode::NOT_FOUND,
                StrategyError::CannotModifyBuiltin(_) => StatusCode::FORBIDDEN,
                StrategyError::Invalid(_) => StatusCode::BAD_REQUEST,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            };
            (status, Json(ErrorResponse::from(e.to_string())))
        })
        .map(Json)
}

/// Delete a strategy template
pub async fn delete_strategy(
    State(state): State<ApiState>,
    Path(id): Path<i64>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    info!("Deleting strategy template {}", id);

    state.strategy_storage.delete(id).await
        .map_err(|e| {
            let status = match &e {
                StrategyError::NotFound(_) => StatusCode::NOT_FOUND,
                StrategyError::CannotModifyBuiltin(_) => StatusCode::FORBIDDEN,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            };
            (status, Json(ErrorResponse::from(e.to_string())))
        })?;

    Ok(StatusCode::NO_CONTENT)
}

/// Duplicate a strategy template
pub async fn duplicate_strategy(
    State(state): State<ApiState>,
    Path(id): Path<i64>,
    Json(request): Json<DuplicateRequest>,
) -> Result<Json<StrategyTemplate>, (StatusCode, Json<ErrorResponse>)> {
    info!("Duplicating strategy template {}", id);

    state.strategy_storage.duplicate(id, request.name).await
        .map_err(|e| {
            let status = match &e {
                StrategyError::NotFound(_) => StatusCode::NOT_FOUND,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            };
            (status, Json(ErrorResponse::from(e.to_string())))
        })
        .map(Json)
}

/// Export a strategy template as TOML
pub async fn export_strategy(
    State(state): State<ApiState>,
    Path(id): Path<i64>,
) -> Result<Json<ExportResponse>, (StatusCode, Json<ErrorResponse>)> {
    debug!("Exporting strategy template {}", id);

    let toml_content = state.strategy_storage.export(id).await
        .map_err(|e| {
            let status = match &e {
                StrategyError::NotFound(_) => StatusCode::NOT_FOUND,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            };
            (status, Json(ErrorResponse::from(e.to_string())))
        })?;

    Ok(Json(ExportResponse {
        format: "toml".to_string(),
        content: toml_content,
    }))
}

/// Import a strategy template from TOML
pub async fn import_strategy(
    State(state): State<ApiState>,
    Json(request): Json<ImportRequest>,
) -> Result<Json<StrategyTemplate>, (StatusCode, Json<ErrorResponse>)> {
    info!("Importing strategy template");

    state.strategy_storage.import(&request.content).await
        .map_err(|e| {
            let status = match &e {
                StrategyError::Invalid(_) => StatusCode::BAD_REQUEST,
                StrategyError::TomlDeserialize(_) => StatusCode::BAD_REQUEST,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            };
            (status, Json(ErrorResponse::from(e.to_string())))
        })
        .map(Json)
}

/// Import a strategy template from uploaded file
pub async fn import_strategy_file(
    State(state): State<ApiState>,
    mut multipart: Multipart,
) -> Result<Json<StrategyTemplate>, (StatusCode, Json<ErrorResponse>)> {
    info!("Importing strategy template from file");

    while let Some(field) = multipart.next_field().await.ok().flatten() {
        if field.name() == Some("file") {
            let content = field.text().await.map_err(|e| {
                (StatusCode::BAD_REQUEST, Json(ErrorResponse::from(e.to_string())))
            })?;

            return state.strategy_storage.import(&content).await
                .map_err(|e| {
                    let status = match &e {
                        StrategyError::Invalid(_) => StatusCode::BAD_REQUEST,
                        StrategyError::TomlDeserialize(_) => StatusCode::BAD_REQUEST,
                        _ => StatusCode::INTERNAL_SERVER_ERROR,
                    };
                    (status, Json(ErrorResponse::from(e.to_string())))
                })
                .map(Json);
        }
    }

    Err((StatusCode::BAD_REQUEST, Json(ErrorResponse::from("No file uploaded"))))
}

/// Duplicate request
#[derive(Debug, Clone, Deserialize)]
pub struct DuplicateRequest {
    /// New name for the duplicated template
    #[serde(default)]
    pub name: Option<String>,
}

/// Export response
#[derive(Debug, Clone, Serialize)]
pub struct ExportResponse {
    /// Export format
    pub format: String,
    /// Exported content
    pub content: String,
}

/// Import request
#[derive(Debug, Clone, Deserialize)]
pub struct ImportRequest {
    /// TOML content to import
    pub content: String,
}

/// Error response
#[derive(Debug, Clone, Serialize)]
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
