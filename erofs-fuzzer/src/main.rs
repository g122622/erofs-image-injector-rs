//! EROFS Image Fuzzer - Main Entry Point
//!
//! A LibAFL-based fuzzer for EROFS filesystem images.

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

    FmtSubscriber::builder()
        .with_max_level(log_level)
        .with_target(false)
        .with_thread_ids(false)
        .pretty()
        .init();

    // Handle web mode
    if args.web {
        tracing::info!("Starting EROFS Web Console on port {}", args.web_port);
        let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
        rt.block_on(async {
            if let Err(e) = erofs_web::run(args.web_port).await {
                tracing::error!("Web server error: {}", e);
                std::process::exit(1);
            }
        });
        return;
    }

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
