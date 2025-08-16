use anyhow::{Context, Result};
use std::sync::Arc;
use tower_lsp::lsp_types::{CompletionItem, CompletionItemKind, CompletionTextEdit, Documentation, InsertTextFormat, TextEdit};
use tracing::{debug, instrument};

use crate::domain::{CompletionContext, Snippet, SnippetFilter};
use crate::repositories::SnippetRepository;
use crate::services::LanguageTranslator;

/// Service for handling completion logic
pub struct CompletionService {
    repository: Arc<dyn SnippetRepository>,
}

impl std::fmt::Debug for CompletionService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CompletionService")
            .field("repository", &"<SnippetRepository>")
            .finish()
    }
}

impl CompletionService {
    pub fn new(repository: Arc<dyn SnippetRepository>) -> Self {
        Self { repository }
    }

    /// Generate completion items from context
    #[instrument(skip(self))]
    pub async fn get_completions(&self, context: &CompletionContext) -> Result<Vec<CompletionItem>> {
        let filter = self.build_snippet_filter(context);
        
        let snippets = self.repository
            .fetch_snippets(&filter)
            .await
            .context("fetch snippets from repository")?;

        let completion_items: Vec<CompletionItem> = snippets
            .iter()
            .map(|snippet| {
                self.snippet_to_completion_item(
                    snippet,
                    context.get_query_text().unwrap_or(""),
                    context.get_replacement_range(),
                    context.language_id.as_deref().unwrap_or("unknown"),
                    &context.uri,
                )
            })
            .collect::<Result<Vec<_>>>()
            .context("convert snippets to completion items")?;

        debug!("Generated {} completion items", completion_items.len());
        Ok(completion_items)
    }

    /// Build snippet filter from completion context
    fn build_snippet_filter(&self, context: &CompletionContext) -> SnippetFilter {
        let query_prefix = context.get_query_text().map(|s| s.to_string());
        SnippetFilter::new(
            context.language_id.clone(),
            query_prefix,
            50, // TODO: Make configurable
        )
    }

    /// Convert snippet to LSP completion item with proper text replacement
    fn snippet_to_completion_item(
        &self,
        snippet: &Snippet,
        query: &str,
        replacement_range: Option<tower_lsp::lsp_types::Range>,
        language_id: &str,
        uri: &tower_lsp::lsp_types::Url,
    ) -> Result<CompletionItem> {
        // Translate content if this is a universal snippet
        let snippet_content = LanguageTranslator::translate_snippet(snippet, language_id, uri)
            .context("translate snippet content for target language")?;

        let label = snippet.title.clone();

        debug!(
            "Creating completion item: query='{}', label='{}', content_preview='{}'",
            query,
            label,
            snippet_content.chars().take(20).collect::<String>()
        );

        let mut completion_item = CompletionItem {
            label: label.clone(),
            kind: Some(CompletionItemKind::SNIPPET),
            detail: Some("bkmr snippet".to_string()),
            documentation: Some(Documentation::String(if snippet_content.len() > 500 {
                format!("{}...", &snippet_content[..500])
            } else {
                snippet_content.clone()
            })),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            filter_text: Some(label.clone()),
            sort_text: Some(label.clone()),
            ..Default::default()
        };

        // Use TextEdit for proper replacement if we have a range
        if let Some(range) = replacement_range {
            completion_item.text_edit = Some(CompletionTextEdit::Edit(TextEdit {
                range,
                new_text: snippet_content,
            }));
            debug!("Set text_edit for range replacement: {:?}", range);
        } else {
            // Fallback to insert_text for backward compatibility
            completion_item.insert_text = Some(snippet_content);
            debug!("Using fallback insert_text (no range available)");
        }

        Ok(completion_item)
    }

    /// Health check for the completion service
    pub async fn health_check(&self) -> Result<()> {
        self.repository
            .health_check()
            .await
            .context("check repository health")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repositories::MockSnippetRepository;
    use tower_lsp::lsp_types::{Position, Range, Url};

    #[tokio::test]
    async fn given_context_with_query_when_getting_completions_then_returns_filtered_items() {
        // Arrange
        let snippet = Snippet::new(
            1,
            "Hello World".to_string(),
            "println!(\"Hello, World!\");".to_string(),
            "Hello world example".to_string(),
            vec!["rust".to_string(), "_snip_".to_string()],
        );

        let repository = Arc::new(
            MockSnippetRepository::new()
                .with_snippets(vec![snippet.clone()])
        );

        let service = CompletionService::new(repository);
        
        let uri = Url::parse("file:///test.rs").expect("parse URI");
        let context = CompletionContext::new(
            uri,
            Position { line: 0, character: 5 },
            Some("rust".to_string()),
        );

        // Act
        let result = service.get_completions(&context).await;

        // Assert
        assert!(result.is_ok());
        let items = result.expect("valid completion items");
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].label, "Hello World");
        assert_eq!(items[0].kind, Some(CompletionItemKind::SNIPPET));
    }

    #[tokio::test]
    async fn given_universal_snippet_when_creating_completion_item_then_translates_content() {
        // Arrange
        let universal_snippet = Snippet::new(
            1,
            "Universal Comment".to_string(),
            "// This is a universal comment".to_string(),
            "Universal snippet".to_string(),
            vec!["universal".to_string(), "_snip_".to_string()],
        );

        let repository = Arc::new(MockSnippetRepository::new());
        let service = CompletionService::new(repository);
        
        let uri = Url::parse("file:///test.py").expect("parse URI");

        // Act
        let result = service.snippet_to_completion_item(
            &universal_snippet,
            "",
            None,
            "python",
            &uri,
        );

        // Assert
        assert!(result.is_ok());
        let item = result.expect("valid completion item");
        
        // Should have translated Rust comment to Python comment
        let insert_text = item.insert_text.expect("insert text");
        assert!(insert_text.contains("# This is a universal comment"));
    }

    #[tokio::test]
    async fn given_completion_item_with_range_when_creating_then_uses_text_edit() {
        // Arrange
        let snippet = Snippet::new(
            1,
            "Test Snippet".to_string(),
            "test content".to_string(),
            "Test description".to_string(),
            vec!["rust".to_string(), "_snip_".to_string()],
        );

        let repository = Arc::new(MockSnippetRepository::new());
        let service = CompletionService::new(repository);
        
        let uri = Url::parse("file:///test.rs").expect("parse URI");
        let range = Range {
            start: Position { line: 0, character: 0 },
            end: Position { line: 0, character: 4 },
        };

        // Act
        let result = service.snippet_to_completion_item(
            &snippet,
            "test",
            Some(range),
            "rust",
            &uri,
        );

        // Assert
        assert!(result.is_ok());
        let item = result.expect("valid completion item");
        
        match item.text_edit {
            Some(CompletionTextEdit::Edit(edit)) => {
                assert_eq!(edit.range, range);
                assert_eq!(edit.new_text, "test content");
            }
            _ => panic!("Expected text edit"),
        }
    }

    #[tokio::test]
    async fn given_healthy_repository_when_health_check_then_returns_ok() {
        // Arrange
        let repository = Arc::new(MockSnippetRepository::new());
        let service = CompletionService::new(repository);

        // Act
        let result = service.health_check().await;

        // Assert
        assert!(result.is_ok());
    }
}