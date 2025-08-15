# Universal Snippets with LSP Placeholders

This document explains the universal snippet functionality in bkmr-lsp, which allows creating language-agnostic snippets that automatically adapt to the current file type.

## Overview

Universal snippets use special `LSP_` prefixed placeholders that are replaced with language-specific syntax when the snippet is inserted. This provides similar functionality to UltiSnips' Python functions, but without requiring changes to the bkmr CLI.

## LSP Placeholder Syntax

All LSP placeholders follow the pattern `LSP_<FUNCTION>` and are case-sensitive. They are replaced by bkmr-lsp during snippet completion based on the current file's language ID.

### Available Placeholders

| Placeholder | Description | Example (Rust) | Example (Python) | Example (HTML) |
|-------------|-------------|----------------|------------------|----------------|
| `LSP_COMMENT_LINE` | Single line comment syntax | `//` | `#` | `<!--  -->` |
| `LSP_COMMENT_BLOCK_START` | Block comment start | `/*` | `"""` | `<!--` |
| `LSP_COMMENT_BLOCK_END` | Block comment end | `*/` | `"""` | `-->` |
| `LSP_INDENT` | Language-specific indentation | `    ` (4 spaces) | `    ` (4 spaces) | `    ` (4 spaces) |
| `LSP_FOLD_START` | Fold marker start | `{{{` | `{{{` | `{{{` |
| `LSP_FOLD_END` | Fold marker end | `}}}` | `}}}` | `}}}` |
| `LSP_FILEPATH` | Current file name | `main.rs` | `script.py` | `index.html` |

### Language Support

bkmr-lsp includes built-in support for 20+ languages with appropriate comment syntax, indentation, and fold markers:

**C-style languages:** Rust, JavaScript, TypeScript, Go, Java, C, C++, Swift, Kotlin, PHP  
**Hash-comment languages:** Python, Shell (bash/zsh), YAML  
**Markup languages:** HTML, XML, Markdown  
**Style languages:** CSS, SCSS  
**Other:** Ruby, Vim script, JSON  

Languages without line comments (like HTML, CSS) use block comment syntax for `LSP_COMMENT_LINE`.

## Example Universal Snippets

### Function Template

```
LSP_COMMENT_LINE Function: {{ function_name }}
LSP_COMMENT_LINE Description: {{ description }}
LSP_COMMENT_LINE Author: {{ author }}

function {{ function_name }}() {
LSP_INDENTLSp_COMMENT_LINE TODO: implement {{ function_name }}
LSP_INDENTreturn {{ default_value }};
}
```

**Result in Rust:**
```rust
// Function: hello_world
// Description: Says hello
// Author: Developer

function hello_world() {
    // TODO: implement hello_world
    return "Hello";
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

### File Header Template

```
LSP_COMMENT_LINE LSP_FILEPATH
LSP_COMMENT_LINE 
LSP_COMMENT_LINE Created: {{ current_date }}
LSP_COMMENT_LINE Author: {{ author }}
LSP_COMMENT_LINE Description: {{ description }}

LSP_COMMENT_BLOCK_START
{{ detailed_description }}

Example usage:
{{ example_code }}
LSP_COMMENT_BLOCK_END

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

**Result in HTML:**
```html
<!--  --> index.html
<!--  --> 
<!--  --> Created: 2024-01-15
<!--  --> Author: Developer
<!--  --> Description: Example page

<!--
A comprehensive example page showing best practices.

Example usage:
<script src="example.js"></script>
-->

<!-- main content here -->
```

### Class Template with Folds

```
LSP_COMMENT_LINE Class: {{ class_name }}
class {{ class_name }} {
LSP_FOLD_START
LSP_INDENTLSp_COMMENT_LINE Constructor
LSP_INDENTconstructor({{ parameters }}) {
LSP_INDENTLSP_INDENTthis.{{ property }} = {{ value }};
LSP_INDENT}

LSP_INDENTLSp_COMMENT_LINE Methods
LSP_INDENT{{ method_definitions }}
LSP_FOLD_END
}
```

**Result in TypeScript:**
```typescript
// Class: UserService
class UserService {
{{{
    // Constructor
    constructor(apiUrl: string) {
        this.apiUrl = apiUrl;
    }

    // Methods
    async getUser(id: number) { ... }
}}}
}
```

### Error Handling Template

```
LSP_COMMENT_LINE Error handling for {{ operation }}
try {
LSP_INDENTLSp_COMMENT_LINE Attempt {{ operation }}
LSP_INDENT{{ operation_code }}
} catch ({{ error_variable }}) {
LSP_INDENTLSp_COMMENT_LINE Handle error: {{ error_type }}
LSP_INDENTLSp_COMMENT_LINE TODO: Add proper error handling
LSP_INDENTconsole.error('{{ operation }} failed:', {{ error_variable }});
LSP_INDENTthrow {{ error_variable }};
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

### 2. Use LSP Placeholders for Language-Specific Elements

Replace language-specific syntax with LSP placeholders:
- Comment syntax → `LSP_COMMENT_LINE`, `LSP_COMMENT_BLOCK_*`
- Indentation → `LSP_INDENT`
- File references → `LSP_FILEPATH`

### 3. Keep bkmr Template Variables

Continue using bkmr's `{{ variable }}` syntax for user input:
- `{{ function_name }}`
- `{{ description }}`  
- `{{ author }}`
- `{{ current_date }}`

### 4. Test Across Languages

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
3. **bkmr-lsp** processes `LSP_` placeholders based on file language ID
4. **LSP client** receives final processed snippet for insertion

### Language Detection

Language ID is determined by:
1. LSP client's document language identifier
2. File extension mapping (fallback)
3. Default to "unknown" (uses hash comments)

### Placeholder Replacement

- Exact string replacement (case-sensitive)
- No regex or partial matching
- Processes in order: comments, indentation, folds, filepath
- Unknown placeholders remain unchanged

## Best Practices

1. **Test thoroughly** across target languages
2. **Keep it simple** - complex logic may not translate well
3. **Document expectations** in snippet descriptions
4. **Use descriptive variable names** in bkmr templates
5. **Consider indentation** when placing `LSP_INDENT`
6. **Be consistent** with placeholder usage across snippet collections

## Troubleshooting

### Placeholders Not Replaced

- Check exact spelling (case-sensitive)
- Verify language ID detection
- Ensure bkmr-lsp version supports universal snippets

### Incorrect Language Detection

- Check file extension mapping in language database
- LSP client may be sending incorrect language ID
- Use debug logs to verify detected language

### Malformed Output

- Review placeholder placement in template
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
```
LSP_COMMENT_LINE TODO: Add implementation
```

The LSP placeholder approach is more declarative and doesn't require scripting knowledge, while providing the same language-adaptation functionality.