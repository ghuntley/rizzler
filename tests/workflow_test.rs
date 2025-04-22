#[cfg(test)]
mod workflow_tests {
    use std::path::Path;
    use std::process::Command;
    use std::fs;

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

    #[test]
    fn test_cargo_toml_version_format() {
        // Read Cargo.toml
        let cargo_toml = fs::read_to_string("Cargo.toml")
            .expect("Failed to read Cargo.toml");
        
        // Extract version
        let version_line = cargo_toml
            .lines()
            .find(|line| line.trim().starts_with("version ="))
            .expect("Could not find version in Cargo.toml");
        
        // Parse version string
        let version = version_line
            .split('"')
            .nth(1)
            .expect("Failed to parse version string");
        
        // Check format using regex
        let re = regex::Regex::new(r"^\d+\.\d+\.\d+$").unwrap();
        assert!(re.is_match(version), 
            "Version '{}' does not match semantic versioning format (MAJOR.MINOR.PATCH)", 
            version);
        
        println!("Version {} follows semantic versioning format", version);
    }
} 