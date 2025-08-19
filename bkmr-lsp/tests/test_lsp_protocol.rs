// Basic LSP protocol structure testing

use tower_lsp::jsonrpc;
use tower_lsp::lsp_types::*;

#[test_log::test]
fn test_lsp_request_format() {
    // Test LSP request formatting without requiring full server
    let request = jsonrpc::Request::build("initialize")
        .id(1)
        .params(serde_json::json!({
            "capabilities": {
                "textDocument": {
                    "completion": {
                        "completionItem": {
                            "snippetSupport": true
                        }
                    }
                }
            }
        }))
        .finish();

    // Verify request structure
    let json_str = serde_json::to_string(&request).unwrap();
    assert!(json_str.contains("initialize"));
    assert!(json_str.contains("snippetSupport"));
}

#[test_log::test]
fn test_completion_item_structure() {
    // Test completion item creation
    let item = CompletionItem {
        label: "test_snippet".to_string(),
        kind: Some(CompletionItemKind::TEXT),
        detail: Some("bkmr snippet".to_string()),
        documentation: Some(Documentation::String("Test content".to_string())),
        insert_text_format: Some(InsertTextFormat::PLAIN_TEXT),
        ..Default::default()
    };

    assert_eq!(item.label, "test_snippet");
    assert_eq!(item.kind, Some(CompletionItemKind::TEXT));
    assert_eq!(item.detail, Some("bkmr snippet".to_string()));
}

#[test_log::test]
fn test_server_capabilities_structure() {
    // Test server capabilities structure
    let capabilities = ServerCapabilities {
        text_document_sync: Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL)),
        completion_provider: Some(CompletionOptions {
            resolve_provider: Some(false),
            trigger_characters: None,
            all_commit_characters: None,
            work_done_progress_options: WorkDoneProgressOptions::default(),
            completion_item: None,
        }),
        execute_command_provider: Some(ExecuteCommandOptions {
            commands: vec!["bkmr.insertFilepathComment".to_string()],
            work_done_progress_options: WorkDoneProgressOptions::default(),
        }),
        ..Default::default()
    };

    assert!(capabilities.completion_provider.is_some());
    assert!(capabilities.execute_command_provider.is_some());
}

#[test_log::test]
fn test_position_and_range() {
    // Test LSP position and range structures
    let position = Position {
        line: 5,
        character: 10,
    };

    let range = Range {
        start: position,
        end: Position {
            line: 5,
            character: 20,
        },
    };

    assert_eq!(range.start.line, 5);
    assert_eq!(range.start.character, 10);
    assert_eq!(range.end.character, 20);
}

#[test_log::test]
fn test_text_edit_structure() {
    // Test text edit structure
    let range = Range {
        start: Position {
            line: 0,
            character: 0,
        },
        end: Position {
            line: 0,
            character: 5,
        },
    };

    let edit = TextEdit {
        range,
        new_text: "replacement".to_string(),
    };

    assert_eq!(edit.new_text, "replacement");
    assert_eq!(edit.range.start.line, 0);
}
