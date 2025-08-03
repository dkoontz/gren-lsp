use serde_json::{json, Value};
use tower_lsp::lsp_types::*;

pub struct MessageBuilder {
    id_counter: u64,
}

impl MessageBuilder {
    pub fn new() -> Self {
        Self { id_counter: 1 }
    }

    fn next_id(&mut self) -> u64 {
        let id = self.id_counter;
        self.id_counter += 1;
        id
    }

    pub fn initialize_request(&mut self, capabilities: Option<ClientCapabilities>) -> Value {
        json!({
            "jsonrpc": "2.0",
            "id": self.next_id(),
            "method": "initialize",
            "params": {
                "processId": std::process::id(),
                "rootUri": "file:///tmp/test-workspace",
                "capabilities": capabilities.unwrap_or_default()
            }
        })
    }

    pub fn initialized_notification(&self) -> Value {
        json!({
            "jsonrpc": "2.0",
            "method": "initialized",
            "params": {}
        })
    }

    pub fn shutdown_request(&mut self) -> Value {
        json!({
            "jsonrpc": "2.0",
            "id": self.next_id(),
            "method": "shutdown"
        })
    }

    pub fn exit_notification(&self) -> Value {
        json!({
            "jsonrpc": "2.0",
            "method": "exit"
        })
    }

    pub fn hover_request(&mut self, uri: &str, line: u32, character: u32) -> Value {
        json!({
            "jsonrpc": "2.0",
            "id": self.next_id(),
            "method": "textDocument/hover",
            "params": {
                "textDocument": {"uri": uri},
                "position": {"line": line, "character": character}
            }
        })
    }

    pub fn completion_request(&mut self, uri: &str, line: u32, character: u32) -> Value {
        json!({
            "jsonrpc": "2.0",
            "id": self.next_id(),
            "method": "textDocument/completion",
            "params": {
                "textDocument": {"uri": uri},
                "position": {"line": line, "character": character}
            }
        })
    }

    pub fn invalid_method_request(&mut self, method: &str) -> Value {
        json!({
            "jsonrpc": "2.0",
            "id": self.next_id(),
            "method": method,
            "params": {}
        })
    }

    pub fn request_without_jsonrpc(&mut self, method: &str) -> Value {
        json!({
            "id": self.next_id(),
            "method": method,
            "params": {}
        })
    }

    pub fn request_without_id(&self, method: &str) -> Value {
        json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": {}
        })
    }

    pub fn request_with_invalid_version(&mut self, method: &str) -> Value {
        json!({
            "jsonrpc": "1.0",
            "id": self.next_id(),
            "method": method,
            "params": {}
        })
    }

    pub fn large_message(&mut self, size_mb: usize) -> Value {
        let large_string = "x".repeat(size_mb * 1024 * 1024);
        json!({
            "jsonrpc": "2.0",
            "id": self.next_id(),
            "method": "textDocument/hover",
            "params": {
                "textDocument": {"uri": format!("file:///{}", large_string)},
                "position": {"line": 0, "character": 0}
            }
        })
    }

    pub fn concurrent_requests(&mut self, count: usize) -> Vec<Value> {
        (0..count)
            .map(|i| {
                json!({
                    "jsonrpc": "2.0",
                    "id": self.next_id(),
                    "method": "textDocument/hover",
                    "params": {
                        "textDocument": {"uri": format!("file:///test{}.gren", i)},
                        "position": {"line": i as u32, "character": 0}
                    }
                })
            })
            .collect()
    }
}

impl Default for MessageBuilder {
    fn default() -> Self {
        Self::new()
    }
}