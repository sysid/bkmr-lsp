mod backend;

pub use backend::*;

#[cfg(test)]
mod tests {
    use super::*;
    use tower_lsp::lsp_types::Url;

    #[test]
    fn test_universal_snippet_tag_detection() {
        // Test that snippets with "universal" tag are identified correctly
        let universal_snippet = BkmrSnippet {
            id: 1,
            title: "Test Universal Snippet".to_string(),
            url: "// This is a test".to_string(),
            description: "Test description".to_string(),
            tags: vec!["universal".to_string(), "test".to_string()],
            access_count: 0,
        };
        
        let regular_snippet = BkmrSnippet {
            id: 2,
            title: "Test Regular Snippet".to_string(),
            url: "// This is a test".to_string(),
            description: "Test description".to_string(),
            tags: vec!["rust".to_string(), "test".to_string()],
            access_count: 0,
        };
        
        // Test tag detection
        assert!(universal_snippet.tags.contains(&"universal".to_string()));
        assert!(!regular_snippet.tags.contains(&"universal".to_string()));
    }

    #[test]
    fn test_rust_comment_translation() {
        let uri = Url::parse("file:///test/example.py").unwrap();
        
        // Test line comments
        let rust_content = r#"// This is a line comment
    // Indented comment
let x = 5; // End of line comment"#;
        
        let python_result = BkmrLspBackend::translate_rust_patterns_static(rust_content, "python", &uri);
        assert!(python_result.contains("# This is a line comment"));
        assert!(python_result.contains("    # Indented comment"));
        assert!(python_result.contains("let x = 5; # End of line comment"));
        
        // Test with HTML (no line comments)
        let html_result = BkmrLspBackend::translate_rust_patterns_static(rust_content, "html", &uri);
        assert!(html_result.contains("<!-- This is a line comment -->"));
        assert!(html_result.contains("  <!-- Indented comment -->"));  // HTML uses 2 spaces
        assert!(html_result.contains("let x = 5; <!-- End of line comment -->"));
    }

    #[test]
    fn test_rust_block_comment_translation() {
        let uri = Url::parse("file:///test/example.py").unwrap();
        
        let rust_content = r#"/* This is a block comment */
/*
Multi-line
block comment
*/"#;
        
        let python_result = BkmrLspBackend::translate_rust_patterns_static(rust_content, "python", &uri);
        assert!(python_result.contains("\"\"\" This is a block comment \"\"\""));
        assert!(python_result.contains("\"\"\"\nMulti-line\nblock comment\n\"\"\""));
        
        let html_result = BkmrLspBackend::translate_rust_patterns_static(rust_content, "html", &uri);
        assert!(html_result.contains("<!-- This is a block comment -->"));
        assert!(html_result.contains("<!--\nMulti-line\nblock comment\n-->"));
    }

    #[test]
    fn test_rust_indentation_translation() {
        let uri = Url::parse("file:///test/example.go").unwrap();
        
        let rust_content = r#"fn example() {
    let x = 5;
        let y = 10;
            let z = 15;
}"#;
        
        // Go uses tabs
        let go_result = BkmrLspBackend::translate_rust_patterns_static(rust_content, "go", &uri);
        assert!(go_result.contains("fn example() {"));
        assert!(go_result.contains("\tlet x = 5;"));
        assert!(go_result.contains("\t\tlet y = 10;"));
        assert!(go_result.contains("\t\t\tlet z = 15;"));
        
