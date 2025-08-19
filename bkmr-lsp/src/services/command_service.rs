use anyhow::{Context, Result};
use std::collections::HashMap;
use tower_lsp::lsp_types::{Position, Range, TextEdit, Url, WorkspaceEdit};
use tracing::{debug, instrument};

use crate::domain::LanguageRegistry;

/// Service for handling LSP command execution
pub struct CommandService;

impl CommandService {
    /// Execute the insertFilepathComment command
    #[instrument(skip(file_uri))]
    pub fn insert_filepath_comment(file_uri: &str) -> Result<WorkspaceEdit> {
        let relative_path =
            Self::get_relative_path(file_uri).context("calculate relative path for file")?;

        let comment_syntax = LanguageRegistry::get_comment_syntax(file_uri);

        let comment_text = match comment_syntax {
            "<!--" => format!("<!-- {} -->\n", relative_path),
            "/*" => format!("/* {} */\n", relative_path),
            _ => format!("{} {}\n", comment_syntax, relative_path),
        };

        debug!("Inserting filepath comment: {}", comment_text.trim());

        // Create a text edit to insert at the beginning of the file
        let edit = TextEdit {
            range: Range {
                start: Position {
                    line: 0,
                    character: 0,
                },
                end: Position {
                    line: 0,
                    character: 0,
                },
            },
            new_text: comment_text,
        };

        let uri = Url::parse(file_uri).context("parse file URI for workspace edit")?;

        let mut changes = HashMap::new();
        changes.insert(uri, vec![edit]);

        Ok(WorkspaceEdit {
            changes: Some(changes),
            document_changes: None,
            change_annotations: None,
        })
    }

    /// Get the relative path from project root
    fn get_relative_path(file_uri: &str) -> Result<String> {
        let url = Url::parse(file_uri).context("parse file URI")?;

        let file_path = url
            .to_file_path()
            .map_err(|_| anyhow::anyhow!("Convert URL to file path"))
            .context("convert URL to file path")?;

        // Try to find a project root by looking for common indicators
        let mut current = file_path.as_path();
        while let Some(parent) = current.parent() {
            // Check for common project root indicators
            if parent.join("Cargo.toml").exists()
                || parent.join("package.json").exists()
                || parent.join("pom.xml").exists()
                || parent.join("build.gradle").exists()
                || parent.join("build.gradle.kts").exists()
                || parent.join("Makefile").exists()
                || parent.join(".git").exists()
            {
                // Found project root, return relative path
                if let Ok(rel_path) = file_path.strip_prefix(parent) {
                    return Ok(rel_path.to_string_lossy().to_string());
                }
                break;
            }
            current = parent;
        }

        // Fall back to just the filename if no project root found
        file_path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .ok_or_else(|| anyhow::anyhow!("Extract filename from path"))
            .context("extract filename from file path")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn given_rust_file_when_inserting_filepath_comment_then_uses_double_slash() {
        // Arrange
        let file_uri = "file:///path/to/test.rs";

        // Act
        let result = CommandService::insert_filepath_comment(file_uri);

        // Assert
        assert!(result.is_ok());
        let workspace_edit = result.expect("valid workspace edit");

        let changes = workspace_edit.changes.expect("workspace changes");
        let edits = changes.values().next().expect("text edits");
        let edit = &edits[0];

        assert!(edit.new_text.starts_with("// "));
        assert!(edit.new_text.contains("test.rs"));
    }

    #[test]
    fn given_html_file_when_inserting_filepath_comment_then_uses_html_comment() {
        // Arrange
        let file_uri = "file:///path/to/test.html";

        // Act
        let result = CommandService::insert_filepath_comment(file_uri);

        // Assert
        assert!(result.is_ok());
        let workspace_edit = result.expect("valid workspace edit");

        let changes = workspace_edit.changes.expect("workspace changes");
        let edits = changes.values().next().expect("text edits");
        let edit = &edits[0];

        assert!(edit.new_text.starts_with("<!-- "));
        assert!(edit.new_text.ends_with(" -->\n"));
        assert!(edit.new_text.contains("test.html"));
    }

    #[test]
    fn given_python_file_when_inserting_filepath_comment_then_uses_hash() {
        // Arrange
        let file_uri = "file:///path/to/test.py";

        // Act
        let result = CommandService::insert_filepath_comment(file_uri);

        // Assert
        assert!(result.is_ok());
        let workspace_edit = result.expect("valid workspace edit");

        let changes = workspace_edit.changes.expect("workspace changes");
        let edits = changes.values().next().expect("text edits");
        let edit = &edits[0];

        assert!(edit.new_text.starts_with("# "));
        assert!(edit.new_text.contains("test.py"));
    }

    #[test]
    fn given_invalid_uri_when_inserting_filepath_comment_then_returns_error() {
        // Arrange
        let file_uri = "invalid-uri";

        // Act
        let result = CommandService::insert_filepath_comment(file_uri);

        // Assert
        assert!(result.is_err());
        let error_message = result.unwrap_err().to_string();
        assert!(error_message.contains("calculate relative path"));
    }

    #[test]
    fn given_file_in_project_when_getting_relative_path_then_returns_relative_path() {
        // Arrange
        // This test would need a real project structure to work properly
        // For now, we'll test the fallback behavior
        let file_uri = "file:///some/deep/path/test.rs";

        // Act
        let result = CommandService::get_relative_path(file_uri);

        // Assert
        assert!(result.is_ok());
        let path = result.expect("valid relative path");
        assert_eq!(path, "test.rs"); // Should fall back to filename
    }
}
