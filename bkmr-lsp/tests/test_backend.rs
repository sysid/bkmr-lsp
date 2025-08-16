use bkmr_lsp::{BkmrConfig, BkmrSnippet};

mod test_utils;
use test_utils::SnippetBuilder;

#[test_log::test(tokio::test)]
async fn test_config_defaults() {
    let config = BkmrConfig::default();
    assert_eq!(config.bkmr_binary, "bkmr");
    assert_eq!(config.max_completions, 50);
}

#[test_log::test(tokio::test)]
async fn test_snippet_deserialization() {
    let json = r#"{
        "id": 123,
        "title": "Test Snippet",
        "url": "console.log('hello');",
        "description": "A test snippet",
        "tags": ["javascript", "test"],
        "access_count": 5
    }"#;

    let snippet: BkmrSnippet = serde_json::from_str(json).unwrap();
    assert_eq!(snippet.id, 123);
    assert_eq!(snippet.title, "Test Snippet");
    assert_eq!(snippet.url, "console.log('hello');");
    assert_eq!(snippet.description, "A test snippet");
    assert_eq!(snippet.tags, vec!["javascript", "test"]);
    assert_eq!(snippet.access_count, 5);
}

#[test_log::test(tokio::test)]
async fn test_snippet_default_access_count() {
    let json = r#"{
        "id": 456,
        "title": "Test Snippet 2",
        "url": "println!(\"hello\");",
        "description": "A rust snippet",
        "tags": ["rust"]
    }"#;

    let snippet: BkmrSnippet = serde_json::from_str(json).unwrap();
    assert_eq!(snippet.access_count, 0); // Default value
}

#[test_log::test(tokio::test)]
async fn test_snippet_builder() {
    let snippet = SnippetBuilder::new()
        .with_id(42)
        .with_title("Test Function")
        .with_content("fn test() {}")
        .with_language("rust")
        .with_snippet_tag()
        .build();

    assert_eq!(snippet.id, 42);
    assert_eq!(snippet.title, "Test Function");
    assert_eq!(snippet.url, "fn test() {}");
    assert!(snippet.tags.contains(&"rust".to_string()));
    assert!(snippet.tags.contains(&"_snip_".to_string()));
}

#[test_log::test]
fn test_snippet_tags() {
    let mut snippet = SnippetBuilder::new().build();
    
    // Test initial state
    assert_eq!(snippet.tags, vec!["test"]);

    // Test with multiple tags
    snippet = SnippetBuilder::new()
        .with_tags(vec!["rust", "function"])
        .with_snippet_tag()
        .with_universal_tag()
        .build();

    assert!(snippet.tags.contains(&"rust".to_string()));
    assert!(snippet.tags.contains(&"function".to_string()));
    assert!(snippet.tags.contains(&"_snip_".to_string()));
    assert!(snippet.tags.contains(&"universal".to_string()));
}
