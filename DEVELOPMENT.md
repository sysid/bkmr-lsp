# Development Guide

## Architecture Overview

The bkmr-lsp project follows **Clean Architecture principles** with clear separation of concerns, dependency inversion, and layered design. This architecture provides maintainability, testability, and flexibility.

### Clean Architecture Layers

#### Domain Layer (`src/domain/`)
Pure business logic with no external dependencies:

- **`snippet.rs`**: Core `Snippet` domain model and `BkmrSnippet` compatibility type
- **`language.rs`**: `LanguageInfo` and `LanguageRegistry` for language-specific information
- **`completion.rs`**: `CompletionQuery`, `CompletionContext`, and `SnippetFilter` models

#### Repository Layer (`src/repositories/`)
Data access abstraction using the Repository pattern:

- **`snippet_repository.rs`**: `SnippetRepository` trait defining data access interface
- **`bkmr_repository.rs`**: `BkmrRepository` implementing actual bkmr CLI integration
- **`mock_repository.rs`**: `MockSnippetRepository` for testing without external dependencies

#### Service Layer (`src/services/`)
Business logic orchestration and domain operations:

- **`document_service.rs`**: `DocumentService` managing document state and query extraction
- **`completion_service.rs`**: `CompletionService` handling completion logic with repository injection
- **`language_translator.rs`**: `LanguageTranslator` converting Rust patterns to target languages
- **`command_service.rs`**: `CommandService` handling LSP command execution

#### Infrastructure Layer (`src/backend.rs`)
LSP protocol implementation and external integrations:

- **`BkmrLspBackend`**: Main LSP server with dependency injection
- **`BkmrConfig`**: Configuration management
- **LSP Protocol Handlers**: `initialize`, `completion`, `execute_command`, etc.

### Key Architecture Principles

#### Dependency Inversion
- Services depend on repository traits, not concrete implementations
- Enables easy testing with mock repositories
- Allows swapping implementations without changing business logic

#### Error Handling
- Consistent use of `anyhow::Result` with `.context()` for error chain building
- Prefer `.expect()` over `.unwrap()` with descriptive messages
- Structured error propagation through all layers

#### Async/Await
- Full tokio async support throughout the stack
- Non-blocking I/O for external CLI calls and LSP communication
- Proper async trait usage with `#[async_trait]`

#### Dependency Injection
- Constructor injection pattern for services
- Repository instances injected into services at creation time
- Test-friendly design with configurable dependencies

### File Structure

```
bkmr-lsp/                          # Main Rust project
├── src/                           # Source code
│   ├── main.rs                    # Server entry point
│   ├── backend.rs                 # LSP implementation (Infrastructure layer)
│   ├── lib.rs                     # Module exports
│   ├── domain/                    # Domain layer (business models)
│   │   ├── mod.rs                 # Domain module exports
│   │   ├── snippet.rs             # Snippet domain model
│   │   ├── language.rs            # Language information registry
│   │   └── completion.rs          # Completion query models
│   ├── repositories/              # Repository layer (data access)
│   │   ├── mod.rs                 # Repository module exports
│   │   ├── snippet_repository.rs  # Repository trait definition
│   │   ├── bkmr_repository.rs     # bkmr CLI implementation
│   │   └── mock_repository.rs     # Testing implementation
│   └── services/                  # Service layer (business logic)
│       ├── mod.rs                 # Service module exports
│       ├── document_service.rs    # Document state management
│       ├── completion_service.rs  # Completion orchestration
│       ├── language_translator.rs # Rust pattern translation
│       └── command_service.rs     # LSP command handling
├── tests/                         # Integration and unit tests
├── Cargo.toml                     # Rust project config
└── target/                        # Build artifacts

vim-bkmr-lsp/                      # Vim plugin
├── plugin/bkmr_lsp.vim           # Vim integration
└── README.md                      # Plugin documentation

scripts/                           # Development tools
├── test_lsp.py                    # Python LSP test client
├── test_lsp.sh                    # Shell LSP testing
└── test_lsp2.sh                   # Additional LSP tests
```

### Implementation Patterns

#### Service Creation with Dependency Injection
```rust
// Repository configuration
let repository_config = RepositoryConfig::from(config.clone());
let repository: Arc<dyn SnippetRepository> = Arc::new(BkmrRepository::new(repository_config));

// Service creation with injected dependencies
let document_service = Arc::new(DocumentService::new());
let completion_service = Arc::new(CompletionService::new(repository));

// Backend with injected services
Self {
    client,
    config,
    document_service,
    completion_service,
}
```

