/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

//! Channel 3: MCP Server (bidirectional)
//!
//! Messages for MCP Server functionality (Browser Use API).

use serde::{Deserialize, Serialize};

/// MCP request source information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpSource {
    pub agent: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
}

/// JSON-RPC 2.0 request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: serde_json::Value,
    pub method: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,
}

/// JSON-RPC 2.0 response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: serde_json::Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

/// JSON-RPC 2.0 error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

/// MCP request (Agent → Extension)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpRequestPayload {
    pub request_id: String,
    pub source: McpSource,
    pub payload: JsonRpcRequest,
}

/// MCP response (Extension → Agent)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpResponsePayload {
    pub request_id: String,
    pub payload: JsonRpcResponse,
}

/// All Channel 3 message types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload", rename_all = "snake_case")]
pub enum McpMessage {
    McpRequest(McpRequestPayload),
    McpResponse(McpResponsePayload),
}

impl McpMessage {
    /// Get request_id
    pub fn request_id(&self) -> &str {
        match self {
            Self::McpRequest(p) => &p.request_id,
            Self::McpResponse(p) => &p.request_id,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mcp_request_serialization() {
        let msg = McpMessage::McpRequest(McpRequestPayload {
            request_id: "req-1".to_string(),
            source: McpSource {
                agent: "claude-code".to_string(),
                session_id: None,
            },
            payload: JsonRpcRequest {
                jsonrpc: "2.0".to_string(),
                id: serde_json::json!(1),
                method: "browser_use/click".to_string(),
                params: Some(serde_json::json!({"selector": "#btn"})),
            },
        });
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("mcp_request"));
        assert!(json.contains("browser_use/click"));
    }

    #[test]
    fn test_mcp_response_serialization() {
        let msg = McpMessage::McpResponse(McpResponsePayload {
            request_id: "req-1".to_string(),
            payload: JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: serde_json::json!(1),
                result: Some(serde_json::json!({"success": true})),
                error: None,
            },
        });
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("mcp_response"));
        assert!(json.contains("success"));
    }
}
