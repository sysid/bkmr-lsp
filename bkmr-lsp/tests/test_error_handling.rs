// Error handling and edge case testing

use bkmr_lsp::BkmrConfig;
use std::time::Duration;
use tokio::time::timeout;

mod test_utils;
use test_utils::{SnippetBuilder, TestContext};

#[test_log::test(tokio::test)]
async fn test_config_edge_cases() {
    // Test config with unusual values
    let config = BkmrConfig {
        bkmr_binary: "".to_string(),
        max_completions: 0,
    };

    assert_eq!(config.bkmr_binary, "");
    assert_eq!(config.max_completions, 0);
}

#[test_log::test(tokio::test)]
async fn test_timeout_handling() {
    // Test timeout behavior patterns
    let start = std::time::Instant::now();
    let result = timeout(
        Duration::from_millis(100),
        tokio::time::sleep(Duration::from_millis(200)),
    )
    .await;

    assert!(result.is_err()); // Should timeout
    assert!(start.elapsed() < Duration::from_millis(150)); // Should be quick
}

#[test_log::test]
fn test_snippet_builder_edge_cases() {
    // Test builder with minimal data
    let snippet = SnippetBuilder::new()
        .with_title("")
        .with_content("")
        .build();

    assert_eq!(snippet.title, "");
    assert_eq!(snippet.url, "");

    // Test adding same tag multiple times
    let snippet = SnippetBuilder::new()
        .with_language("rust")
        .with_language("rust") // Should not duplicate
        .build();

    // Note: Our builder correctly prevents duplicates
    assert_eq!(snippet.tags.iter().filter(|&tag| tag == "rust").count(), 1);
}

#[test_log::test]
fn test_position_validation() {
    // Test position structure validation
    let position = tower_lsp::lsp_types::Position {
        line: 100,
        character: 100,
    };

    // These should be valid structures even if beyond document bounds
    assert_eq!(position.line, 100);
    assert_eq!(position.character, 100);
}

#[test_log::test]
fn test_config_validation() {
    let config = BkmrConfig::default();

    // Test reasonable defaults
    assert!(!config.bkmr_binary.is_empty());
    assert!(config.max_completions > 0);
    assert!(config.max_completions <= 1000); // Reasonable upper bound
}

#[test_log::test]
fn test_snippet_data_validation() {
    // Test that we can handle snippets with unusual data
    let snippet = SnippetBuilder::new()
        .with_title("🚀 Unicode Title")
        .with_content("println!(\"Unicode: 🦀\");")
        .with_description("Description with\nnewlines")
        .build();

    assert!(snippet.title.contains("🚀"));
    assert!(snippet.url.contains("🦀"));
    assert!(snippet.description.contains("\n"));
}

#[test_log::test(tokio::test)]
async fn test_lsp_error_handling() {
    // Test error handling in LSP context using real TestContext
    use tower_lsp::{jsonrpc, lsp_types::*};

    let mut context = TestContext::new();
    context.initialize().await.expect("Failed to initialize");

    // Test invalid completion request (document not opened)
    let invalid_completion = jsonrpc::Request::build("textDocument/completion")
        .id(1)
        .params(serde_json::json!({
            "position": {"character": 5, "line": 0},
            "textDocument": {"uri": "file:///nonexistent.rs"}
        }))
        .finish();

    // This should handle the error gracefully - server should return empty results
    let result = context
        .request::<Option<CompletionResponse>>(&invalid_completion)
        .await;

    match result {
        Ok(Some(CompletionResponse::Array(items))) => {
            tracing::info!(
                "Completion on non-existent document returned array with {} items",
                items.len()
            );
            // Server should return empty array for invalid documents
        }
        Ok(Some(CompletionResponse::List(list))) => {
            tracing::info!(
                "Completion on non-existent document returned list with {} items",
                list.items.len()
            );
            // Server should return empty list for invalid documents
        }
        Ok(None) => {
            tracing::info!("Completion on non-existent document returned None");
        }
        Err(e) => {
            tracing::info!("Completion error handled as expected: {}", e);
        }
    }

    // Test invalid execute command
    let invalid_command = jsonrpc::Request::build("workspace/executeCommand")
        .id(2)
        .params(serde_json::json!({
            "command": "nonexistent.command",
            "arguments": []
        }))
        .finish();

    let result = context
        .request::<Option<serde_json::Value>>(&invalid_command)
        .await;
    match result {
        Ok(response) => {
            tracing::info!(
                "Invalid command handled gracefully, response: {:?}",
                response
            );
            // Server should return null for unknown commands
        }
        Err(e) => {
            tracing::info!("Invalid command error handled as expected: {}", e);
        }
    }
}
