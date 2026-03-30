//! EROFS Web Console
//!
//! Web-based management console for EROFS Image Fuzzer.

// #![deny(missing_docs)]  // Temporarily disabled for faster development

pub mod api;
pub mod db;
pub mod seeds;
pub mod strategy;
pub mod strategy_types;
pub mod task_manager;
pub mod types;
pub mod ws;

use std::net::SocketAddr;
use std::path::PathBuf;

use axum::{
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use mime_guess::from_path;
use rust_embed::RustEmbed;
use tower_http::trace::TraceLayer;
use tracing::info;

use crate::api::ApiState;
use crate::db::Database;
use crate::strategy::StrategyStorage;
use crate::task_manager::TaskManager;

#[derive(RustEmbed)]
#[folder = "static/"]
struct WebAssets;

/// Web server configuration
#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// Server listen address
    pub addr: SocketAddr,
    /// Database path (None for in-memory)
    pub db_path: Option<PathBuf>,
    /// Maximum concurrent tasks
    pub max_concurrent_tasks: usize,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            addr: SocketAddr::from(([0, 0, 0, 0], 8080)),
            db_path: None,
            max_concurrent_tasks: 4,
        }
    }
}

/// Run the web server
pub async fn run_server(config: ServerConfig) -> Result<(), Box<dyn std::error::Error>> {
    info!("Starting EROFS Web Console on {}", config.addr);

    // Initialize database
    let db = match &config.db_path {
        Some(path) => Database::new(path)?,
        None => {
            info!("Using in-memory database");
            Database::in_memory()?
        }
    };

    // Initialize strategy storage
    let strategy_storage = StrategyStorage::with_default_path()?;
    strategy_storage.initialize().await?;

    // Create task manager
    let task_manager = if config.max_concurrent_tasks != 4 {
        TaskManager::with_concurrency(db.clone(), config.max_concurrent_tasks)
    } else {
        TaskManager::new(db.clone())
    };

    // Create API state
    let state = ApiState {
        db,
        task_manager,
        strategy_storage,
    };

    // Build router with state
    let app = create_app(state);

    // Start server
    let listener = tokio::net::TcpListener::bind(config.addr).await?;
    info!("Server listening on {}", config.addr);

    axum::serve(listener, app).await?;

    Ok(())
}

/// Create the application router
fn create_app(state: ApiState) -> Router {
    Router::new()
        // API routes
        .merge(api::create_router())
        // WebSocket
        .route("/ws", get(ws::ws_handler))
        // Static files + SPA fallback
        .fallback(get(serve_static))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

/// Serve embedded static files generated from `web-ui/dist`
async fn serve_static(uri: axum::http::Uri) -> Response {
    let raw_path = uri.path().trim_start_matches('/');
    let asset_path = if raw_path.is_empty() {
        "index.html"
    } else {
        raw_path
    };

    if let Some(asset) = WebAssets::get(asset_path) {
        return response_with_content_type(asset_path, asset.data.into_owned());
    }

    // SPA fallback for client-side routes
    if let Some(index) = WebAssets::get("index.html") {
        return response_with_content_type("index.html", index.data.into_owned());
    }

    (
        axum::http::StatusCode::NOT_FOUND,
        "Web UI assets not found. Build with cargo build --release.",
    )
        .into_response()
}

fn response_with_content_type(path: &str, bytes: Vec<u8>) -> Response {
    let mime = from_path(path).first_or_octet_stream();
    let mut headers = axum::http::header::HeaderMap::new();
    if let Ok(header_value) = axum::http::header::HeaderValue::from_str(mime.as_ref()) {
        headers.insert(axum::http::header::CONTENT_TYPE, header_value);
    }

    (axum::http::StatusCode::OK, headers, bytes).into_response()
}

/// Convenience function to run server with default config
pub async fn run(port: u16) -> Result<(), Box<dyn std::error::Error>> {
    let config = ServerConfig {
        addr: SocketAddr::from(([0, 0, 0, 0], port)),
        ..Default::default()
    };
    run_server(config).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_server_config_default() {
        let config = ServerConfig::default();
        assert_eq!(config.addr.port(), 8080);
        assert!(config.db_path.is_none());
        assert_eq!(config.max_concurrent_tasks, 4);
    }
}
