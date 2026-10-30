/// Integration test: verify the tuirowser binary builds successfully.
use std::process::Command;

#[test]
fn test_binary_builds() {
    // Build the project
    let build = Command::new("cargo")
        .args(["build", "--release"])
        .current_dir("../..")
        .output()
        .expect("Failed to run cargo build");
    
    assert!(
        build.status.success(),
        "Build failed: {}",
        String::from_utf8_lossy(&build.stderr)
    );

    // Verify the binary exists
    let stat = Command::new("test")
        .args(["-f", "../../target/release/tuirowser"])
        .output()
        .expect("Failed to check binary existence");
    
    assert!(
        stat.status.success(),
        "Binary not found at target/release/tuirowser"
    );
}
