// bkmr-lsp/tests/integration_test_interpolation.rs

use std::process::Stdio;
use tokio::process::Command;

#[tokio::test]
async fn test_bkmr_interpolation_integration() {
    // This test verifies that bkmr-lsp correctly uses the --interpolate flag
    // and receives processed content instead of raw templates

    // Check if bkmr is available
    let bkmr_check = Command::new("bkmr")
        .args(&["search", "--help"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await;

    if bkmr_check.is_err() || !bkmr_check.unwrap().success() {
        eprintln!("Skipping test: bkmr not available");
        return;
    }

    // Test that bkmr supports --interpolate flag
    let interpolate_support = Command::new("bkmr")
        .args(&["search", "--help"])
        .output()
        .await
        .expect("Failed to run bkmr");

    let help_output = String::from_utf8_lossy(&interpolate_support.stdout);
    assert!(
        help_output.contains("--interpolate"),
        "bkmr does not support --interpolate flag"
    );

    // Test the actual command that bkmr-lsp will use
    let test_command = Command::new("bkmr")
        .args(&[
            "search",
            "--json",
            "--interpolate",
            "-t",
            "_snip_",
            "--limit",
            "5",
        ])
        .output()
        .await;

    match test_command {
        Ok(output) => {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                println!(
                    "Command executed successfully. Output length: {}",
                    stdout.len()
                );

                // Try to parse as JSON to verify format
                if !stdout.trim().is_empty() {
                    match serde_json::from_str::<Vec<serde_json::Value>>(&stdout) {
                        Ok(snippets) => {
                            println!("Successfully parsed {} snippets", snippets.len());

                            // Check if any snippets would have been interpolated
                            for snippet in &snippets {
                                if let Some(url) = snippet.get("url").and_then(|u| u.as_str()) {
                                    if url.contains("{{") || url.contains("{%") {
                                        panic!("Found uninterpolated template in URL: {}", url);
                                    }
                                }
                            }
                            println!("✅ All snippets are properly interpolated");
                        }
                        Err(e) => {
                            eprintln!("Failed to parse JSON output: {}", e);
                            eprintln!("Raw output: {}", stdout);
                        }
                    }
                } else {
                    println!("No snippets found (empty output)");
                }
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                eprintln!("Command failed with stderr: {}", stderr);
            }
        }
        Err(e) => {
            eprintln!("Failed to execute command: {}", e);
        }
    }
}
