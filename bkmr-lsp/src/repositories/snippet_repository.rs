use anyhow::Result;
use async_trait::async_trait;

use crate::domain::{Snippet, SnippetFilter};

/// Repository trait for snippet retrieval operations
#[async_trait]
pub trait SnippetRepository: Send + Sync {
    /// Fetch snippets based on the provided filter
    async fn fetch_snippets(&self, filter: &SnippetFilter) -> Result<Vec<Snippet>>;

    /// Check if the repository is available and properly configured
    async fn health_check(&self) -> Result<()>;
}

/// Configuration for snippet repositories
#[derive(Debug, Clone)]
pub struct RepositoryConfig {
    pub binary_path: String,
    pub max_results: usize,
    pub timeout_seconds: u64,
}

impl Default for RepositoryConfig {
    fn default() -> Self {
        Self {
            binary_path: "bkmr".to_string(),
            max_results: 50,
            timeout_seconds: 10,
        }
    }
}