#### Error Handling Pattern
```rust
// Service layer error handling
async fn get_completions(&self, context: &CompletionContext) -> Result<Vec<CompletionItem>> {
    let snippets = self.repository
        .search_snippets(&filter)
        .await
        .context("search snippets from repository")?;

    let completion_items = self.process_snippets(snippets, context)
        .context("process snippets into completion items")?;

    Ok(completion_items)
}
```

#### Repository Pattern Implementation
```rust
#[async_trait]
pub trait SnippetRepository: Send + Sync + std::fmt::Debug {
    async fn search_snippets(&self, filter: &SnippetFilter) -> Result<Vec<Snippet>>;
    async fn health_check(&self) -> Result<()>;
}

// Real implementation
pub struct BkmrRepository {
    config: RepositoryConfig,
}

// Mock implementation for testing
pub struct MockSnippetRepository {
    snippets: Vec<Snippet>,
}
```

### Testing Architecture

#### Layer-Specific Testing
- **Domain Layer**: Pure unit tests with no external dependencies
- **Repository Layer**: Integration tests with real bkmr CLI + mock implementations for unit tests
- **Service Layer**: Unit tests with mock repositories + integration tests with real repositories
- **Infrastructure Layer**: Full LSP protocol tests with real server instances

#### Test Categories by Architecture Layer
```rust
// Domain layer testing (pure logic)
#[test]
fn given_language_id_when_building_fts_query_then_includes_universal_snippets() {
    let filter = SnippetFilter::new(Some("rust".to_string()), None, 50);
    let query = filter.build_fts_query();
    // Pure business logic validation
}

// Service layer testing (with dependency injection)
#[tokio::test]
async fn given_backend_with_mock_repository_when_completing_then_uses_repository() {
    let repository = Arc::new(MockSnippetRepository::new().with_snippets(vec![snippet]));
    let backend = BkmrLspBackend::with_repository(client, config, repository);
    // Test service orchestration with controlled dependencies
}

// Infrastructure layer testing (full LSP protocol)
#[test_log::test(tokio::test)]
async fn test_real_lsp_completion_flow() {
    let mut context = TestContext::new();  // Spawns real LSP server
    context.initialize().await?;
    // Test complete LSP protocol compliance
}
```

### Development Workflow

#### Adding New Features
1. **Domain Models**: Define business entities in `domain/`
2. **Repository Interface**: Extend repository traits if new data access needed
3. **Service Logic**: Implement business operations in `services/`
4. **Infrastructure**: Wire up LSP protocol handlers in `backend.rs`
5. **Testing**: Add tests at each layer (unit → integration → LSP protocol)

#### Refactoring Guidelines
- Keep domain layer pure (no external dependencies)
- Use dependency injection for testability
- Maintain clear boundaries between layers
- Follow consistent error handling patterns
- Update tests at appropriate architectural levels

### Architecture Benefits

#### Maintainability
- Clear separation of concerns makes code easier to understand and modify
- Dependencies flow inward (domain ← service ← infrastructure)
- Business logic isolated from LSP protocol details

#### Testability
- Mock repositories enable fast unit testing without external CLI
- Dependency injection allows testing with controlled environments
- Each layer can be tested independently

#### Flexibility
- Repository pattern allows swapping data sources
- Service layer enables different LSP implementations
- Domain models can support multiple protocols (LSP, CLI, web API)

#### Code Quality
- Consistent error handling prevents silent failures
- Async/await eliminates blocking operations
- Strong typing with comprehensive error contexts

## Server Output and Logging

The LSP server output goes to different locations depending on how you run it:

### Default LSP Server Output

**stderr**: The server logs to stderr by default (see `main.rs:77`)
```rust
.with_writer(std::io::stderr)
```

### Development Logging

**~/bkmr-lsp.log**: When running with `RUST_LOG` environment variable, output typically gets redirected here. The Makefile shows this pattern:

```bash
make log-lsp    # Tails ~/bkmr-lsp.log with JSON formatting
```

### Manual Logging Setup

To capture server output during development:

```bash
# Redirect stderr to a log file
RUST_LOG=debug bkmr-lsp 2>~/bkmr-lsp.log

# Or use the make target to watch logs
make log-lsp    # Tails ~/bkmr-lsp.log and formats JSON output
```

### LSP Client Integration

When run by an LSP client (VS Code, Vim, IntelliJ), the server output typically goes to:
- The client's LSP logs (varies by editor)
- For IntelliJ plugin development: `make log-plugin` shows filtered completion logs

