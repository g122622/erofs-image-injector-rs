use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=../web-ui/src");
    println!("cargo:rerun-if-changed=../web-ui/index.html");
    println!("cargo:rerun-if-changed=../web-ui/package.json");
    println!("cargo:rerun-if-changed=../web-ui/vite.config.ts");
    println!("cargo:rerun-if-changed=../web-ui/tailwind.config.js");
    println!("cargo:rerun-if-changed=../web-ui/postcss.config.js");
    println!("cargo:rerun-if-changed=../web-ui/tsconfig.json");
    println!("cargo:rerun-if-changed=../web-ui/tsconfig.node.json");

    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR missing"));
    let static_dir = manifest_dir.join("static");

    let profile = env::var("PROFILE").unwrap_or_else(|_| "debug".to_string());
    if profile != "release" {
        ensure_placeholder_static(&static_dir);
        return;
    }

    let web_ui_dir = manifest_dir
        .parent()
        .expect("Failed to locate workspace root")
        .join("web-ui");

    run_cmd(&web_ui_dir, "npm", &["--version"]);
    run_cmd(&web_ui_dir, "npm", &["install"]);
    run_cmd(&web_ui_dir, "npm", &["run", "build"]);

    if !static_dir.join("index.html").exists() {
        panic!(
            "Frontend build did not produce expected static assets at {}",
            static_dir.display()
        );
    }
}

fn ensure_placeholder_static(static_dir: &Path) {
    if fs::create_dir_all(static_dir).is_err() {
        return;
    }

    let index = static_dir.join("index.html");
    if !index.exists() {
        let _ = fs::write(
            index,
            "<!doctype html><html><body><h1>Debug build: run cargo build --release to embed web-ui assets.</h1></body></html>",
        );
    }
}

fn run_cmd(dir: &Path, program: &str, args: &[&str]) {
    let status = Command::new(program)
        .current_dir(dir)
        .args(args)
        .status()
        .unwrap_or_else(|e| panic!("Failed to execute {program} {args:?}: {e}"));

    if !status.success() {
        panic!("Command failed in {}: {} {:?}", dir.display(), program, args);
    }
}
