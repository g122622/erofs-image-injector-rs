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
    let image_path = &crash.image_path;

    let script = match task.executor_type {
        ExecutorType::Erofsfuse => {
            format!(
                r#"#!/bin/bash
# Reproduction script for crash #{} ({})

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
                crash.id, crash.crash_type, image_path
            )
        }
        ExecutorType::Qemu => {
            let kernel = task.kernel_path.as_deref().unwrap_or("./kernel_build/bzImage");
            let initramfs = task.initramfs_path.as_deref().unwrap_or("./kernel_build/rootfs.cpio.gz");
            let memory = task.qemu_memory.unwrap_or(512);

            format!(
                r#"#!/bin/bash
# Reproduction script for crash #{} ({})

IMAGE="{}"
KERNEL="{}"
INITRAMFS="{}"
MEMORY="{}"

# Create temporary initramfs with the crash image
TEMP_DIR=$(mktemp -d)
TEMP_INITRAMFS="$TEMP_DIR/rootfs.cpio.gz"

# Create a basic init script
cat > "$TEMP_DIR/init" << 'EOF'
#!/bin/sh
mount -t proc none /proc
mount -t sysfs none /sys
mount -t devtmpfs none /dev

echo "Reproducing crash #{} ({})"
echo "Looking for EROFS image device..."

for dev in /dev/vd* /dev/sd* /dev/hd*; do
    if [ -b "$dev" ]; then
        echo "Found block device: $dev"
        echo "Attempting to mount as EROFS..."
        if mount -t erofs "$dev" /mnt 2>&1; then
            echo "Mounted successfully. Listing contents:"
            ls -la /mnt/
            echo ""
            echo "Traversing filesystem..."
            find /mnt -type f 2>/dev/null | head -20
            umount /mnt
        fi
    fi
done

echo ""
echo "Test complete. Powering off."
exec busybox poweroff -f
EOF
chmod +x "$TEMP_DIR/init"

# Pack the initramfs
cd "$TEMP_DIR"
echo "Creating custom initramfs..."
(find . | cpio -o -H newc 2>/dev/null | gzip) > "$TEMP_INITRAMFS"

# Run QEMU
echo "Running QEMU with crash image..."
qemu-system-x86_64 \
    -kernel "$KERNEL" \
    -initrd "$TEMP_INITRAMFS" \
    -m "$MEMORY" \
    -drive file="$IMAGE",format=raw,if=virtio \
    -nographic \
    -append "console=ttyS0"

# Cleanup
rm -rf "$TEMP_DIR"
"#,
                crash.id, crash.crash_type, image_path, kernel, initramfs, memory,
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
