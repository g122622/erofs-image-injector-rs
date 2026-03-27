#!/bin/bash
#
# Run QEMU with a test kernel and EROFS image
# This script is used to test EROFS images against a kernel with EROFS support.
#
# Usage:
#   ./scripts/run_qemu_test.sh <erofs_image> [options]
#
# Environment variables:
#   KERNEL     - Path to kernel bzImage (default: ./kernel_build/bzImage)
#   ROOTFS     - Path to initramfs (default: ./kernel_build/rootfs.cpio.gz)
#   MEMORY     - Memory for QEMU in MB (default: 512)
#   TIMEOUT    - Timeout in seconds (default: 60)
#   QEMU       - QEMU binary (default: qemu-system-x86_64)
#

set -e

# Configuration
KERNEL="${KERNEL:-./kernel_build/bzImage}"
ROOTFS="${ROOTFS:-./kernel_build/rootfs.cpio.gz}"
MEMORY="${MEMORY:-512}"
TIMEOUT="${TIMEOUT:-60}"
QEMU="${QEMU:-qemu-system-x86_64}"
SMP="${SMP:-2}"

# Check arguments
if [ $# -lt 1 ]; then
    echo "Usage: $0 <erofs_image> [options]"
    echo ""
    echo "Options:"
    echo "  --kernel PATH     Path to kernel bzImage"
    echo "  --rootfs PATH     Path to initramfs"
    echo "  --memory MB       Memory for QEMU"
    echo "  --timeout SEC     Timeout in seconds"
    echo "  --debug           Enable QEMU debug output"
    echo ""
    echo "Environment variables:"
    echo "  KERNEL            Path to kernel bzImage"
    echo "  ROOTFS            Path to initramfs"
    echo "  MEMORY            Memory for QEMU in MB"
    echo "  TIMEOUT           Timeout in seconds"
    echo "  QEMU              QEMU binary"
    echo ""
    echo "Example:"
    echo "  $0 ./test.erofs"
    echo "  KERNEL=./my_kernel/bzImage $0 ./test.erofs"
    exit 1
fi

# Parse arguments
EROFS_IMAGE="$1"
shift

DEBUG=0

while [ $# -gt 0 ]; do
    case "$1" in
        --kernel)
            KERNEL="$2"
            shift 2
            ;;
        --rootfs)
            ROOTFS="$2"
            shift 2
            ;;
        --memory)
            MEMORY="$2"
            shift 2
            ;;
        --timeout)
            TIMEOUT="$2"
            shift 2
            ;;
        --debug)
            DEBUG=1
            shift
            ;;
        *)
            echo "Unknown option: $1"
            exit 1
            ;;
    esac
done

# Verify files exist
if [ ! -f "$KERNEL" ]; then
    echo "Error: Kernel not found at $KERNEL"
    echo "Run scripts/build_kernel.sh first"
    exit 1
fi

if [ ! -f "$ROOTFS" ]; then
    echo "Error: Rootfs not found at $ROOTFS"
    echo "Run scripts/build_kernel.sh first"
    exit 1
fi

if [ ! -f "$EROFS_IMAGE" ]; then
    echo "Error: EROFS image not found at $EROFS_IMAGE"
    exit 1
fi

# Check QEMU is available
if ! command -v "$QEMU" &> /dev/null; then
    echo "Error: QEMU not found ($QEMU)"
    echo "Install QEMU: sudo apt install qemu-system-x86"
    exit 1
fi

echo "=============================================="
echo "QEMU EROFS Test"
echo "=============================================="
echo "Kernel:     $KERNEL"
echo "Rootfs:     $ROOTFS"
echo "Image:      $EROFS_IMAGE"
echo "Memory:     ${MEMORY}MB"
echo "Timeout:    ${TIMEOUT}s"
echo "=============================================="
echo ""

# Build QEMU arguments
QEMU_ARGS=(
    -kernel "$KERNEL"
    -initrd "$ROOTFS"
    -drive "file=$EROFS_IMAGE,format=raw,if=virtio,readonly=on"
    -append "console=ttyS0 panic=1 quiet"
    -nographic
    -no-reboot
    -m "${MEMORY}M"
    -smp "$SMP"
)

# Add debug options if requested
if [ $DEBUG -eq 1 ]; then
    QEMU_ARGS+=(
        -append "console=ttyS0 panic=1 debug loglevel=8"
    )
fi

# Run QEMU with timeout
echo "Starting QEMU..."
echo ""

# Capture output
OUTPUT_FILE=$(mktemp)
trap "rm -f $OUTPUT_FILE" EXIT

timeout "$TIMEOUT" "$QEMU" "${QEMU_ARGS[@]}" 2>&1 | tee "$OUTPUT_FILE" || EXIT_CODE=$?

EXIT_CODE=${EXIT_CODE:-0}

echo ""
echo "=============================================="

# Analyze output
if grep -qE "(Kernel panic|kernel panic)" "$OUTPUT_FILE"; then
    echo "Result: KERNEL PANIC DETECTED"
    echo ""
    grep -A 20 "Kernel panic\|kernel panic" "$OUTPUT_FILE" | head -30
    EXIT_CODE=1
elif grep -qE "(Oops:|BUG:|general protection fault)" "$OUTPUT_FILE"; then
    echo "Result: KERNEL OOPS/BUG DETECTED"
    echo ""
    grep -A 10 "Oops:\|BUG:\|general protection fault" "$OUTPUT_FILE" | head -20
    EXIT_CODE=1
elif grep -qE "(EROFS error|erofs: error|erofs_readpage)" "$OUTPUT_FILE"; then
    echo "Result: EROFS ERROR DETECTED"
    echo ""
    grep "EROFS\|erofs" "$OUTPUT_FILE"
    EXIT_CODE=1
elif grep -qE "(Call Trace:|RIP:)" "$OUTPUT_FILE"; then
    echo "Result: KERNEL CRASH DETECTED"
    echo ""
    grep -A 15 "Call Trace\|RIP:" "$OUTPUT_FILE" | head -25
    EXIT_CODE=1
elif [ $EXIT_CODE -eq 124 ]; then
    echo "Result: TIMEOUT (no crash detected within ${TIMEOUT}s)"
elif [ $EXIT_CODE -eq 0 ]; then
    echo "Result: SUCCESS (clean shutdown)"
else
    echo "Result: UNKNOWN (exit code: $EXIT_CODE)"
fi

echo "=============================================="

exit $EXIT_CODE
