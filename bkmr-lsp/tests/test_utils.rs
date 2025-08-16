// Test utilities for LSP server testing

use std::pin::Pin;
use std::str::FromStr;
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tower_lsp::jsonrpc;
use bkmr_lsp::BkmrSnippet;

/// Async input stream for mock LSP communication
pub struct AsyncIn(UnboundedReceiver<String>);

/// Async output stream for mock LSP communication  
pub struct AsyncOut(UnboundedSender<String>);

/// Encode LSP message with proper Content-Length header
fn encode_message(content_type: Option<&str>, message: &str) -> String {
    let content_type = content_type
        .map(|ty| format!("\r\nContent-Type: {ty}"))
        .unwrap_or_default();

    format!(
        "Content-Length: {}{}\r\n\r\n{}",
        message.len(),
        content_type,
        message
    )
}

/// Parse multiple LSP messages from a single response string
fn parse_lsp_messages(response: &str) -> Vec<String> {
    let mut messages = Vec::new();
    let mut remaining = response;
    
    while !remaining.is_empty() {
        // Look for Content-Length header
        if let Some(content_length_start) = remaining.find("Content-Length: ") {
            let header_start = &remaining[content_length_start..];
            if let Some(header_end) = header_start.find("\r\n\r\n") {
                // Extract Content-Length value
                let length_str = &header_start[16..]; // Skip "Content-Length: "
                if let Some(length_end) = length_str.find("\r\n") {
                    if let Ok(content_length) = length_str[..length_end].parse::<usize>() {
                        let message_start = content_length_start + header_end + 4; // Skip past \r\n\r\n
                        if message_start + content_length <= remaining.len() {
                            let message = remaining[message_start..message_start + content_length].to_string();
                            messages.push(message);
                            remaining = &remaining[message_start + content_length..];
                            continue;
                        }
                    }
                }
            }
        }
        
        // If we can't parse a proper LSP message, try to extract JSON from the end
        // This is a fallback for malformed responses
        if let Some(last_line) = remaining.split('\n').last() {
            if !last_line.trim().is_empty() && (last_line.contains("jsonrpc") || last_line.starts_with('{')) {
                messages.push(last_line.to_string());
            }
        }
        break;
    }
    
    messages
}

impl AsyncRead for AsyncIn {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        let rx = self.get_mut();
        match rx.0.poll_recv(cx) {
            Poll::Ready(Some(v)) => {
                tracing::debug!("Mock LSP read value: {:?}", v);
                buf.put_slice(v.as_bytes());
                Poll::Ready(Ok(()))
            }
            _ => Poll::Pending,
        }
    }
}

impl AsyncWrite for AsyncOut {
    fn poll_write(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        let tx = self.get_mut();
        let value = String::from_utf8(buf.to_vec()).unwrap();
        tracing::debug!("Mock LSP write value: {value:?}");
        let _ = tx.0.send(value);
        Poll::Ready(Ok(buf.len()))
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}

/// Real test context that spawns an actual LSP server for comprehensive testing
pub struct TestContext {
    pub request_tx: UnboundedSender<String>,
    pub response_rx: UnboundedReceiver<String>,
    pub _server: tokio::task::JoinHandle<()>,
    pub _client: tokio::task::JoinHandle<()>,
}

impl Default for TestContext {
    fn default() -> Self {
        TestContext::new()
    }
}

impl TestContext {
    /// Create a new test context with real LSP server spawning
    pub fn new() -> Self {
        use tokio::sync::mpsc;
        
        let (request_tx, rx) = mpsc::unbounded_channel::<String>();
        let (tx, mut client_response_rx) = mpsc::unbounded_channel::<String>();
        let (client_tx, response_rx) = mpsc::unbounded_channel::<String>();

        let async_in = AsyncIn(rx);
        let async_out = AsyncOut(tx);

        let server = tokio::spawn(async move {
            bkmr_lsp::start_server(async_in, async_out).await
        });

        let client = tokio::spawn(async move {
            loop {
                let Some(response) = client_response_rx.recv().await else {
                    continue;
                };
                if client_tx.send(response).is_err() {
                    tracing::error!("Failed to pass client response");
                }
            }
        });

        Self {
            request_tx,
            response_rx,
            _server: server,
            _client: client,
        }
    }

    /// Send multiple LSP messages in sequence
    pub async fn send_all(&mut self, messages: &[&str]) -> anyhow::Result<()> {
        for message in messages {
            self.send(&jsonrpc::Request::from_str(message)?).await?;
        }
        Ok(())
    }

    /// Send a single LSP request
    pub async fn send(&mut self, request: &jsonrpc::Request) -> anyhow::Result<()> {
        self.request_tx
            .send(encode_message(None, &serde_json::to_string(request)?))?;
        Ok(())
    }