### Quick Check

To see if the server is producing output:
```bash
ls -la ~/bkmr-lsp.log    # Check if log file exists
tail -f ~/bkmr-lsp.log   # Watch live output
```

The `make init` command clears this log file as part of development setup.

## Testing Strategy

Our testing approach combines unit tests, integration tests, and LSP protocol tests to ensure comprehensive coverage of all functionality.

### Test Categories

#### 1. Unit Tests (`tests/test_backend.rs`)
- **Configuration testing**: Default values, validation
- **Data structure testing**: Snippet deserialization, builder patterns
- **Business logic testing**: FTS query building, language info, Rust pattern translation
- **Static method testing**: Functions that don't require external dependencies

#### 2. LSP Protocol Structure Tests (`tests/test_lsp_protocol.rs`)
- **LSP data structure validation**: Request/response formatting
- **Server capabilities structure**: Completion options, command providers
- **Position and range validation**: LSP coordinate system testing
- **Text edit structures**: Completion text replacement testing

#### 3. Comprehensive LSP Integration Tests (`tests/test_lsp_integration.rs`)
- **Real server initialization**: Full client-server handshake with actual BkmrLspBackend
- **Document lifecycle testing**: Real didOpen, didChange, didSave operations
- **Live completion testing**: Actual completion requests with server responses
- **Command execution testing**: Real workspace/executeCommand handling
- **Multi-document scenarios**: Testing server state management across files
- **Configuration change testing**: Dynamic server reconfiguration
- **Error handling verification**: Server stability under invalid requests

**Key Benefits:**
- Tests actual LSP protocol compliance, not just mocked responses
- Verifies real async communication and message serialization
- Ensures server handles concurrent requests correctly
- Validates complete request/response cycles including logging

#### 4. Integration Tests (`tests/integration_test_interpolation.rs`)
- **External CLI integration**: Real bkmr command execution
- **End-to-end validation**: Full snippet retrieval and interpolation
- **Environment testing**: bkmr availability and version compatibility

#### 5. Error Handling Tests (`tests/test_error_handling.rs`)
- **Invalid input handling**: Malformed requests, invalid positions
- **External dependency failures**: Missing bkmr binary, timeouts
- **Edge cases**: Empty data, Unicode content, unusual language IDs
- **Graceful degradation**: Server stability under error conditions
- **LSP error scenario testing**: Real server error handling using TestContext

### Testing Tools and Frameworks

#### Core Testing Infrastructure
- **`test-log`**: Enhanced test logging with trace support
  ```rust
  #[test_log::test(tokio::test)]
  async fn my_test() {
      // Automatic logging setup for debugging
  }
  ```

- **`tokio-test`**: Async testing utilities and assertions

#### Real LSP Communication (`tests/test_utils.rs`)
- **`TestContext`**: **Real LSP server spawning** with full protocol communication
- **`AsyncIn/AsyncOut`**: Async streams implementing real AsyncRead/AsyncWrite traits
- **`SnippetBuilder`**: Test data creation with builder pattern
- **`start_server`**: Function to spawn actual BkmrLspBackend for testing
- **`parse_lsp_messages`**: Advanced Content-Length header parsing for concatenated messages

**Key Features:**
- **Real server spawning**: Uses actual `BkmrLspBackend` instances via `start_server()`
- **Proper LSP protocol parsing**: Handles Content-Length headers and message boundaries
- **Concatenated message support**: Parses multiple LSP messages in single response
- **Intelligent message filtering**: Automatically skips `window/logMessage` notifications
- **Comprehensive error handling**: Graceful parsing of malformed or incomplete messages
- **Debug visibility**: Detailed logging of message parsing and server communication

Example usage:
```rust
let mut context = TestContext::new();  // Spawns real LSP server
context.initialize().await?;

// Test real LSP document lifecycle
context.send_all(&[
    r#"{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"languageId":"rust","text":":hello","uri":"file:///tmp/test.rs","version":0}}}"#
]).await?;

// Test real completion with actual server response
let completion_request = jsonrpc::Request::build("textDocument/completion")
    .id(1)
    .params(serde_json::json!({"position":{"character":6,"line":0},"textDocument":{"uri":"file:///tmp/test.rs"}}))
    .finish();

let response = context.request::<Option<CompletionResponse>>(&completion_request).await?;
```

### Test Execution