        // JavaScript uses 2 spaces
        let js_result = BkmrLspBackend::translate_rust_patterns_static(rust_content, "javascript", &uri);
        assert!(js_result.contains("  let x = 5;"));
        assert!(js_result.contains("    let y = 10;"));
        assert!(js_result.contains("      let z = 15;"));
    }

    #[test]
    fn test_filename_replacement() {
        let uri = Url::parse("file:///path/to/example.rs").unwrap();
        
        let content = "// File: {{ filename }}";
        let result = BkmrLspBackend::translate_rust_patterns_static(content, "rust", &uri);
        assert!(result.contains("// File: example.rs"));
    }

    #[test]
    fn test_mixed_pattern_translation() {
        let uri = Url::parse("file:///test/example.py").unwrap();
        
        let rust_content = r#"// Function: {{ function_name }}
// File: {{ filename }}
fn {{ function_name }}() {
    // TODO: implement
    /* Block comment here */
        let value = "hello";
}"#;
        
        let python_result = BkmrLspBackend::translate_rust_patterns_static(rust_content, "python", &uri);
        
        // Check comment translation
        assert!(python_result.contains("# Function: {{ function_name }}"));
        assert!(python_result.contains("# File: example.py"));
        assert!(python_result.contains("    # TODO: implement"));
        assert!(python_result.contains("\"\"\" Block comment here \"\"\""));
        
        // Check that bkmr templates are preserved
        assert!(python_result.contains("{{ function_name }}"));
        
        // Check indentation (Python uses 4 spaces like Rust, so no change)
        assert!(python_result.contains("        let value = \"hello\";"));
    }

    #[test]
    fn test_language_info_retrieval() {
        let rust_info = BkmrLspBackend::get_language_info_static("rust");
        assert_eq!(rust_info.line_comment, Some("//".to_string()));
        assert_eq!(rust_info.block_comment, Some(("/*".to_string(), "*/".to_string())));
        assert_eq!(rust_info.indent_char, "    ");
        
        let python_info = BkmrLspBackend::get_language_info_static("python");
        assert_eq!(python_info.line_comment, Some("#".to_string()));
        assert_eq!(python_info.block_comment, Some(("\"\"\"".to_string(), "\"\"\"".to_string())));
        assert_eq!(python_info.indent_char, "    ");
        
        let go_info = BkmrLspBackend::get_language_info_static("go");
        assert_eq!(go_info.line_comment, Some("//".to_string()));
        assert_eq!(go_info.indent_char, "\t");
        
        let html_info = BkmrLspBackend::get_language_info_static("html");
        assert_eq!(html_info.line_comment, None);
        assert_eq!(html_info.block_comment, Some(("<!--".to_string(), "-->".to_string())));
        assert_eq!(html_info.indent_char, "  ");
    }

    #[test]
    fn test_edge_cases() {
        let uri = Url::parse("file:///test/example.py").unwrap();
        
        // Empty content
        let result = BkmrLspBackend::translate_rust_patterns_static("", "python", &uri);
        assert_eq!(result, "");
        
        // No Rust patterns
        let no_patterns = "Just plain text here";
        let result = BkmrLspBackend::translate_rust_patterns_static(no_patterns, "python", &uri);
        assert_eq!(result, no_patterns);
        
        // Comments in strings (should not be translated)
        let string_comments = r#"let url = "https://example.com"; // Real comment"#;
        let result = BkmrLspBackend::translate_rust_patterns_static(string_comments, "python", &uri);
        assert!(result.contains("\"https://example.com\""));
        assert!(result.contains("# Real comment"));
        
        // Multiple line patterns
        let multi_line = "//Comment1\n//Comment2\n    //Comment3";
        let result = BkmrLspBackend::translate_rust_patterns_static(multi_line, "python", &uri);
        assert!(result.contains("# Comment1"));
        assert!(result.contains("# Comment2"));
        assert!(result.contains("    # Comment3"));
    }

    #[test]
    fn test_fts_query_builder() {
        // Test with specific language
        let query = BkmrLspBackend::build_snippet_fts_query_for_test(Some("markdown"));
        assert_eq!(
            query,
            Some(r#"(tags:markdown AND tags:"_snip_") OR (tags:universal AND tags:"_snip_")"#.to_string())
        );
        
        // Test with rust language
        let query = BkmrLspBackend::build_snippet_fts_query_for_test(Some("rust"));
        assert_eq!(
            query,
            Some(r#"(tags:rust AND tags:"_snip_") OR (tags:universal AND tags:"_snip_")"#.to_string())
        );
        
        // Test with empty language
        let query = BkmrLspBackend::build_snippet_fts_query_for_test(Some(""));
        assert_eq!(query, Some(r#"tags:"_snip_""#.to_string()));
        
        // Test with whitespace-only language
        let query = BkmrLspBackend::build_snippet_fts_query_for_test(Some("   "));
        assert_eq!(query, Some(r#"tags:"_snip_""#.to_string()));
        
        // Test with None language
        let query = BkmrLspBackend::build_snippet_fts_query_for_test(None);
        assert_eq!(query, Some(r#"tags:"_snip_""#.to_string()));
        
        // Test with complex language names
        let query = BkmrLspBackend::build_snippet_fts_query_for_test(Some("typescript"));
        assert_eq!(
            query,
            Some(r#"(tags:typescript AND tags:"_snip_") OR (tags:universal AND tags:"_snip_")"#.to_string())
        );
    }
}
