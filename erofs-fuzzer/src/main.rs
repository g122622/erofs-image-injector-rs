//! EROFS Image Fuzzer - Main Entry Point
//!
//! A LibAFL-based fuzzer for EROFS filesystem images.

use std::path::PathBuf;

use clap::Parser;
use tracing::Level;
use tracing_subscriber::FmtSubscriber;

use erofs_fuzzer::{CliArgs, run_fuzzer};

fn main() {
    // Parse command line arguments
    let args = CliArgs::parse();

    // Initialize logging
    let log_level = match args.log_level.as_str() {
        "trace" => Level::TRACE,
        "debug" => Level::DEBUG,
        "info" => Level::INFO,
        "warn" => Level::WARN,
        "error" => Level::ERROR,
        _ => Level::INFO,
    };

    let subscriber = FmtSubscriber::builder()
        .with_max_level(log_level)
        .with_target(false)
        .with_thread_ids(false)
        .pretty()
        .init();

    tracing::info!("EROFS Image Fuzzer starting...");
    tracing::info!("Seeds directory: {:?}", args.seeds);
    tracing::info!("Output directory: {:?}", args.output);
    tracing::info!("erofsfuse path: {:?}", args.erofsfuse_path);

    // Run the fuzzer
    if let Err(e) = run_fuzzer(args) {
        tracing::error!("Fuzzer error: {}", e);
        std::process::exit(1);
    }

    tracing::info!("Fuzzer finished");
}
