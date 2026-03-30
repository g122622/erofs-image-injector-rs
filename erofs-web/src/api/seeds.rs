//! Seed management API handlers

use std::path::PathBuf;

use axum::{
    extract::{Multipart, Path, Query, State},
    http::{header, StatusCode},
    response::Response,
    Json,
};
use tokio::fs;
use tracing::{debug, error, info};

use crate::seeds::{get_default_templates, get_template_by_id, SeedGenerator, SeedGenError};
use crate::types::*;

use super::{ApiState, ErrorResponse};

/// List seeds
pub async fn list_seeds(
    State(state): State<ApiState>,
    Query(filter): Query<SeedFilter>,
) -> Result<Json<Vec<Seed>>, (StatusCode, Json<ErrorResponse>)> {
    debug!("Listing seeds with filter: {:?}", filter);

    let seeds = state.db.list_seeds(&filter).await
        .map_err(|e| {
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse::from(format!("Failed to list seeds: {}", e))))
        })?;

    Ok(Json(seeds))
}

/// Get a single seed by ID
pub async fn get_seed(
    State(state): State<ApiState>,
    Path(id): Path<i64>,
) -> Result<Json<Seed>, (StatusCode, Json<ErrorResponse>)> {
    debug!("Getting seed: {}", id);

    let seed = state.db.get_seed(id).await
        .map_err(|e| {
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse::from(format!("Failed to get seed: {}", e))))
        })?
        .ok_or_else(|| {
            (StatusCode::NOT_FOUND, Json(ErrorResponse::from("Seed not found")))
        })?;

    Ok(Json(seed))
}

/// Generate seeds from configuration
pub async fn generate_seeds(
    State(state): State<ApiState>,
    Json(request): Json<CreateSeedRequest>,
) -> Result<Json<Vec<Seed>>, (StatusCode, Json<ErrorResponse>)> {
    debug!("Generating seeds: {} (count: {:?})", request.name, request.count);

    let count = request.count.unwrap_or(1);
    let output_dir = std::env::current_dir()
        .map(|p| p.join("seeds"))
        .unwrap_or_else(|_| PathBuf::from("./seeds"));

    // Ensure output directory exists
    fs::create_dir_all(&output_dir).await
        .map_err(|e| {
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse::from(format!("Failed to create output directory: {}", e))))
        })?;

    // Find mkfs.erofs
    let mkfs_path = find_mkfs_erofs()
        .map_err(|e| {
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse::from(e)))
        })?;

    // Clone for the closure
    let name = request.name.clone();
    let name_for_db = request.name.clone();
    let config = request.config.clone();
    let output_dir_clone = output_dir.clone();
    let db = state.db.clone();

    // Generate seeds in a blocking task
    let seeds = tokio::task::spawn_blocking(move || {
        let mut generator = SeedGenerator::new(&mkfs_path, &output_dir_clone);
        generator.generate_batch(&name, &config, count)
    }).await
        .map_err(|e| {
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse::from(format!("Generation task failed: {}", e))))
        })?
        .map_err(|e: SeedGenError| {
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse::from(format!("Seed generation failed: {}", e))))
        })?;

    // Store seeds in database
    let mut stored_seeds = Vec::new();
    for (path, size) in &seeds {
        let seed_name = path.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or(&name_for_db)
            .to_string();

        // Calculate checksum
        let checksum = calculate_file_checksum(path)
            .map_err(|e| {
                (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse::from(format!("Failed to calculate checksum: {}", e))))
            })?;

        let seed_id = db.create_seed(
            &seed_name,
            &path.to_string_lossy(),
            *size,
            Some(&checksum),
            &request.config,
        ).await
            .map_err(|e| {
                (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse::from(format!("Failed to create seed record: {}", e))))
            })?;

        let seed = db.get_seed(seed_id).await
            .map_err(|e| {
                (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse::from(format!("Failed to get created seed: {}", e))))
            })?
            .ok_or_else(|| {
                (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse::from("Created seed not found")))
            })?;

        stored_seeds.push(seed);
    }

    info!("Generated {} seeds", stored_seeds.len());
    Ok(Json(stored_seeds))
}

