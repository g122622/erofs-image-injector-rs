# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is a LibAFL-based fuzzing tool for EROFS (Enhanced Read-Only File System) images. It performs structure-aware fuzzing by generating valid EROFS seed images, applying mutations, and testing them against both user-space (`erofsfuse`) and kernel (QEMU) parsers. Part of GSoC 2026.

## Build Commands

```bash
# Build everything (Rust backend + Vue frontend)
cargo build --release

# Run tests
cargo test

# Run specific crate tests
cargo test -p erofs-format
cargo test -p erofs-mutator
cargo test -p erofs-fuzzer

# Check without building
cargo check
```

## Running the Fuzzer

```bash
# User-space mode (erofsfuse)
./target/release/erofs-fuzzer --seeds ./seeds --iterations 1000 --timeout 30

# Kernel mode (QEMU) - requires kernel built via scripts/build_kernel.sh
./target/release/erofs-fuzzer --seeds ./seeds --executor qemu \
    --kernel ./kernel_build/bzImage \
    --initramfs ./kernel_build/rootfs.cpio.gz \
    --iterations 100 --timeout 120

# Web console mode
./target/release/erofs-fuzzer --web --web-port 8080
```

## Architecture

This is a Rust workspace with these crates:

| Crate | Purpose |
|-------|---------|
| `erofs-format` | EROFS on-disk format definitions (superblock, inode, dirent, xattr, compression) - mirrors kernel `erofs_fs.h` |
| `erofs-mutator` | Structure-aware mutators: bitflip, superblock, inode, directory, xattr, targeted |
| `erofs-input` | Custom LibAFL input type for EROFS images |
| `erofs-generator` | Seed image generation using `mkfs.erofs` |
| `erofs-fuzzer` | Main binary: CLI, fuzzer loop, executors (erofsfuse + QEMU kernel) |
| `erofs-web` | Web console backend (Axum, SQLite, WebSocket) |

### Key Files

- `erofs-fuzzer/src/cli.rs` - CLI argument definitions (clap)
- `erofs-fuzzer/src/fuzzer.rs` - Main fuzzer loop and LibAFL setup
- `erofs-fuzzer/src/executor.rs` - User-space erofsfuse executor
- `erofs-fuzzer/src/qemu_executor.rs` - QEMU kernel executor
- `erofs-fuzzer/src/kernel_monitor.rs` - Kernel crash/oops detection
- `erofs-mutator/src/*.rs` - Individual mutator implementations
- `erofs-mutator/src/field_locator.rs` - Locates EROFS struct fields by name
- `erofs-mutator/src/targeted_mutator.rs` - Targeted field-level mutations

### Web Stack

- Backend: Axum (Rust) in `erofs-web/`
- Frontend: Vue 3 + TypeScript + Tailwind CSS in `web-ui/`
- Build: `cargo build --release` runs `npm install && npm run build` via `build.rs`
- Database: SQLite for task persistence

## Testing Modes

1. **erofsfuse mode** (default): Tests user-space EROFS parser. Detects crashes via exit codes, signals, and ASan.
2. **QEMU mode** (`--executor qemu`): Tests kernel EROFS driver. Detects kernel panics, oops, KASAN reports.

## Mutator System

Mutators are selected by weight. Each mutator understands EROFS structure:
- `ErofsSuperblockMutator`: Modifies magic, checksum, block size, rootnid, features
- `ErofsInodeMutator`: Modifies i_format, i_mode, i_size, i_uid/i_gid
- `ErofsDirectoryMutator`: Modifies dirent structures and names
- `ErofsXattrMutator`: Modifies extended attribute entries
- `ErofsBitflipMutator`: Generic bit flips
- `TargetedMutator`: Precise field-level mutations via `--target` or `--range`

## Scripts

- `scripts/build_kernel.sh` - Builds Linux kernel with KASAN, KCOV, EROFS debug options
- `scripts/run_qemu_test.sh` - Tests a single EROFS image in QEMU

## Key Dependencies

- LibAFL 0.15 - Fuzzing framework
- clap - CLI parsing
- tokio - Async runtime for web server
- axum - Web framework
- rusqlite - Database
- Vue 3 + Pinia + vue-router - Frontend

## Output Structure

```
crashes/
├── crash-<hash>-signal-11.erofs     # User-space crash
├── crash-<hash>-kernel-panic.erofs  # Kernel panic
├── crash-<hash>-kernel-oops.erofs   # Kernel oops
└── *.log                            # Crash details

crashes_kernel/  # Kernel test outputs (separate directory used in practice)
```
