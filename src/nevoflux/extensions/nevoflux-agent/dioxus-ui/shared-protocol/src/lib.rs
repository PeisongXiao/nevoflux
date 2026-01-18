/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

//! Shared Protocol for NevoFlux Extension Communication (v4.0)
//!
//! This crate defines the Rust types for communication between:
//! - Chat Sidebar (Dioxus WASM) <-> Native Agent (Rust)
//!
//! ## Channel Architecture
//!
//! - Channel 1 (Input): Sidebar → Agent
//! - Channel 2 (Output): Agent → Sidebar
//! - Channel 3 (MCP Server): Bidirectional MCP requests
//! - Channel 4 (Page Mode): Bidirectional LLM via browser

pub mod common;
pub mod channel1;
pub mod channel2;
pub mod channel3;
pub mod channel4;

// Re-export common types
pub use common::*;

// Re-export channel types
pub use channel1::{
    InputMessage, ChatMessagePayload, SkillCommandPayload,
    StopGenerationPayload, PermissionResponsePayload,
    PluginCommandPayload, SystemCommandPayload,
};

pub use channel2::{
    OutputMessage, StreamChunkPayload, StreamEndPayload, StreamMetadata,
    ContentBlockPayload, PermissionRequestPayload, AgentStatePayload,
    StepInfo, ToolInfo, ErrorPayload, AccountStatusPayload, AccountInfo,
    PlanInfo, QuotaInfo, UsageQuota, SystemResponsePayload, SystemError,
};

pub use channel3::{
    McpMessage, McpRequestPayload, McpResponsePayload,
    McpSource, JsonRpcRequest, JsonRpcResponse, JsonRpcError,
};

pub use channel4::{
    PageLlmMessage, PageLlmRequestPayload, PageLlmChunkPayload,
    PageLlmDonePayload, PageLlmErrorPayload, PageLlmError,
    OpenAiRequest, OpenAiMessage, OpenAiChunk, OpenAiChunkChoice,
    OpenAiDelta, OpenAiCompletion, OpenAiCompletionChoice, OpenAiUsage,
};

#[cfg(feature = "wasm")]
mod wasm_bindings {
    use super::*;
    use wasm_bindgen::prelude::*;

    /// Get protocol version
    #[wasm_bindgen]
    pub fn get_protocol_version() -> String {
        PROTOCOL_VERSION.to_string()
    }

    /// Serialize InputMessage to JSON
    #[wasm_bindgen]
    pub fn serialize_input_message(message: JsValue) -> Result<String, JsValue> {
        let msg: InputMessage = serde_wasm_bindgen::from_value(message)?;
        serde_json::to_string(&msg)
            .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))
    }

    /// Deserialize JSON to OutputMessage
    #[wasm_bindgen]
    pub fn deserialize_output_message(json: &str) -> Result<JsValue, JsValue> {
        let msg: OutputMessage = serde_json::from_str(json)
            .map_err(|e| JsValue::from_str(&format!("Deserialization error: {}", e)))?;
        serde_wasm_bindgen::to_value(&msg)
            .map_err(|e| JsValue::from_str(&format!("Conversion error: {}", e)))
    }

    /// Serialize McpMessage to JSON
    #[wasm_bindgen]
    pub fn serialize_mcp_message(message: JsValue) -> Result<String, JsValue> {
        let msg: McpMessage = serde_wasm_bindgen::from_value(message)?;
        serde_json::to_string(&msg)
            .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))
    }

    /// Deserialize JSON to McpMessage
    #[wasm_bindgen]
    pub fn deserialize_mcp_message(json: &str) -> Result<JsValue, JsValue> {
        let msg: McpMessage = serde_json::from_str(json)
            .map_err(|e| JsValue::from_str(&format!("Deserialization error: {}", e)))?;
        serde_wasm_bindgen::to_value(&msg)
            .map_err(|e| JsValue::from_str(&format!("Conversion error: {}", e)))
    }

    /// Serialize PageLlmMessage to JSON
    #[wasm_bindgen]
    pub fn serialize_page_llm_message(message: JsValue) -> Result<String, JsValue> {
        let msg: PageLlmMessage = serde_wasm_bindgen::from_value(message)?;
        serde_json::to_string(&msg)
            .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))
    }

    /// Deserialize JSON to PageLlmMessage
    #[wasm_bindgen]
    pub fn deserialize_page_llm_message(json: &str) -> Result<JsValue, JsValue> {
        let msg: PageLlmMessage = serde_json::from_str(json)
            .map_err(|e| JsValue::from_str(&format!("Deserialization error: {}", e)))?;
        serde_wasm_bindgen::to_value(&msg)
            .map_err(|e| JsValue::from_str(&format!("Conversion error: {}", e)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_protocol_version() {
        assert_eq!(PROTOCOL_VERSION, "4.0.0");
    }

    #[test]
    fn test_input_message_roundtrip() {
        let msg = InputMessage::ChatMessage(ChatMessagePayload {
            session_id: "s1".to_string(),
            message_id: "m1".to_string(),
            text: "Hello".to_string(),
            attachments: vec![],
        });
        let json = serde_json::to_string(&msg).unwrap();
        let parsed: InputMessage = serde_json::from_str(&json).unwrap();
        match parsed {
            InputMessage::ChatMessage(p) => assert_eq!(p.text, "Hello"),
            _ => panic!("Wrong type"),
        }
    }

    #[test]
    fn test_output_message_roundtrip() {
        let msg = OutputMessage::StreamChunk(StreamChunkPayload {
            session_id: "s1".to_string(),
            stream_id: "st1".to_string(),
            delta: "World".to_string(),
            format: StreamFormat::Markdown,
        });
        let json = serde_json::to_string(&msg).unwrap();
        let parsed: OutputMessage = serde_json::from_str(&json).unwrap();
        match parsed {
            OutputMessage::StreamChunk(p) => assert_eq!(p.delta, "World"),
            _ => panic!("Wrong type"),
        }
    }
}
