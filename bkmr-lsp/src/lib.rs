mod backend;

pub use backend::*;

#[cfg(test)]
mod tests {
    use super::*;
    use tower_lsp::lsp_types::Url;

    #[test]
    fn test_language_info_database() {
        // Test a few key languages from our database
        let rust_info = BkmrLspBackend::get_language_info_static("rust");
        assert_eq!(rust_info.name, "Rust");
        assert_eq!(rust_info.line_comment, Some("//".to_string()));
        assert_eq!(rust_info.block_comment, Some(("/*".to_string(), "*/".to_string())));
        assert_eq!(rust_info.indent_char, "    ");
        assert_eq!(rust_info.indent_width, 4);
        
        let python_info = BkmrLspBackend::get_language_info_static("python");
        assert_eq!(python_info.name, "Python");
        assert_eq!(python_info.line_comment, Some("#".to_string()));
        assert_eq!(python_info.block_comment, Some(("\"\"\"".to_string(), "\"\"\"".to_string())));
        assert_eq!(python_info.indent_char, "    ");
        assert_eq!(python_info.indent_width, 4);
        
        let go_info = BkmrLspBackend::get_language_info_static("go");
        assert_eq!(go_info.name, "Go");
        assert_eq!(go_info.indent_char, "\t");
        assert_eq!(go_info.indent_width, 1);
        
        let html_info = BkmrLspBackend::get_language_info_static("html");
        assert_eq!(html_info.name, "HTML");
        assert_eq!(html_info.line_comment, None);
        assert_eq!(html_info.block_comment, Some(("<!--".to_string(), "-->".to_string())));
        
        // Test unknown language fallback
        let unknown_info = BkmrLspBackend::get_language_info_static("nonexistent");
        assert_eq!(unknown_info.name, "Unknown");
        assert_eq!(unknown_info.line_comment, Some("#".to_string()));
    }

    #[test]
    fn test_lsp_placeholder_processing() {
        // Test template with all supported LSP placeholders
        let universal_template = r#"LSP_COMMENT_LINE This is a line comment
LSP_COMMENT_BLOCK_START
This is a block comment
LSP_COMMENT_BLOCK_END

function example() {
LSP_INDENTconsole.log("Hello world");
LSP_FOLD_START
LSP_INDENTsome code here
LSP_FOLD_END
}

LSP_COMMENT_LINE LSP_FILEPATH"#;

        // Test with different language IDs and verify appropriate replacements
        
        // JavaScript/TypeScript
        let js_url = Url::parse("file:///test/example.js").unwrap();
        let js_result = BkmrLspBackend::process_lsp_placeholders_static(universal_template, "javascript", &js_url);
        assert!(js_result.contains("// This is a line comment"));
        assert!(js_result.contains("/*\nThis is a block comment\n*/"));
        assert!(js_result.contains("    console.log"));  // 4-space indent
        assert!(js_result.contains("{{{"));  // fold markers
        assert!(js_result.contains("// example.js"));
        
        // Python
        let py_url = Url::parse("file:///test/example.py").unwrap();  
        let py_result = BkmrLspBackend::process_lsp_placeholders_static(universal_template, "python", &py_url);
        assert!(py_result.contains("# This is a line comment"));
        assert!(py_result.contains("\"\"\"\nThis is a block comment\n\"\"\""));
        assert!(py_result.contains("    console.log"));  // 4-space indent
        assert!(py_result.contains("# example.py"));
        
        // Rust
        let rs_url = Url::parse("file:///test/example.rs").unwrap();
        let rs_result = BkmrLspBackend::process_lsp_placeholders_static(universal_template, "rust", &rs_url);
        assert!(rs_result.contains("// This is a line comment"));
        assert!(rs_result.contains("/*\nThis is a block comment\n*/"));
        assert!(rs_result.contains("// example.rs"));
        
        // Go (tab indentation)
        let go_url = Url::parse("file:///test/example.go").unwrap();
        let go_result = BkmrLspBackend::process_lsp_placeholders_static(universal_template, "go", &go_url);
        assert!(go_result.contains("// This is a line comment"));
        assert!(go_result.contains("\tconsole.log"));  // tab indent
        assert!(go_result.contains("// example.go"));
        
        // HTML (no line comments)
        let html_url = Url::parse("file:///test/example.html").unwrap();
        let html_result = BkmrLspBackend::process_lsp_placeholders_static(universal_template, "html", &html_url);
        assert!(html_result.contains("<!--  --> This is a line comment"));
        assert!(html_result.contains("<!--\nThis is a block comment\n-->"));
        assert!(html_result.contains("<!--  --> example.html"));
        
        // CSS (only block comments)
        let css_url = Url::parse("file:///test/example.css").unwrap();
        let css_result = BkmrLspBackend::process_lsp_placeholders_static(universal_template, "css", &css_url);
        assert!(css_result.contains("/*  */ This is a line comment"));
        assert!(css_result.contains("/*\nThis is a block comment\n*/"));
        
        // Unknown language (fallback to hash comments)
        let unknown_url = Url::parse("file:///test/example.unknown").unwrap();
        let unknown_result = BkmrLspBackend::process_lsp_placeholders_static(universal_template, "unknown", &unknown_url);
        assert!(unknown_result.contains("# This is a line comment"));
        assert!(unknown_result.contains("# example.unknown"));
    }

    #[test]
    fn test_empty_and_edge_cases() {
        let url = Url::parse("file:///test/example.rs").unwrap();
        
        // Empty string
        let result = BkmrLspBackend::process_lsp_placeholders_static("", "rust", &url);
        assert_eq!(result, "");
        
        // No placeholders
        let no_placeholders = "Just normal text here";
        let result = BkmrLspBackend::process_lsp_placeholders_static(no_placeholders, "rust", &url);
        assert_eq!(result, no_placeholders);
        
        // Only whitespace
        let whitespace = "   \n\t  \n";
        let result = BkmrLspBackend::process_lsp_placeholders_static(whitespace, "rust", &url);
        assert_eq!(result, whitespace);
        
        // Partial placeholder matches (should not be replaced)
        let partial = "LSP_COMMENT text LSP_INVALID_PLACEHOLDER more text";
        let result = BkmrLspBackend::process_lsp_placeholders_static(partial, "rust", &url);
        assert_eq!(result, partial);  // Should remain unchanged
    }
}
