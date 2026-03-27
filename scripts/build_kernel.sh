#!/bin/bash
#
# Build Linux Kernel with EROFS support and debugging options
# This script downloads, configures, and builds a Linux kernel suitable
# for fuzzing the EROFS filesystem driver.
#
# Usage:
#   ./scripts/build_kernel.sh [VERSION]
#
# Environment variables:
#   KERNEL_VERSION  - Linux kernel version (default: 6.8)
#   BUILD_DIR       - Build directory (default: ./kernel_build)
#   JOBS            - Number of parallel jobs (default: auto-detect)
#

set -e

# Configuration
KERNEL_VERSION="${KERNEL_VERSION:-6.8}"
BUILD_DIR="${BUILD_DIR:-./kernel_build}"
JOBS="${JOBS:-$(nproc)}"

KERNEL_SRC="$BUILD_DIR/linux-$KERNEL_VERSION"
KERNEL_OUTPUT="$BUILD_DIR/bzImage"
ROOTFS_OUTPUT="$BUILD_DIR/rootfs.cpio.gz"
BUSYBOX_URL="https://busybox.net/downloads/binaries/1.35.0-x86_64-linux-musl/busybox"

echo "=============================================="
echo "Building Linux $KERNEL_VERSION with EROFS support"
echo "Build directory: $BUILD_DIR"
echo "Parallel jobs: $JOBS"
echo "=============================================="

# Create build directory
mkdir -p "$BUILD_DIR"

# 1. Download kernel source
download_kernel() {
    echo "[1/5] Downloading Linux $KERNEL_VERSION..."

    if [ -d "$KERNEL_SRC" ]; then
        echo "Kernel source already exists, skipping download"
        return 0
    fi

    local KERNEL_URL="https://cdn.kernel.org/pub/linux/kernel/v${KERNEL_VERSION%%.*}.x/linux-$KERNEL_VERSION.tar.xz"
    local KERNEL_TAR="$BUILD_DIR/linux-$KERNEL_VERSION.tar.xz"

    if [ ! -f "$KERNEL_TAR" ]; then
        echo "Downloading from $KERNEL_URL..."
        wget -q --show-progress "$KERNEL_URL" -O "$KERNEL_TAR" || {
            echo "Failed to download from kernel.org, trying alternative..."
            KERNEL_URL="https://mirrors.edge.kernel.org/pub/linux/kernel/v${KERNEL_VERSION%%.*}.x/linux-$KERNEL_VERSION.tar.xz"
            wget -q --show-progress "$KERNEL_URL" -O "$KERNEL_TAR"
        }
    fi

    echo "Extracting kernel source..."
    tar xf "$KERNEL_TAR" -C "$BUILD_DIR"
}

# 2. Configure kernel
configure_kernel() {
    echo "[2/5] Configuring kernel..."

    cd "$KERNEL_SRC"

    # Start with default config
    make defconfig

    # Enable EROFS and debugging options
    cat >> .config << 'EOF'

# ============================================
# EROFS Filesystem Support
# ============================================
CONFIG_EROFS_FS=y
CONFIG_EROFS_FS_DEBUG=y
CONFIG_EROFS_FS_XATTR=y
CONFIG_EROFS_FS_ZIP=y
CONFIG_EROFS_FS_ZIP_LZ4=y
CONFIG_EROFS_FS_ZIP_LZMA=y
CONFIG_EROFS_FS_ZIP_DEFLATE=y
CONFIG_EROFS_FS_ZIP_ZSTD=y
CONFIG_EROFS_FS_ONDEMAND=y

# ============================================
# Debugging Options
# ============================================
CONFIG_DEBUG_INFO=y
CONFIG_DEBUG_INFO_DWARF_TOOLCHAIN_DEFAULT=y
CONFIG_DEBUG_INFO_REDUCED=n
CONFIG_DEBUG_INFO_COMPRESSED=n
CONFIG_FRAME_POINTER=y
CONFIG_STACKTRACE=y
CONFIG_DEBUG_STACK_USAGE=y

# ============================================
# KASAN - Kernel Address Sanitizer
# ============================================
CONFIG_KASAN=y
CONFIG_KASAN_INLINE=y
CONFIG_KASAN_STACK=y
CONFIG_KASAN_VMALLOC=y
CONFIG_KASAN_KUNIT_TEST=n

# ============================================
# KCOV - Code Coverage
# ============================================
CONFIG_KCOV=y
CONFIG_KCOV_ENABLE_COMPARISONS=y
CONFIG_KCOV_INSTRUMENT_ALL=y

# ============================================
# Other Debug Options
# ============================================
CONFIG_DEBUG_KMEMLEAK=y
CONFIG_DEBUG_VM=y
CONFIG_LOCKDEP=y
CONFIG_PROVE_LOCKING=y
CONFIG_DEBUG_ATOMIC_SLEEP=y
CONFIG_DEBUG_LIST=y
CONFIG_DEBUG_PLIST=y
CONFIG_DEBUG_SG=y
CONFIG_DEBUG_NOTIFIERS=y
CONFIG_DEBUG_CREDENTIALS=y
CONFIG_DEBUG_BOOT_PARAMS=y

# ============================================
# Required Drivers
# ============================================
CONFIG_VIRTIO=y
CONFIG_VIRTIO_PCI=y
CONFIG_VIRTIO_BLK=y
CONFIG_VIRTIO_NET=y
CONFIG_BLK_DEV_INITRD=y
CONFIG_DEVTMPFS=y
CONFIG_DEVTMPFS_MOUNT=y

# ============================================
# Filesystems
# ============================================
CONFIG_PROC_FS=y
CONFIG_SYSFS=y
CONFIG_TMPFS=y
CONFIG_DEBUG_FS=y

# ============================================
# Required for Testing
# ============================================
CONFIG_PRINTK=y
CONFIG_PRINTK_TIME=y
CONFIG_PRINTK_CALLER=y
CONFIG_CONSOLE_LOGLEVEL_DEFAULT=8
CONFIG_MESSAGE_LOGLEVEL_DEFAULT=7
CONFIG_DYNAMIC_DEBUG=y

EOF

    # Resolve any new config options
    make olddefconfig

    echo "Kernel configuration complete"
    echo "EROFS support: $(grep CONFIG_EROFS_FS .config | head -1)"
    echo "KASAN support: $(grep CONFIG_KASAN .config | head -1)"
}