/// Upload seed file
pub async fn upload_seed(
    State(state): State<ApiState>,
    mut multipart: Multipart,
) -> Result<Json<Seed>, (StatusCode, Json<ErrorResponse>)> {
    debug!("Uploading seed file");

    let mut name: Option<String> = None;
    let mut file_data: Option<Vec<u8>> = None;
    let mut config: Option<SeedConfig> = None;

    while let Some(field) = multipart.next_field().await
        .map_err(|e| {
            (StatusCode::BAD_REQUEST, Json(ErrorResponse::from(format!("Multipart error: {}", e))))
        })? {
        let field_name = field.name().unwrap_or("").to_string();

        match field_name.as_str() {
            "name" => {
                name = Some(field.text().await
                    .map_err(|e| {
                        (StatusCode::BAD_REQUEST, Json(ErrorResponse::from(format!("Failed to read name: {}", e))))
                    })?);
            }
            "config" => {
                let config_text = field.text().await
                    .map_err(|e| {
                        (StatusCode::BAD_REQUEST, Json(ErrorResponse::from(format!("Failed to read config: {}", e))))
                    })?;
                if !config_text.is_empty() {
                    config = Some(serde_json::from_str(&config_text)
                        .map_err(|e| {
                            (StatusCode::BAD_REQUEST, Json(ErrorResponse::from(format!("Invalid config JSON: {}", e))))
                        })?);
                }
            }
            "file" => {
                file_data = Some(field.bytes().await
                    .map_err(|e| {
                        (StatusCode::BAD_REQUEST, Json(ErrorResponse::from(format!("Failed to read file: {}", e))))
                    })?
                    .to_vec());
            }
            _ => {}
        }
    }

    let name = name.ok_or_else(|| {
        (StatusCode::BAD_REQUEST, Json(ErrorResponse::from("Name is required")))
    })?;
    let file_data = file_data.ok_or_else(|| {
        (StatusCode::BAD_REQUEST, Json(ErrorResponse::from("File is required")))
    })?;
    let config = config.unwrap_or_default();

    // Save file
    let output_dir = std::env::current_dir()
        .map(|p| p.join("seeds"))
        .unwrap_or_else(|_| PathBuf::from("./seeds"));

    fs::create_dir_all(&output_dir).await
        .map_err(|e| {
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse::from(format!("Failed to create output directory: {}", e))))
        })?;

    let file_path = output_dir.join(format!("{}.erofs", name));
    fs::write(&file_path, &file_data).await
        .map_err(|e| {
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse::from(format!("Failed to write file: {}", e))))
        })?;

    // Calculate checksum
    let checksum = {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(&file_data);
        format!("{:x}", hasher.finalize())
    };

    let file_size = file_data.len() as i64;

    // Check for duplicate
    if state.db.seed_exists_by_checksum(&checksum).await
        .map_err(|e| {
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse::from(format!("Database error: {}", e))))
        })? {
        // Remove uploaded file
        let _ = fs::remove_file(&file_path).await;
        return Err((StatusCode::CONFLICT, Json(ErrorResponse::from("Seed with this checksum already exists"))));
    }

    // Create seed record
    let seed_id = state.db.create_seed(
        &name,
        &file_path.to_string_lossy(),
        file_size,
        Some(&checksum),
        &config,
    ).await
        .map_err(|e| {
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse::from(format!("Failed to create seed record: {}", e))))
        })?;

    let seed = state.db.get_seed(seed_id).await
        .map_err(|e| {
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse::from(format!("Failed to get created seed: {}", e))))
        })?
        .ok_or_else(|| {
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse::from("Created seed not found")))
        })?;

    info!("Uploaded seed: {} ({})", name, file_path.display());
    Ok(Json(seed))
}

