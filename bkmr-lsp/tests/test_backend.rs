use bkmr_lsp::{BkmrConfig, BkmrSnippet};

#[tokio::test]
async fn test_config_defaults() {
    let config = BkmrConfig::default();
    assert_eq!(config.bkmr_binary, "bkmr");
    assert_eq!(config.max_completions, 50);
}

#[tokio::test]
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

#[tokio::test]
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
