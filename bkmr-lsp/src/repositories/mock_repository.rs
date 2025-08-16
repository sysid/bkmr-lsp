use anyhow::Result;
use async_trait::async_trait;

use crate::domain::{Snippet, SnippetFilter};
use crate::repositories::SnippetRepository;

/// Mock repository implementation for testing
pub struct MockSnippetRepository {
    pub snippets: Vec<Snippet>,
    pub health_check_result: Result<(), anyhow::Error>,
}

impl MockSnippetRepository {
    pub fn new() -> Self {
        Self {
            snippets: Vec::new(),
            health_check_result: Ok(()),
        }
    }

    pub fn with_snippets(mut self, snippets: Vec<Snippet>) -> Self {
        self.snippets = snippets;
        self
    }

    pub fn with_health_check_error(mut self, error: anyhow::Error) -> Self {
        self.health_check_result = Err(error);
        self
    }
}

impl Default for MockSnippetRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SnippetRepository for MockSnippetRepository {
    async fn fetch_snippets(&self, filter: &SnippetFilter) -> Result<Vec<Snippet>> {
        let mut filtered_snippets = self.snippets.clone();

        // Apply language filter
        if let Some(ref language) = filter.language_id {
            filtered_snippets.retain(|snippet| {
                snippet.has_language(language) || snippet.is_universal()
            });
        }

        // Apply prefix filter
        if let Some(ref prefix) = filter.query_prefix {
            let prefix_lower = prefix.to_lowercase();
            filtered_snippets.retain(|snippet| {
                snippet.title.to_lowercase().contains(&prefix_lower) ||
                snippet.description.to_lowercase().contains(&prefix_lower)
            });
        }

        // Apply limit
        filtered_snippets.truncate(filter.max_results);

        Ok(filtered_snippets)
    }

    async fn health_check(&self) -> Result<()> {
        match &self.health_check_result {
            Ok(()) => Ok(()),
            Err(e) => Err(anyhow::anyhow!("{}", e)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn given_empty_repository_when_fetching_snippets_then_returns_empty_list() {
        // Arrange
        let repository = MockSnippetRepository::new();
        let filter = SnippetFilter::default();

        // Act
        let result = repository.fetch_snippets(&filter).await;

        // Assert
        assert!(result.is_ok());
        assert!(result.expect("empty snippet list").is_empty());
    }

    #[tokio::test]
    async fn given_snippets_with_language_filter_when_fetching_then_returns_matching_snippets() {
        // Arrange
        let rust_snippet = Snippet::new(
            1,
            "Rust Function".to_string(),
            "fn test() {}".to_string(),
            "A Rust function".to_string(),
            vec!["rust".to_string(), "_snip_".to_string()],
        );

        let python_snippet = Snippet::new(
            2,
            "Python Function".to_string(),
            "def test(): pass".to_string(),
            "A Python function".to_string(),
            vec!["python".to_string(), "_snip_".to_string()],
        );

        let universal_snippet = Snippet::new(
            3,
            "Universal Snippet".to_string(),
            "// TODO: implement".to_string(),
            "Universal snippet".to_string(),
            vec!["universal".to_string(), "_snip_".to_string()],
        );

        let repository = MockSnippetRepository::new()
            .with_snippets(vec![rust_snippet.clone(), python_snippet, universal_snippet.clone()]);

        let filter = SnippetFilter::new(Some("rust".to_string()), None, 50);

        // Act
        let result = repository.fetch_snippets(&filter).await;

        // Assert
        assert!(result.is_ok());
        let snippets = result.expect("filtered snippets");
        assert_eq!(snippets.len(), 2); // rust snippet + universal snippet
        assert!(snippets.iter().any(|s| s.id == rust_snippet.id));
        assert!(snippets.iter().any(|s| s.id == universal_snippet.id));
    }

    #[tokio::test]
    async fn given_snippets_with_prefix_filter_when_fetching_then_returns_matching_snippets() {
        // Arrange
        let hello_snippet = Snippet::new(
            1,
            "Hello World".to_string(),
            "println!(\"Hello, World!\");".to_string(),
            "Hello world example".to_string(),
            vec!["rust".to_string(), "_snip_".to_string()],
        );

        let goodbye_snippet = Snippet::new(
            2,
            "Goodbye".to_string(),
            "println!(\"Goodbye!\");".to_string(),
            "Goodbye example".to_string(),
            vec!["rust".to_string(), "_snip_".to_string()],
        );

        let repository = MockSnippetRepository::new()
            .with_snippets(vec![hello_snippet.clone(), goodbye_snippet]);

        let filter = SnippetFilter::new(None, Some("hello".to_string()), 50);

        // Act
        let result = repository.fetch_snippets(&filter).await;

        // Assert
        assert!(result.is_ok());
        let snippets = result.expect("filtered snippets");
        assert_eq!(snippets.len(), 1);
        assert_eq!(snippets[0].id, hello_snippet.id);
    }

    #[tokio::test]
    async fn given_healthy_repository_when_health_check_then_returns_ok() {
        // Arrange
        let repository = MockSnippetRepository::new();

        // Act
        let result = repository.health_check().await;

        // Assert
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn given_unhealthy_repository_when_health_check_then_returns_error() {
        // Arrange
        let repository = MockSnippetRepository::new()
            .with_health_check_error(anyhow::anyhow!("Health check failed"));

        // Act
        let result = repository.health_check().await;

        // Assert
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Health check failed"));
    }
}