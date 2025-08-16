# Universal Snippets with Natural Rust Syntax

This document explains the universal snippet functionality in bkmr-lsp, which allows creating language-agnostic snippets that automatically adapt to the current file type.

## Overview

Universal snippets use natural Rust syntax as a reference format that gets automatically translated to the target language when the snippet is inserted. This provides a more ergonomic authoring experience compared to explicit placeholders, while maintaining the same language-adaptation functionality.

## Natural Rust Syntax Translation

Universal snippets are written using standard Rust syntax, which gets automatically translated to the target language during snippet insertion. This approach is more natural and doesn't require learning special placeholder syntax.

### Supported Rust Patterns

| Rust Pattern | Description | Example (Python) | Example (HTML) |
|-------------|-------------|------------------|----------------|
| `// comment` | Line comments | `# comment` | `<!-- comment -->` |
| `/* comment */` | Block comments | `""" comment """` | `<!-- comment -->` |
| `    ` (4 spaces) | Indentation | `    ` (4 spaces) | `  ` (2 spaces) |
| `{{ filename }}` | File name template | Automatically replaced with current file name |

### Language Support

bkmr-lsp includes built-in support for 20+ languages with appropriate comment syntax and indentation:

**C-style languages:** Rust, JavaScript, TypeScript, Go, Java, C, C++, Swift, Kotlin, PHP  
**Hash-comment languages:** Python, Shell (bash/zsh), YAML  
**Markup languages:** HTML, XML, Markdown  
**Style languages:** CSS, SCSS  
**Other:** Ruby, Vim script, JSON  

Languages without line comments (like HTML, CSS) automatically convert Rust line comments (`//`) to block comment equivalents.

## Example Universal Snippets

### Function Template

```rust
// Function: {{ function_name }}
// Description: {{ description }}
// Author: {{ author }}

function {{ function_name }}() {
    // TODO: implement {{ function_name }}
    return {{ default_value }};
}
```

**Result in Python:**
```python
# Function: hello_world
# Description: Says hello
# Author: Developer

function hello_world() {
    # TODO: implement hello_world
    return "Hello";
}
```

**Result in HTML:**
```html
<!-- Function: hello_world -->
<!-- Description: Says hello -->
<!-- Author: Developer -->

function hello_world() {
  <!-- TODO: implement hello_world -->
  return "Hello";
}
```

### File Header Template

```rust
// {{ filename }}
// 
// Created: {{ current_date }}
// Author: {{ author }}
// Description: {{ description }}

/*
{{ detailed_description }}

Example usage:
{{ example_code }}
*/

{{ main_content }}
```

**Result in JavaScript:**
```javascript
// example.js
// 
// Created: 2024-01-15
// Author: Developer
// Description: Example module

/*
A comprehensive example module showing best practices.

Example usage:
const example = require('./example');
*/

// main content here
```

**Result in Python:**
```python
# example.py
# 
# Created: 2024-01-15
# Author: Developer
# Description: Example module

"""
A comprehensive example module showing best practices.

Example usage:
import example
"""

# main content here
```

### Simple Error Handling Template

```rust
// Error handling for {{ operation }}
try {
    // Attempt {{ operation }}
    {{ operation_code }}
} catch ({{ error_variable }}) {
    // Handle error: {{ error_type }}
    // TODO: Add proper error handling
    console.error('{{ operation }} failed:', {{ error_variable }});
    throw {{ error_variable }};
}
```

**Result in Python:**
```python
# Error handling for data_fetch
try {
    # Attempt data_fetch
    data = fetch_user_data()
} catch (error) {
    # Handle error: NetworkError
    # TODO: Add proper error handling
    console.error('data_fetch failed:', error);
    throw error;
}
```

## Creating Universal Snippets

### 1. Design Language-Agnostic Logic

Focus on programming concepts that translate across languages:
- Function/method structure
- Error handling patterns  
- Documentation headers
- Common algorithms
- Design patterns

### 2. Use Natural Rust Syntax

Write your snippets using standard Rust syntax:
- Line comments: `// comment text`
- Block comments: `/* comment text */`
- Indentation: `    ` (4 spaces)
- File references: `{{ filename }}`

### 3. Keep bkmr Template Variables

Continue using bkmr's `{{ variable }}` syntax for user input:
- `{{ function_name }}`
- `{{ description }}`  
- `{{ author }}`
- `{{ current_date }}`

### 4. Tag as Universal

Make sure to tag your snippets with `universal` so they get processed:
```bash
bkmr add -t universal -t rust my_snippet.rs
```

### 5. Test Across Languages

Verify your universal snippets work correctly in different file types by testing with various language IDs.

## Advanced Patterns

### Conditional Logic (Future Enhancement)

While not currently supported, future versions might include conditional placeholders:

```
LSP_IF_LANGUAGE(rust,go)
LSP_COMMENT_LINE This is for compiled languages
LSP_ENDIF

LSP_IF_LANGUAGE(python,javascript)
LSP_COMMENT_LINE This is for interpreted languages  
LSP_ENDIF
```

### Nested Templates

You can nest LSP placeholders for complex structures:

```
LSP_COMMENT_BLOCK_START
Module: {{ module_name }}

LSP_IF_HAS_DEPENDENCIES
Dependencies:
{{ #dependencies }}
LSP_COMMENT_LINE - {{ name }}: {{ version }}
{{ /dependencies }}
LSP_ENDIF
LSP_COMMENT_BLOCK_END
```

## Implementation Details

### Processing Order

1. **bkmr CLI** processes `{{ variable }}` templates with `--interpolate` flag
2. **bkmr-lsp** receives interpolated content 
3. **bkmr-lsp** detects snippets tagged with "universal"
4. **bkmr-lsp** translates Rust syntax patterns based on target language ID
5. **LSP client** receives final processed snippet for insertion

### Language Detection

Language ID is determined by:
1. LSP client's document language identifier
2. File extension mapping (fallback)
3. Default to "unknown" (uses hash comments)

### Pattern Translation

- Regex-based replacement with multi-line support
- Processes in order: line comments, block comments, indentation, filename
- Only snippets tagged "universal" are processed
- Regular snippets remain unchanged

## Best Practices

1. **Test thoroughly** across target languages
2. **Keep it simple** - complex logic may not translate well
3. **Document expectations** in snippet descriptions
4. **Use descriptive variable names** in bkmr templates
5. **Use standard Rust formatting** - proper spacing and indentation
6. **Tag consistently** - always use "universal" tag for cross-language snippets

## Troubleshooting

### Patterns Not Translated

- Verify snippet is tagged with "universal"
- Check language ID detection
- Ensure bkmr-lsp version supports Rust pattern translation

### Incorrect Language Detection

- Check file extension mapping in language database
- LSP client may be sending incorrect language ID
- Use debug logs to verify detected language

### Malformed Output

- Review Rust syntax in template (proper spacing, comment format)
- Test edge cases (empty content, special characters)
- Verify bkmr template syntax doesn't conflict

## Migration from UltiSnips

If migrating from UltiSnips Python functions:

**UltiSnips:**
```python
def get_comment_char():
    return "//" if vim.eval("&ft") in ["cpp", "java"] else "#"
```

**Universal Snippet:**
```rust
// TODO: Add implementation
```

The natural Rust syntax approach is more declarative and doesn't require scripting knowledge, while providing the same language-adaptation functionality. Just write your snippet in Rust syntax and tag it as "universal".