#### Running Tests
```bash
# All tests (single-threaded to avoid conflicts)
make test
cargo test -- --test-threads=1

# Specific test modules
cargo test test_backend              # Unit tests
cargo test test_lsp_protocol         # LSP data structure tests
cargo test test_lsp_integration      # Comprehensive LSP server tests
cargo test test_error_handling       # Error handling and edge cases

# Integration tests only
cargo test integration_test

# With debug logging
RUST_LOG=debug cargo test test_backend -- --nocapture
```

#### Test Categories by Speed
- **Fast** (Unit tests): No external dependencies, pure logic testing (~0.03s)
- **Fast-Medium** (LSP protocol structure): Data structure validation, no server spawning (~0.01s)
- **Medium** (LSP integration): Real server spawning with message parsing, no external CLI calls (~0.15s)
- **Slow** (Integration): External bkmr CLI execution, file system operations (~0.15s)

**Note:** LSP integration tests are surprisingly fast despite real server spawning due to efficient async communication and proper message parsing that eliminates hanging scenarios.

### Test Data Management

#### Mock Data Creation
Use the `SnippetBuilder` for consistent test data:
```rust
let snippet = SnippetBuilder::new()
    .with_id(42)
    .with_title("Test Function")
    .with_content("fn test() {}")
    .with_language("rust")
    .with_snippet_tag()
    .build();
```

#### External CLI Testing
Integration tests require:
- bkmr CLI installed and in PATH
- Test snippets with `_snip_` tag available
- Proper bkmr configuration

### Debugging Tests

#### Test Logging
```bash
# Enable all logs during tests
RUST_LOG=debug cargo test -- --nocapture

# Filter to specific modules
RUST_LOG=bkmr_lsp=debug cargo test -- --nocapture

# Test-specific logging
RUST_LOG=trace cargo test test_completion -- --nocapture
```

#### LSP Communication Debugging
The `TestContext` provides detailed logging of real LSP message exchange with advanced parsing:

```
Mock LSP read value: "Content-Length: 154\r\n\r\n{\"jsonrpc\":\"2.0\",\"method\":\"textDocument/completion\",...}"
Mock LSP write value: "Content-Length: 38\r\n\r\n{\"jsonrpc\":\"2.0\",\"result\":null,\"id\":2}Content-Length: 115\r\n\r\n{\"jsonrpc\":\"2.0\",\"method\":\"window/logMessage\",...}"

DEBUG Received raw response: Content-Length: 38...
DEBUG Parsed 2 messages from response
DEBUG Processing message: {"jsonrpc":"2.0","result":null,"id":2}
DEBUG Successfully parsed response value: Null
DEBUG Processing message: {"jsonrpc":"2.0","method":"window/logMessage",...}
DEBUG Skipping log message: {...}
```

**Key Debugging Features:**
- **Raw message inspection**: Full Content-Length headers and concatenated responses
- **Message boundary parsing**: Shows how multiple messages are separated and processed
- **Filtering logic**: Demonstrates automatic log message skipping
- **Parsing success/failure**: Detailed JSON-RPC parsing results for each message
- **Response value extraction**: Shows final parsed values returned to tests

This provides complete visibility into the LSP protocol interaction, including the complex message parsing that handles server responses containing multiple concatenated messages.

#### Failed Test Investigation
1. **Check logs**: Use `RUST_LOG=debug` for detailed execution traces
2. **Verify environment**: Ensure bkmr CLI is available for integration tests
3. **Isolate issue**: Run specific test modules to narrow down failures
4. **Mock vs Real**: Use LSP protocol tests for isolated testing, integration tests for end-to-end validation
5. **Message parsing issues**: Look for "Parsed N messages from response" in debug logs
6. **Hanging tests**: Check for infinite loops in `recv()` - usually indicates message parsing problems

##### Common LSP Test Issues and Solutions

**Issue: Test hangs indefinitely**
```bash
# Symptom: Test runs forever, no output
cargo test test_lsp_error_handling -- --nocapture
# Hangs here...
```

**Diagnosis:** Check if server sends concatenated messages that aren't parsed correctly:
```bash
# Look for this pattern in logs:
DEBUG Mock LSP write value: "Content-Length: 38\r\n\r\n{...}Content-Length: 115\r\n\r\n{...}"
```

**Solution:** The `parse_lsp_messages()` function should handle this automatically. If hanging persists:
1. Verify Content-Length header parsing logic
2. Check message boundary detection
3. Ensure log message filtering works correctly

**Issue: Unexpected test results**
```rust
// Expected empty array, got Some(Array([...]))
assert_eq!(items.len(), 0); // Fails
```

**Diagnosis:** Server behavior changed - this tests **real server logic**, not mocked responses.

