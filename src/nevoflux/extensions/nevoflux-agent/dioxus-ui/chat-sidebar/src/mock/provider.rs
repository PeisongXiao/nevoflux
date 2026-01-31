/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

//! Mock data generation

use crate::mock::config::MockConfig;
use shared_protocol::*;

/// Mock data provider
pub struct MockProvider {
    config: MockConfig,
}

/// Mock response types
pub enum MockResponse {
    /// Normal streaming response
    Stream(Vec<StreamChunk>),
    /// Error response
    Error(MockError),
    /// Response with permission request
    WithPermission {
        permission: PermissionRequestPayload,
        response: Vec<StreamChunk>,
    },
}

/// A chunk of streamed content
pub struct StreamChunk {
    /// Content delta
    pub delta: String,
    /// Delay before this chunk (ms)
    pub delay_ms: u64,
}

/// Mock error
pub struct MockError {
    pub code: String,
    pub message: String,
    pub recoverable: bool,
}

impl MockProvider {
    /// Create a new mock provider
    pub fn new(config: MockConfig) -> Self {
        Self { config }
    }

    /// Generate a response based on input text
    pub fn generate_response(&self, input: &str) -> MockResponse {
        let input_lower = input.to_lowercase();

        // Check for error trigger (explicit or random when mock_errors=true)
        if input_lower.contains("error") || self.config.simulate_errors {
            return MockResponse::Error(self.generate_error());
        }

        // Check for permission trigger
        if input_lower.contains("permission") || input_lower.contains("file") || input_lower.contains("read") {
            if self.config.simulate_permissions {
                return MockResponse::WithPermission {
                    permission: self.generate_permission_request(),
                    response: self.generate_text_response(input),
                };
            }
        }

        // Check for code trigger
        if input_lower.contains("code") || input_lower.contains("function") || input_lower.contains("show") {
            return MockResponse::Stream(self.generate_code_response());
        }

        // Default text response
        MockResponse::Stream(self.generate_text_response(input))
    }

    fn generate_text_response(&self, input: &str) -> Vec<StreamChunk> {
        let response = format!(
            "I understand you're asking about: **{}**\n\n\
            Here's a helpful response with some *key points*:\n\n\
            1. **First**, let me explain the concept\n\
            2. **Then**, I'll provide some examples\n\
            3. **Finally**, here are some best practices\n\n\
            You can use `inline code` like this for technical terms.\n\n\
            Is there anything specific you'd like me to elaborate on?",
            input
        );

        self.text_to_chunks(&response)
    }

    fn generate_code_response(&self) -> Vec<StreamChunk> {
        let response = r#"Here's an example function:

```javascript
function greet(name) {
    return `Hello, ${name}!`;
}

// Usage
console.log(greet("World"));
```

This function takes a name parameter and returns a greeting string."#;

        self.text_to_chunks(response)
    }

    fn generate_error(&self) -> MockError {
        MockError {
            code: "LLM_TIMEOUT".to_string(),
            message: "Request timed out. The server took too long to respond.".to_string(),
            recoverable: true,
        }
    }

    /// Generate a permission request payload
    pub fn generate_permission_request(&self) -> PermissionRequestPayload {
        PermissionRequestPayload {
            request_id: uuid::Uuid::new_v4().to_string(),
            session_id: "mock-session".to_string(),
            resource_type: ResourceType::File,
            action: ResourceAction::Read,
            resource: "/home/user/documents/data.csv".to_string(),
            requester: Requester {
                requester_type: RequesterType::Agent,
                id: "agentic-chat".to_string(),
                name: "Agentic Chat".to_string(),
            },
            reason: "Need to read file contents for analysis".to_string(),
            scope: PermissionScope::Once,
            timeout_ms: 60000,
        }
    }

    /// Convert text to stream chunks (preserves formatting)
    fn text_to_chunks(&self, text: &str) -> Vec<StreamChunk> {
        let mut chunks = Vec::new();
        let mut current = String::new();
        let mut char_count = 0;
        let chunk_size = 20; // Characters per chunk

        for ch in text.chars() {
            current.push(ch);
            char_count += 1;

            // Create chunk every N characters or at newlines
            if char_count >= chunk_size || ch == '\n' {
                let delay = self.config.typing_speed_ms * current.len() as u64 / 10;
                chunks.push(StreamChunk {
                    delta: current.clone(),
                    delay_ms: delay.max(10), // Minimum 10ms delay
                });
                current.clear();
                char_count = 0;
            }
        }

        // Don't forget the last chunk
        if !current.is_empty() {
            chunks.push(StreamChunk {
                delta: current,
                delay_ms: self.config.typing_speed_ms,
            });
        }

        chunks
    }
}