    /// Receive and parse an LSP response
    pub async fn recv<R: std::fmt::Debug + serde::de::DeserializeOwned>(
        &mut self,
    ) -> anyhow::Result<R> {
        loop {
            let response = self
                .response_rx
                .recv()
                .await
                .ok_or_else(|| anyhow::anyhow!("empty response"))?;
            
            tracing::debug!("Received raw response: {}", response);
            
            // Parse potentially multiple LSP messages from the response
            let messages = parse_lsp_messages(&response);
            tracing::debug!("Parsed {} messages from response", messages.len());
            
            for message in messages {
                tracing::debug!("Processing message: {}", message);
                
                // Skip log messages
                if message.contains("window/logMessage") {
                    tracing::debug!("Skipping log message: {}", message);
                    continue;
                }
                
                // Try to parse as JSON-RPC response
                match serde_json::from_str::<jsonrpc::Response>(&message) {
                    Ok(response) => {
                        let (_id, result) = response.into_parts();
                        match result {
                            Ok(value) => {
                                tracing::debug!("Successfully parsed response value: {:?}", value);
                                return Ok(serde_json::from_value(value)?);
                            }
                            Err(error) => {
                                tracing::debug!("JSON-RPC error in response: {:?}", error);
                                return Err(anyhow::anyhow!("JSON-RPC error: {:?}", error));
                            }
                        }
                    }
                    Err(e) => {
                        tracing::debug!("Failed to parse message as JSON-RPC response: {} - Error: {}", message, e);
                        // Continue to next message instead of failing immediately
                        continue;
                    }
                }
            }
            
            // If no valid response was found in this batch, wait for the next response
            tracing::debug!("No valid response found in message batch, waiting for next response");
        }
    }

    /// Send a request and wait for response
    pub async fn request<R: std::fmt::Debug + serde::de::DeserializeOwned>(
        &mut self,
        request: &jsonrpc::Request,
    ) -> anyhow::Result<R> {
        self.send(request).await?;
        self.recv().await
    }

    /// Send initialize request and wait for response
    pub async fn initialize(&mut self) -> anyhow::Result<()> {
        use tower_lsp::lsp_types;
        
        let request = jsonrpc::Request::build("initialize")
            .id(1)
            .params(serde_json::json!({"capabilities":{}}))
            .finish();

        let _ = self
            .request::<lsp_types::InitializeResult>(&request)
            .await?;

        Ok(())
    }
}

/// Test data builder for creating mock bkmr snippets
#[derive(Default)]
pub struct SnippetBuilder {
    snippet: BkmrSnippet,
}

impl SnippetBuilder {
    pub fn new() -> Self {
        Self {
            snippet: BkmrSnippet {
                id: 1,
                title: "Test Snippet".to_string(),
                url: "test content".to_string(),
                description: "Test description".to_string(),
                tags: vec!["test".to_string()],
                access_count: 0,
            },
        }
    }

    pub fn with_id(mut self, id: i32) -> Self {
        self.snippet.id = id;
        self
    }

    pub fn with_title(mut self, title: &str) -> Self {
        self.snippet.title = title.to_string();
        self
    }

    pub fn with_content(mut self, content: &str) -> Self {
        self.snippet.url = content.to_string();
        self
    }

    pub fn with_description(mut self, description: &str) -> Self {
        self.snippet.description = description.to_string();
        self
    }

    pub fn with_tags(mut self, tags: Vec<&str>) -> Self {
        self.snippet.tags = tags.into_iter().map(|s| s.to_string()).collect();
        self
    }

    pub fn with_language(mut self, language: &str) -> Self {
        if !self.snippet.tags.contains(&language.to_string()) {
            self.snippet.tags.push(language.to_string());
        }
        self
    }

    pub fn with_snippet_tag(mut self) -> Self {
        if !self.snippet.tags.contains(&"_snip_".to_string()) {
            self.snippet.tags.push("_snip_".to_string());
        }
        self
    }

    pub fn with_universal_tag(mut self) -> Self {
        if !self.snippet.tags.contains(&"universal".to_string()) {
            self.snippet.tags.push("universal".to_string());
        }
        self
    }

    pub fn build(self) -> BkmrSnippet {
        self.snippet
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_snippet_builder() {
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

    #[test]
    fn test_encode_message() {
        let msg = r#"{"test": "message"}"#;
        let encoded = encode_message(None, msg);
        let expected_length = msg.len();
        
        assert!(encoded.contains(&format!("Content-Length: {}", expected_length)));
        assert!(encoded.ends_with(msg));
    }
}