**Solution:** Update test expectations to match actual server behavior, or fix server logic if incorrect.

**Issue: JSON parsing errors**
```
Failed to parse message as JSON-RPC response: {...} - Error: missing field 'id'
```

**Diagnosis:** Server sent notification (no 'id' field) instead of response, or message boundaries are wrong.

**Solution:** Check if message is a notification that should be filtered, or fix Content-Length parsing.

### Test Coverage Best Practices

#### What We Test
- **All public interfaces**: LSP protocol handlers, external API calls
- **Error conditions**: Invalid input, missing dependencies, timeouts
- **Edge cases**: Empty data, Unicode, unusual language IDs
- **Configuration variants**: Different settings and environment conditions

#### What We Mock
- **External CLI calls**: Mock bkmr CLI calls in unit tests for isolation
- **File system**: Use in-memory structures where possible for speed
- **Network dependencies**: Avoid external service calls in unit tests

#### What We Test Real
- **LSP server communication**: Actual `BkmrLspBackend` instances with real protocol parsing
- **bkmr CLI integration**: Actual command execution in integration tests
- **LSP protocol compliance**: Real message serialization/deserialization with Content-Length headers
- **Snippet interpolation**: End-to-end bkmr functionality
- **Error handling scenarios**: Real server responses to invalid requests

#### Advanced Testing Patterns

##### LSP Message Parsing Testing Strategy
Our `TestContext` implementation addresses a critical challenge in LSP testing: **concatenated message handling**.

**Problem:** LSP servers often send multiple messages in a single write operation:
```
"Content-Length: 38\r\n\r\n{\"jsonrpc\":\"2.0\",\"result\":null,\"id\":2}Content-Length: 115\r\n\r\n{\"jsonrpc\":\"2.0\",\"method\":\"window/logMessage\",...}"
```

**Solution:** Advanced `parse_lsp_messages()` function that:
1. **Parses Content-Length headers** to identify message boundaries
2. **Extracts individual messages** from concatenated responses
3. **Filters log messages** automatically (window/logMessage notifications)
4. **Returns first valid response** for test assertions
5. **Provides fallback parsing** for malformed responses

**Why This Matters:**
- **Prevents test hangs**: Avoids infinite waits when responses are mis-parsed
- **Tests real behavior**: Validates actual LSP protocol compliance, not mocked responses
- **Debugging support**: Detailed logging shows exact message parsing flow
- **Error handling verification**: Tests how server handles invalid requests without timeouts

##### Error Testing Without Timeouts
Our error handling tests validate **actual server behavior** rather than timeout scenarios:

```rust
// Test invalid completion request
let result = context.request::<Option<CompletionResponse>>(&invalid_completion).await;

// Server returns empty array [] for unknown documents - this is the real behavior we test
match result {
    Ok(Some(CompletionResponse::Array(items))) => {
        assert_eq!(items.len(), 0); // Server gracefully returns empty results
    }
    // ... handle other valid responses
}
```

**Key Principles:**
- **Test server logic**: How does the server handle invalid input?
- **Verify graceful degradation**: Server should not crash on bad requests
- **Check response semantics**: Invalid completion → empty array, invalid command → null
- **No artificial timeouts**: Let the server respond naturally to test real behavior

### Continuous Integration

#### Pre-commit Testing
```bash
# Quick validation before commits
make test              # Run all tests
make format           # Code formatting
make lint             # Clippy linting
```

#### CI Pipeline Considerations
- **Environment setup**: Ensure bkmr CLI availability
- **Test isolation**: Single-threaded execution to prevent conflicts
- **Timeout handling**: Set reasonable timeouts for external CLI calls
- **Log collection**: Capture test logs for debugging CI failures

### Performance Testing

#### Timeout Testing
External CLI calls include timeout protection:
```rust
let output = tokio::time::timeout(
    Duration::from_secs(10),
    command_future
).await?;
```

#### Load Testing
Consider testing with:
- Large numbers of completion requests
- Multiple concurrent document operations
- Heavy snippet databases

### Adding New Tests

#### Checklist for New Features
1. **Unit tests**: Test pure logic in isolation
2. **LSP protocol tests**: Test client-server interaction
3. **Integration tests**: Test with real external dependencies
4. **Error handling**: Test failure modes and edge cases
5. **Documentation**: Update this guide with new testing patterns

#### Test Organization
- Keep test files focused on specific aspects
- Use descriptive test names indicating what's being tested
- Group related tests in the same module
- Add comments explaining complex test scenarios