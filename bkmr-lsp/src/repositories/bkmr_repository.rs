use anyhow::{Context, Result};
use async_trait::async_trait;
use tracing::{debug, error, info, instrument};

use crate::domain::{BkmrSnippet, Snippet, SnippetFilter};
use crate::repositories::{RepositoryConfig, SnippetRepository};

/// Repository implementation that uses the bkmr CLI for snippet retrieval
pub struct BkmrRepository {
    config: RepositoryConfig,
}

impl BkmrRepository {
    pub fn new(config: RepositoryConfig) -> Self {
        Self { config }
    }

    /// Build command arguments for bkmr CLI
    fn build_command_args(&self, filter: &SnippetFilter) -> Vec<String> {
        let mut args = vec![
            "search".to_string(),
            "--json".to_string(),
            "--interpolate".to_string(), // Always use interpolation
            "--limit".to_string(),
            filter.max_results.to_string(),
        ];

        // Build FTS query that combines language-specific and universal snippets
        let mut fts_parts = Vec::new();

        // Add language + universal snippet query
        if let Some(snippet_query) = filter.build_fts_query() {
            fts_parts.push(format!("({})", snippet_query));
            debug!("Using snippet query: {}", snippet_query);
        }

        // Add search term if prefix is provided and not empty
        if let Some(ref prefix) = filter.query_prefix {
            if !prefix.trim().is_empty() {
                // Use title prefix search for better snippet matching
                fts_parts.push(format!("metadata:{}*", prefix));
                debug!("Using search prefix: {}", prefix);
            }
        }

        // Combine all FTS parts with AND logic
        if !fts_parts.is_empty() {
            let fts_query = if fts_parts.len() == 1 {
                fts_parts.into_iter().next().expect("single FTS part")
            } else {
                fts_parts.join(" AND ")
            };
            args.push(fts_query);
            debug!("Final FTS query: {}", args.last().expect("FTS query"));
        }

        args
    }

    /// Execute bkmr command and parse output
    #[instrument(skip(self))]
    async fn execute_bkmr_command(&self, args: &[String]) -> Result<Vec<BkmrSnippet>> {
        debug!("Executing bkmr with args: {:?}", args);

        // Add timeout to prevent hanging
        let command_future = tokio::process::Command::new(&self.config.binary_path)
            .args(args)
            .output();

        let output = tokio::time::timeout(
            std::time::Duration::from_secs(self.config.timeout_seconds),
            command_future,
        )
        .await
        .context("execute bkmr command within timeout")?
        .context("spawn bkmr process")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            error!("bkmr command failed with stderr: {}", stderr);
            return Err(anyhow::anyhow!("bkmr command failed: {}", stderr))
                .context("execute bkmr command successfully");
        }

        let stdout_str = String::from_utf8_lossy(&output.stdout);

        if stdout_str.trim().is_empty() {
            debug!("Empty output from bkmr");
            return Ok(Vec::new());
        }

        let snippets: Vec<BkmrSnippet> =
            serde_json::from_str(&stdout_str).context("parse bkmr JSON output")?;

        info!(
            "Successfully fetched {} interpolated snippets",
            snippets.len()
        );
        Ok(snippets)
    }
}

#[async_trait]
impl SnippetRepository for BkmrRepository {
    #[instrument(skip(self))]
    async fn fetch_snippets(&self, filter: &SnippetFilter) -> Result<Vec<Snippet>> {
        let args = self.build_command_args(filter);
        let bkmr_snippets = self
            .execute_bkmr_command(&args)
            .await
            .context("fetch snippets from bkmr CLI")?;

        // Convert BkmrSnippet to domain Snippet
        let snippets: Vec<Snippet> = bkmr_snippets
            .into_iter()
            .map(|bkmr_snippet| bkmr_snippet.into())
            .collect();

        Ok(snippets)
    }

    #[instrument(skip(self))]
    async fn health_check(&self) -> Result<()> {
        debug!("Verifying bkmr availability");

        let command_future = tokio::process::Command::new(&self.config.binary_path)
            .args(["--help"])
            .output();

        let output = tokio::time::timeout(std::time::Duration::from_secs(5), command_future)
            .await
            .context("execute bkmr health check within timeout")?
            .context("spawn bkmr process for health check")?;

        if !output.status.success() {
            return Err(anyhow::anyhow!("bkmr binary is not working properly"))
                .context("verify bkmr binary functionality");
        }

        info!("bkmr binary verified successfully");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn given_basic_filter_when_building_args_then_includes_required_args() {
        // Arrange
        let config = RepositoryConfig::default();
        let repository = BkmrRepository::new(config);
        let filter = SnippetFilter::new(Some("rust".to_string()), None, 25);

        // Act
        let args = repository.build_command_args(&filter);

        // Assert
        assert!(args.contains(&"search".to_string()));
        assert!(args.contains(&"--json".to_string()));
        assert!(args.contains(&"--interpolate".to_string()));
        assert!(args.contains(&"--limit".to_string()));
        assert!(args.contains(&"25".to_string()));
    }

    #[test]
    fn given_filter_with_prefix_when_building_args_then_includes_metadata_search() {
        // Arrange
        let config = RepositoryConfig::default();
        let repository = BkmrRepository::new(config);
        let filter = SnippetFilter::new(Some("rust".to_string()), Some("hello".to_string()), 50);

        // Act
        let args = repository.build_command_args(&filter);

        // Assert
        assert!(args.iter().any(|arg| arg.contains("metadata:hello*")));
    }

    #[test]
    fn given_filter_with_language_when_building_args_then_includes_language_query() {
        // Arrange
        let config = RepositoryConfig::default();
        let repository = BkmrRepository::new(config);
        let filter = SnippetFilter::new(Some("python".to_string()), None, 50);

        // Act
        let args = repository.build_command_args(&filter);

        // Assert
        assert!(
            args.iter()
                .any(|arg| arg.contains("tags:python") && arg.contains("universal"))
        );
    }
}