/// Delete a seed
pub async fn delete_seed(
    State(state): State<ApiState>,
    Path(id): Path<i64>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    debug!("Deleting seed: {}", id);

    let seed = state.db.get_seed(id).await
        .map_err(|e| {
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse::from(format!("Failed to get seed: {}", e))))
        })?
        .ok_or_else(|| {
            (StatusCode::NOT_FOUND, Json(ErrorResponse::from("Seed not found")))
        })?;

    // Delete file
    if let Err(e) = fs::remove_file(&seed.file_path).await {
        error!("Failed to delete seed file {}: {}", seed.file_path, e);
        // Continue anyway to remove DB record
    }

    // Delete database record
    state.db.delete_seed(id).await
        .map_err(|e| {
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse::from(format!("Failed to delete seed: {}", e))))
        })?;

    info!("Deleted seed: {}", id);
    Ok(StatusCode::NO_CONTENT)
}

/// Download a seed file
pub async fn download_seed(
    State(state): State<ApiState>,
    Path(id): Path<i64>,
) -> Result<Response, (StatusCode, Json<ErrorResponse>)> {
    debug!("Downloading seed: {}", id);

    let seed = state.db.get_seed(id).await
        .map_err(|e| {
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse::from(format!("Failed to get seed: {}", e))))
        })?
        .ok_or_else(|| {
            (StatusCode::NOT_FOUND, Json(ErrorResponse::from("Seed not found")))
        })?;

    // Check if file exists
    let path = PathBuf::from(&seed.file_path);
    if !path.exists() {
        // Mark as invalid
        let _ = state.db.update_seed_validity(id, false).await;
        return Err((StatusCode::NOT_FOUND, Json(ErrorResponse::from("Seed file not found"))));
    }

    let file_data = fs::read(&path).await
        .map_err(|e| {
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse::from(format!("Failed to read seed file: {}", e))))
        })?;

    let filename = format!("{}.erofs", seed.name);

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_DISPOSITION, format!("attachment; filename=\"{}\"", filename))
        .header(header::CONTENT_TYPE, "application/octet-stream")
        .body(file_data.into())
        .map_err(|e| {
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse::from(format!("Failed to build response: {}", e))))
        })
}

/// Get seed templates
pub async fn list_templates() -> Result<Json<Vec<SeedTemplate>>, (StatusCode, Json<ErrorResponse>)> {
    let templates = get_default_templates();
    Ok(Json(templates))
}

/// Get a specific template
pub async fn get_template(
    Path(id): Path<String>,
) -> Result<Json<SeedTemplate>, (StatusCode, Json<ErrorResponse>)> {
    get_template_by_id(&id)
        .map(Json)
        .ok_or_else(|| {
            (StatusCode::NOT_FOUND, Json(ErrorResponse::from("Template not found")))
        })
}

/// Helper: Find mkfs.erofs binary
fn find_mkfs_erofs() -> Result<PathBuf, String> {
    // Try common locations
    let paths = [
        "/usr/sbin/mkfs.erofs",  // Most common on Linux
        "/usr/bin/mkfs.erofs",
        "/usr/local/sbin/mkfs.erofs",
        "/usr/local/bin/mkfs.erofs",
        "mkfs.erofs", // Check PATH
    ];

    for path in &paths {
        let pb = PathBuf::from(path);
        if pb.exists() {
            return Ok(pb);
        }
    }

    // Also try which to find in PATH
    if let Ok(path) = which::which("mkfs.erofs") {
        return Ok(path);
    }

    Err("mkfs.erofs not found. Please install erofs-utils (e.g., 'sudo apt install erofs-utils' on Ubuntu/Debian)".to_string())
}

/// Helper: Calculate SHA256 checksum of a file
fn calculate_file_checksum(path: &PathBuf) -> Result<String, std::io::Error> {
    use sha2::{Digest, Sha256};

    let data = std::fs::read(path)?;
    let mut hasher = Sha256::new();
    hasher.update(&data);
    Ok(format!("{:x}", hasher.finalize()))
}