# 3. Build kernel
build_kernel() {
    echo "[3/5] Building kernel (this may take a while)..."

    cd "$KERNEL_SRC"

    make -j"$JOBS" bzImage

    # Copy kernel image
    cp arch/x86/boot/bzImage "$KERNEL_OUTPUT"

    echo "Kernel built: $KERNEL_OUTPUT"
    ls -lh "$KERNEL_OUTPUT"
}

# 4. Build minimal root filesystem
build_rootfs() {
    echo "[4/5] Building minimal root filesystem..."

    local ROOTFS_DIR="$BUILD_DIR/rootfs"
    mkdir -p "$ROOTFS_DIR"/{bin,sbin,etc,proc,sys,dev,mnt,tmp,lib,lib64}

    # Download busybox
    local BUSYBOX="$BUILD_DIR/busybox"
    if [ ! -f "$BUSYBOX" ]; then
        echo "Downloading busybox..."
        wget -q --show-progress "$BUSYBOX_URL" -O "$BUSYBOX"
        chmod +x "$BUSYBOX"
    fi

    # Install busybox
    cp "$BUSYBOX" "$ROOTFS_DIR/bin/"

    # Create symlinks for busybox applets
    cd "$ROOTFS_DIR/bin"
    for prog in sh ash cat ls mkdir mount umount echo cp mv rm ln chmod chown \
                grep sed awk head tail wc tr cut sort uniq diff find xargs \
                dmesg ps kill sleep test; do
        ln -sf busybox "$prog" 2>/dev/null || true
    done

    # Create init script
    cat > "$ROOTFS_DIR/init" << 'INIT_EOF'
#!/bin/sh

# Mount essential filesystems
mount -t proc none /proc
mount -t sysfs none /sys
mount -t devtmpfs none /dev

# Print banner
echo ""
echo "======================================"
echo "  EROFS Kernel Test Environment"
echo "======================================"
echo ""

# Wait for devices to settle
sleep 1

# Find and test EROFS image
echo "Looking for EROFS image device..."
for dev in /dev/vd* /dev/sd* /dev/hd*; do
    if [ -b "$dev" ]; then
        echo ""
        echo "Found block device: $dev"
        echo "Attempting to mount as EROFS..."

        if mount -t erofs "$dev" /mnt 2>&1; then
            echo "SUCCESS: EROFS mounted from $dev"
            echo ""
            echo "Contents:"
            ls -la /mnt/
            echo ""

            # Traversal test
            echo "Traversing filesystem..."
            find /mnt -type f 2>/dev/null | head -20

            # Unmount
            umount /mnt
            echo ""
            echo "Unmount successful"
        else
            echo "Not an EROFS image or mount failed"
        fi
    fi
done

echo ""
echo "======================================"
echo "  Test complete, powering off"
echo "======================================"
echo ""

# Power off
poweroff -f
INIT_EOF

    chmod +x "$ROOTFS_DIR/init"

    # Create minimal /etc
    echo "root:x:0:0:root:/root:/bin/sh" > "$ROOTFS_DIR/etc/passwd"
    echo "root:x:0:" > "$ROOTFS_DIR/etc/group"

    # Create initramfs
    echo "Creating initramfs..."
    cd "$ROOTFS_DIR"
    find . | cpio -H newc -o 2>/dev/null | gzip > "$ROOTFS_OUTPUT"

    echo "Rootfs built: $ROOTFS_OUTPUT"
    ls -lh "$ROOTFS_OUTPUT"
}

# 5. Print summary
print_summary() {
    echo ""
    echo "[5/5] Build Summary"
    echo "=============================================="
    echo "Kernel: $KERNEL_OUTPUT"
    echo "Rootfs: $ROOTFS_OUTPUT"
    echo ""
    echo "To test:"
    echo "  ./scripts/run_qemu_test.sh <erofs_image>"
    echo ""
    echo "Or use with the fuzzer:"
    echo "  ./target/release/erofs-fuzzer \\"
    echo "    --seeds ./seeds \\"
    echo "    --executor qemu \\"
    echo "    --kernel $KERNEL_OUTPUT \\"
    echo "    --initramfs $ROOTFS_OUTPUT"
    echo "=============================================="
}

# Main
main() {
    download_kernel
    configure_kernel
    build_kernel
    build_rootfs
    print_summary
}

main "$@"
