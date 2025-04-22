#[cfg(test)]
mod workflow_tests {
    use std::path::Path;
    use std::process::Command;

    #[test]
    #[cfg(feature = "integration-tests")]
    fn test_build_release_binary() {
        // Skip this test if not running in CI
        if std::env::var("CI").is_err() {
            println!("Skipping workflow test outside of CI environment");
            return;
        }

        // Clean any existing build artifacts
        let status = Command::new("cargo")
            .args(["clean"])
            .status()
            .expect("Failed to run cargo clean");
        assert!(status.success(), "Failed to clean project");

        // Build in release mode
        let status = Command::new("cargo")
            .args(["build", "--release"])
            .status()
            .expect("Failed to run cargo build");
        
        assert!(status.success(), "Failed to build project in release mode");

        // Verify the binary exists
        #[cfg(target_os = "windows")]
        let binary_path = Path::new("target/release/rizzler.exe");
        #[cfg(not(target_os = "windows"))]
        let binary_path = Path::new("target/release/rizzler");

        assert!(binary_path.exists(), "Release binary was not created at expected path");
        
        // Basic smoke test to ensure the binary runs
        let output = Command::new(binary_path)
            .arg("--version")
            .output()
            .expect("Failed to execute rizzler binary");
        
        assert!(output.status.success(), "Binary failed to execute with --version flag");
    }
} 