# EROFS Image Injector

A LibAFL-based fuzzing tool for EROFS filesystem images with structure-aware mutation strategies.

## Overview

This tool is part of the GSoC 2026 project "Advanced Fuzzing and Image Injection for the Kernel and erofs-utils". It performs fuzzing by:

1. Generating valid EROFS seed images using `mkfs.erofs`
2. Applying structure-aware mutations to the images
3. Mounting mutated images with `erofsfuse` to trigger parsing code paths
4. Detecting crashes, memory errors (via ASan), and unexpected behavior

## Features

- **Structure-aware mutations**: Mutations that understand EROFS on-disk format
- **Multiple injection targets**: Superblock, Inodes, Directories, Xattrs, Compressed data
- **Coverage-guided fuzzing**: Uses LibAFL's coverage feedback mechanisms
- **Multi-process parallel fuzzing**: Scales across CPU cores
- **AddressSanitizer integration**: Detects memory safety issues

## Project Structure

```
erofs-image-injector-rs/
├── erofs-fuzzer/      # Main fuzzer binary
├── erofs-format/      # EROFS on-disk format definitions
├── erofs-mutator/     # Structure-aware mutators
├── erofs-generator/   # Seed image generation
└── erofs-input/       # Custom LibAFL input type
```

## Prerequisites

### Linux (WSL2 or native)

1. **Rust toolchain**:
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

2. **erofs-utils with FUSE support** (compiled with ASan):
   ```bash
   # Install dependencies
   sudo apt install -y build-essential clang llvm libfuse3-dev liblz4-dev libzstd-dev

   # Clone and build erofs-utils with ASan
   git clone https://git.kernel.org/pub/scm/linux/kernel/git/xiang/erofs-utils.git
   cd erofs-utils
   CC=clang CFLAGS="-fsanitize=address -fno-omit-frame-pointer -g -fcommon" \
     LDFLAGS="-fsanitize=address" ./configure --enable-fuse
   make -j$(nproc)
   ```

3. **mkfs.erofs** (for seed generation):
   ```bash
   # Usually included with erofs-utils, or install separately
   sudo apt install erofs-utils
   ```

## Building

```bash
cd erofs-image-injector-rs
cargo build --release
```

## Usage

```bash
# Basic usage
./target/release/erofs-fuzzer \
  --seeds ./seeds \
  --output ./crashes \
  --erofsfuse-path /path/to/erofsfuse \
  --iterations 100000

# With all options
./target/release/erofs-fuzzer \
  --seeds ./seeds \
  --output ./crashes \
  --timeout 60 \
  --iterations 1000000 \
  --erofsfuse-path ./erofs-utils/fuse/erofsfuse \
  --workers 4 \
  --log-level debug
```

### Command Line Options

| Option | Description | Default |
|--------|-------------|---------|
| `-s, --seeds` | Directory containing seed images | Required |
| `-o, --output` | Output directory for crashes | `./crashes` |
| `-t, --timeout` | Timeout per execution (seconds) | `60` |
| `-i, --iterations` | Maximum iterations (0 = unlimited) | `0` |
| `--erofsfuse-path` | Path to erofsfuse binary | `erofsfuse` |
| `-w, --workers` | Number of parallel workers | `1` |
| `--log-level` | Log level (trace/debug/info/warn/error) | `info` |

## Seed Generation

Seeds can be generated using `mkfs.erofs`:

```bash
# Create a simple directory structure
mkdir -p seed_content/dir1/dir2
echo "Hello, EROFS!" > seed_content/file.txt
echo "Nested content" > seed_content/dir1/nested.txt

# Generate EROFS image
mkfs.erofs -E noinline_data seed.erofs seed_content/

# Copy to seeds directory
cp seed.erofs seeds/
```

## Mutation Strategies

The fuzzer applies multiple mutation strategies:

1. **Bitflip mutations**: Random bit flips in the image data
2. **Superblock mutations**: Modify magic, checksum, block size, etc.
3. **Inode mutations**: Modify inode format, mode, size, uid/gid
4. **Directory mutations**: Modify dirent structures and names
5. **Xattr mutations**: Modify extended attribute entries
6. **Splice mutations**: Combine interesting parts from multiple images

## Output

Crashes and interesting inputs are saved to the output directory:

```
crashes/
├── crash-<hash>.erofs       # Crashing input
├── crash-<hash>.log         # Crash details
└── corpus/                  # Interesting inputs that found new paths
    └── sample-<hash>.erofs
```

## License

MIT OR Apache-2.0

## Acknowledgments

This project is part of GSoC 2026, mentored by the EROFS team.
