//! Crash API handlers

use std::path::PathBuf;
use std::fs;

use axum::{
    extract::{Path, Query, State},
    http::{StatusCode, header},
    Json,
};
use tracing::{debug, info};

use crate::api::{ApiState, ErrorResponse};
use crate::types::*;

/// List crashes with optional filter
pub async fn list_crashes(
    State(state): State<ApiState>,
    Query(filter): Query<CrashFilter>,
) -> Result<Json<Vec<Crash>>, (StatusCode, Json<ErrorResponse>)> {
    info!("[list_crashes] Listing crashes with filter: {:?}", filter);

    let crashes = state.db.list_crashes(&filter).await
        .map_err(|e| {
            info!("[list_crashes] Error: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse::from(e.to_string())))
        })?;

    info!("[list_crashes] Returning {} crashes", crashes.len());
    for crash in &crashes {
        info!("[list_crashes] Crash #{}: task_id={}, type={:?}, path={}", crash.id, crash.task_id, crash.crash_type, crash.image_path);
    }

    Ok(Json(crashes))
}

/// Get a single crash
pub async fn get_crash(
    State(state): State<ApiState>,
    Path(id): Path<i64>,
) -> Result<Json<Crash>, (StatusCode, Json<ErrorResponse>)> {
    debug!("Getting crash {}", id);

    let crash = state.db.get_crash(id).await
        .map_err(|e| {
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse::from(e.to_string())))
        })?
        .ok_or_else(|| {
            (StatusCode::NOT_FOUND, Json(ErrorResponse::from("Crash not found")))
        })?;

    Ok(Json(crash))
}

/// Get crash image file
pub async fn get_crash_image(
    State(state): State<ApiState>,
    Path(id): Path<i64>,
) -> Result<(StatusCode, [(header::HeaderName, &'static str); 2], Vec<u8>), (StatusCode, Json<ErrorResponse>)> {
    debug!("Getting crash image {}", id);

    let crash = state.db.get_crash(id).await
        .map_err(|e| {
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse::from(e.to_string())))
        })?
        .ok_or_else(|| {
            (StatusCode::NOT_FOUND, Json(ErrorResponse::from("Crash not found")))
        })?;

    let image_path = PathBuf::from(&crash.image_path);
    if !image_path.exists() {
        return Err((StatusCode::NOT_FOUND, Json(ErrorResponse::from("Image file not found"))));
    }

    let content = fs::read(&image_path)
        .map_err(|e| {
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse::from(format!("Failed to read image: {}", e))))
        })?;

    Ok((
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "application/octet-stream"),
            (header::CONTENT_DISPOSITION, "attachment"),
        ],
        content,
    ))
}

/// Get crash log file
pub async fn get_crash_log(
    State(state): State<ApiState>,
    Path(id): Path<i64>,
) -> Result<(StatusCode, [(header::HeaderName, &'static str); 2], String), (StatusCode, Json<ErrorResponse>)> {
    debug!("Getting crash log {}", id);

    let crash = state.db.get_crash(id).await
        .map_err(|e| {
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse::from(e.to_string())))
        })?
        .ok_or_else(|| {
            (StatusCode::NOT_FOUND, Json(ErrorResponse::from("Crash not found")))
        })?;

    // Check if log_path is set
    let log_path = match &crash.log_path {
        Some(path) => PathBuf::from(path),
        None => {
            // Try to find log file by image path convention
            let image_path = PathBuf::from(&crash.image_path);
            let log_path = image_path.with_extension("erofs.log");
            if log_path.exists() {
                log_path
            } else {
                return Err((StatusCode::NOT_FOUND, Json(ErrorResponse::from("No log file available"))));
            }
        }
    };

    if !log_path.exists() {
        return Err((StatusCode::NOT_FOUND, Json(ErrorResponse::from("Log file not found"))));
    }

    let content = fs::read_to_string(&log_path)
        .map_err(|e| {
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse::from(format!("Failed to read log: {}", e))))
        })?;

    Ok((
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "text/plain; charset=utf-8"),
            (header::CONTENT_DISPOSITION, "inline"),
        ],
        content,
    ))
}

