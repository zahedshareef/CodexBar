//! Build script for CodexBar
//!
//! Injects build metadata (git commit, build date) at compile time.

use std::process::Command;

fn main() {
    // Get git commit hash
    let git_commit = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .and_then(|output| {
            if output.status.success() {
                String::from_utf8(output.stdout).ok()
            } else {
                None
            }
        })
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    // Get build date
    let build_date = chrono::Utc::now().format("%Y-%m-%d").to_string();

    println!("cargo:rustc-env=GIT_COMMIT={}", git_commit);
    println!("cargo:rustc-env=BUILD_DATE={}", build_date);

    // Re-run if git HEAD changes
    println!("cargo:rerun-if-changed=.git/HEAD");
    if let Ok(head) = std::fs::read_to_string(".git/HEAD") {
        let head = head.trim();
        if let Some(reference) = head.strip_prefix("ref: ") {
            let reference = reference.trim();
            if !reference.is_empty() {
                println!("cargo:rerun-if-changed=.git/{}", reference);
            }
        }
    }
}
