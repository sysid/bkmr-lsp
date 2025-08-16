# TODO: Architecture Optimization Plan for bkmr-lsp

## Current Architecture Assessment

### ✅ **Strengths**
1. **Clean LSP Implementation**: Well-structured tower-lsp backend with proper async/await patterns
2. **Comprehensive Testing**: Multiple test categories (unit, integration, mocking utilities) 
3. **Good Error Handling**: Uses anyhow for error propagation with structured logging
4. **Modular Design**: Clear separation between main.rs, backend.rs, and lib.rs
5. **Modern Rust Patterns**: Uses tokio async runtime, structured logging with tracing
6. **External Process Integration**: Proper timeout handling for bkmr CLI calls

### ❌ **Architecture Issues**

#### **1. Single Monolithic Backend Module (1,064 lines)**
- `backend.rs` violates single responsibility principle
- Mixed concerns: LSP protocol, language processing, external command execution, caching
- Hard to test individual components in isolation

#### **2. Violation of Dependency Inversion**
- Direct coupling to external `bkmr` CLI process
- No abstraction layer for snippet retrieval
- Hardcoded command construction makes testing difficult

#### **3. Missing Domain Separation**
- No clear domain models vs. protocol types
- Language translation logic mixed with LSP handling
- No service layer abstraction

#### **4. Inefficient Caching Strategy**
- Uses `Arc<RwLock<HashMap>>` for document cache (potential bottleneck)
- No cache eviction strategy
- Thread contention on read-heavy workloads

#### **5. Regex Compilation Performance**
- Regex patterns compiled on every function call
- No lazy static compilation for frequently used patterns

## Clean Architecture Optimization Plan

### **Phase 1: Domain Layer Extraction**
- [ ] **Create domain models** (`src/domain/`)
  - [ ] `snippet.rs` - Core snippet domain model
  - [ ] `language.rs` - Language information value objects
  - [ ] `completion.rs` - Completion request/response models

- [ ] **Extract language processing service** (`src/services/`)
  - [ ] `language_translator.rs` - Rust pattern translation logic
  - [ ] `snippet_processor.rs` - Content interpolation and processing

### **Phase 2: Repository Pattern Implementation**
- [ ] **Create repository abstraction** (`src/repositories/`)
  - [ ] `snippet_repository.rs` - Trait for snippet retrieval
  - [ ] `bkmr_repository.rs` - Implementation using bkmr CLI
  - [ ] `mock_repository.rs` - In-memory implementation for testing

- [ ] **Dependency injection setup**
  - [ ] Configure repository in main.rs
  - [ ] Pass repository to backend via constructor

### **Phase 3: Service Layer Refactoring**
- [ ] **Extract LSP services** (`src/services/`)
  - [ ] `completion_service.rs` - Business logic for completions
  - [ ] `document_service.rs` - Document management and caching
  - [ ] `command_service.rs` - LSP command execution

- [ ] **Cache optimization**
  - [ ] Replace `Arc<RwLock<HashMap>>` with `DashMap` for concurrent access
  - [ ] Implement LRU cache with size limits
  - [ ] Add cache eviction policies

### **Phase 4: Performance Optimizations**
- [ ] **Regex compilation optimization**
  - [ ] Use `lazy_static` for regex patterns
  - [ ] Pre-compile all language translation patterns

- [ ] **Memory efficiency improvements**
  - [ ] Use `String` vs `&str` more strategically
  - [ ] Implement Copy-on-Write for cached documents
  - [ ] Reduce allocations in hot paths

- [ ] **Async optimization**
  - [ ] Add connection pooling for external commands
  - [ ] Implement request batching for multiple completions

### **Phase 5: Error Handling & Observability**
- [ ] **Structured error types** (`src/errors/`)
  - [ ] Domain-specific error types
  - [ ] Error context preservation
  - [ ] Recovery strategies

- [ ] **Enhanced observability**
  - [ ] Metrics collection for completion latency
  - [ ] Structured event logging
  - [ ] Health check endpoints

### **Phase 6: Testing Architecture**
- [ ] **Property-based testing** with `quickcheck`
  - [ ] Language translation correctness
  - [ ] Cache consistency properties

- [ ] **Snapshot testing** with `insta`
  - [ ] LSP response format stability
  - [ ] Language translation output verification

- [ ] **Integration test improvements**
  - [ ] Mock external dependencies
  - [ ] End-to-end LSP protocol testing

## Implementation Priority

**High Priority** (Architectural issues):
- [ ] Extract domain models and repository pattern
- [ ] Break down monolithic backend module
- [ ] Implement dependency injection

**Medium Priority** (Performance):
- [ ] Optimize caching strategy
- [ ] Pre-compile regex patterns
- [ ] Improve async patterns

**Low Priority** (Enhancement):
- [ ] Enhanced error types
- [ ] Advanced testing strategies
- [ ] Observability improvements

## Expected Benefits

1. **Maintainability**: 50-70% reduction in module complexity
2. **Testability**: Full mock-ability of external dependencies
3. **Performance**: 20-30% reduction in completion latency
4. **Extensibility**: Easy addition of new language servers
5. **Reliability**: Better error handling and recovery