/// Get reproduction script for a crash
pub async fn get_crash_repro(
    State(state): State<ApiState>,
    Path(id): Path<i64>,
) -> Result<Json<ReproductionScript>, (StatusCode, Json<ErrorResponse>)> {
    debug!("Getting reproduction script for crash {}", id);

    let crash = state.db.get_crash(id).await
        .map_err(|e| {
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse::from(e.to_string())))
        })?
        .ok_or_else(|| {
            (StatusCode::NOT_FOUND, Json(ErrorResponse::from("Crash not found")))
        })?;

    // Get the associated task for configuration
    let task = state.db.get_task(crash.task_id).await
        .map_err(|e| {
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse::from(e.to_string())))
        })?
        .ok_or_else(|| {
            (StatusCode::NOT_FOUND, Json(ErrorResponse::from("Associated task not found")))
        })?;

    // Generate reproduction script based on executor type
    let script = generate_repro_script(&crash, &task);

    Ok(Json(script))
}

/// Generate reproduction script for a crash
fn generate_repro_script(crash: &Crash, task: &Task) -> ReproductionScript {
    // Convert image path to absolute path
    let image_path = std::fs::canonicalize(&crash.image_path)
        .unwrap_or_else(|_| PathBuf::from(&crash.image_path));
    let image_path_str = image_path.to_string_lossy();

    let script = match task.executor_type {
        ExecutorType::Erofsfuse => {
            format!(
                r#"#!/bin/bash
# Reproduction script for crash #{} ({})
# Image: {}

IMAGE="{}"
MOUNT_POINT="/tmp/erofs-repro-$$"

# Create mount point
mkdir -p "$MOUNT_POINT"

# Run erofsfuse
echo "Running erofsfuse on crash image..."
erofsfuse "$IMAGE" "$MOUNT_POINT"

# Check result
EXIT_CODE=$?
echo "Exit code: $EXIT_CODE"

# Cleanup
fusermount -u "$MOUNT_POINT" 2>/dev/null || true
rmdir "$MOUNT_POINT"

exit $EXIT_CODE
"#,
                crash.id, crash.crash_type, image_path_str, image_path_str
            )
        }
        ExecutorType::Qemu => {
            // Convert kernel and initramfs paths to absolute paths
            let kernel = task.kernel_path.as_deref()
                .map(|p| std::fs::canonicalize(p).unwrap_or_else(|_| PathBuf::from(p)))
                .unwrap_or_else(|| PathBuf::from("./kernel_build/bzImage"));
            let initramfs = task.initramfs_path.as_deref()
                .map(|p| std::fs::canonicalize(p).unwrap_or_else(|_| PathBuf::from(p)))
                .unwrap_or_else(|| PathBuf::from("./kernel_build/rootfs.cpio.gz"));
            let memory = task.qemu_memory.unwrap_or(512);

            format!(
                r#"#!/bin/bash
# Reproduction script for crash #{} ({})
# Image: {}

set -e

IMAGE="{}"
KERNEL="{}"
INITRAMFS="{}"
MEMORY="{}"

# Verify files exist
if [ ! -f "$IMAGE" ]; then
    echo "Error: Crash image not found: $IMAGE"
    exit 1
fi
if [ ! -f "$KERNEL" ]; then
    echo "Error: Kernel not found: $KERNEL"
    exit 1
fi
if [ ! -f "$INITRAMFS" ]; then
    echo "Error: Initramfs not found: $INITRAMFS"
    exit 1
fi

echo "=========================================="
echo "Reproducing crash #{} ({})"
echo "Image: $IMAGE"
echo "Kernel: $KERNEL"
echo "Initramfs: $INITRAMFS"
echo "=========================================="
echo ""
echo "The kernel will boot and attempt to mount the EROFS image."
echo "Watch for kernel messages indicating the crash."
echo ""
echo "Press Ctrl+A then X to exit QEMU."
echo ""

# Run QEMU directly with the crash image
# The existing initramfs should handle mounting and testing
qemu-system-x86_64 \
    -kernel "$KERNEL" \
    -initrd "$INITRAMFS" \
    -m "$MEMORY" \
    -drive file="$IMAGE",format=raw,if=virtio,read-only=on \
    -nographic \
    -append "console=ttyS0 quiet"

echo ""
echo "QEMU exited with code: $?"
"#,
                crash.id, crash.crash_type, image_path_str,
                image_path_str,
                kernel.to_string_lossy(),
                initramfs.to_string_lossy(),
                memory,
                crash.id, crash.crash_type
            )
        }
    };

    ReproductionScript {
        script,
        script_type: "bash".to_string(),
        description: format!(
            "Reproduction script for crash #{} ({} at iteration {})",
            crash.id, crash.crash_type, crash.iteration
        ),
    }
}
