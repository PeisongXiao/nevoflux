/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

//! Functions for sending messages to background script
//!
//! Uses the `bg:` API namespace for communication with background.js:
//! - `bg:connect` - Establish connection to native agent
//! - `bg:send_to_agent` - Send ChatMessage to native agent
//! - `bg:exec_tool` - Execute browser tool via background.js
//! - `bg:get_tab_context` - Get current tab context

use crate::messaging::bridge::*;
use shared_protocol::*;
use wasm_bindgen_futures::JsFuture;

// ============================================
// Background API Request Types
// ============================================

/// Background API message wrapper using `bg:` namespace
#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum BackgroundRequest {
    /// Establish connection to native agent
    #[serde(rename = "bg:connect")]
    Connect,
    /// Send ChatMessage to native agent
    #[serde(rename = "bg:send_to_agent")]
    SendToAgent { payload: serde_json::Value },
    /// Execute browser tool via background.js
    #[serde(rename = "bg:exec_tool")]
    ExecTool { payload: BrowserToolRequestPayload },
    /// Get current tab context
    #[serde(rename = "bg:get_tab_context")]
    GetTabContext,
}

/// Response from bg:exec_tool
#[derive(Debug, Clone, serde::Deserialize)]
pub struct ExecToolResponse {
    pub success: bool,
    #[serde(default)]
    pub result: Option<serde_json::Value>,
    #[serde(default)]
    pub error: Option<BrowserToolError>,
}

// ============================================
// Agent Protocol Messages (Chat Channel)
// ============================================

/// Send ChatMessage to agent via bg:send_to_agent
pub async fn send_to_agent(message: ChatMessage) -> Result<(), String> {
    let payload = serde_json::to_value(&message)
        .map_err(|e| format!("Serialize ChatMessage error: {}", e))?;

    let request = BackgroundRequest::SendToAgent { payload };
    let js_value = to_js_value(&request)
        .map_err(|e| format!("Serialize request error: {:?}", e))?;

    JsFuture::from(runtime_send_message(js_value))
        .await
        .map_err(|e| format!("Send failed: {:?}", e))?;

    Ok(())
}

/// Send chat message to agent
pub async fn send_chat_message(
    session_id: &str,
    text: String,
    attachments: Vec<Attachment>,
    tab_id: Option<u32>,
) -> Result<(), String> {
    let message = ChatMessage::ChatMessage(ChatMessagePayload {
        session_id: session_id.to_string(),
        message_id: uuid::Uuid::new_v4().to_string(),
        text,
        attachments,
        tab_id: tab_id.map(|id| id as i64),
    });

    send_to_agent(message).await
}

/// Send stop generation command
pub async fn send_stop_generation(session_id: &str) -> Result<(), String> {
    let message = ChatMessage::StopGeneration(StopGenerationPayload {
        session_id: session_id.to_string(),
    });

    send_to_agent(message).await
}

/// Send permission response
pub async fn send_permission_response(
    request_id: String,
    granted: bool,
    scope: Option<PermissionScope>,
) -> Result<(), String> {
    let message = ChatMessage::PermissionResponse(PermissionResponsePayload {
        request_id,
        granted,
        scope,
    });

    send_to_agent(message).await
}

/// Send skill command
pub async fn send_skill_command(
    session_id: &str,
    skill_name: String,
    args: Option<serde_json::Value>,
) -> Result<(), String> {
    let message = ChatMessage::SkillCommand(SkillCommandPayload {
        session_id: session_id.to_string(),
        skill_name,
        args,
    });

    send_to_agent(message).await
}

/// Send browser tool response back to agent via bg:send_to_agent
pub async fn send_browser_tool_response(response: BrowserToolResponsePayload) -> Result<(), String> {
    let message = ChatMessage::BrowserToolResponse(response);
    send_to_agent(message).await
}

// ============================================
// Browser Tool Execution (bg:exec_tool)
// ============================================

/// Execute browser tool via bg:exec_tool and return result
///
/// This sends the request to background.js which has access to
/// browser.nevoflux.* API and can execute browser tools.
pub async fn exec_browser_tool(request: BrowserToolRequestPayload) -> Result<BrowserToolResponsePayload, String> {
    let bg_request = BackgroundRequest::ExecTool { payload: request.clone() };
    let js_value = to_js_value(&bg_request)
        .map_err(|e| format!("Serialize request error: {:?}", e))?;

    let response_js = JsFuture::from(runtime_send_message(js_value))
        .await
        .map_err(|e| format!("bg:exec_tool failed: {:?}", e))?;

    // Parse response
    let response: ExecToolResponse = from_js_value(response_js)
        .map_err(|e| format!("Parse exec_tool response error: {}", e))?;

    Ok(BrowserToolResponsePayload {
        request_id: request.request_id,
        session_id: request.session_id,
        success: response.success,
        result: response.result,
        error: response.error,
    })
}

// ============================================
// Extension Internal Messages
// ============================================

/// Extension-internal message types (not agent protocol)
/// These are messages between sidebar and background.js only
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", content = "payload", rename_all = "snake_case")]
pub enum InternalMessage {
    /// Ping to check connection
    Ping { timestamp: u64 },
    /// Pong response
    Pong { timestamp: u64 },
    /// Tab context update from background
    TabContextUpdate(TabContextPayload),
    /// Connection status update
    ConnectionStatus { connected: bool },
}

/// Tab context from background script
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TabContextPayload {
    pub tab_id: u32,
    pub url: String,
    pub title: String,
    #[serde(default)]
    pub favicon_url: Option<String>,
}

/// Send internal message to background
pub async fn send_internal(message: InternalMessage) -> Result<(), String> {
    let js_value = to_js_value(&message)
        .map_err(|e| format!("Serialize error: {:?}", e))?;

    JsFuture::from(runtime_send_message(js_value))
        .await
        .map_err(|e| format!("Send failed: {:?}", e))?;

    Ok(())
}

/// Request current tab context via bg:get_tab_context
pub async fn request_tab_context() -> Result<(), String> {
    let request = BackgroundRequest::GetTabContext;
    let js_value = to_js_value(&request)
        .map_err(|e| format!("Serialize error: {:?}", e))?;

    JsFuture::from(runtime_send_message(js_value))
        .await
        .map_err(|e| format!("Send failed: {:?}", e))?;

    Ok(())
}

/// Request connection to native agent via bg:connect
pub async fn request_connect() -> Result<(), String> {
    let request = BackgroundRequest::Connect;
    let js_value = to_js_value(&request)
        .map_err(|e| format!("Serialize error: {:?}", e))?;

    JsFuture::from(runtime_send_message(js_value))
        .await
        .map_err(|e| format!("Send failed: {:?}", e))?;

    Ok(())
}

/// Send ping to check connection
pub async fn send_ping() -> Result<(), String> {
    send_internal(InternalMessage::Ping {
        timestamp: js_sys::Date::now() as u64,
    })
    .await
}

// ============================================
// Browser Tool Messages (Legacy - Deprecated)
// ============================================

// Note: Browser tool execution now uses bg:exec_tool via exec_browser_tool()
// These legacy functions are kept for backward compatibility but should not be used

/// Browser tool request message for forwarding to background.js
/// @deprecated Use exec_browser_tool() instead
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", content = "payload", rename_all = "snake_case")]
pub enum BrowserToolMessage {
    /// Forward browser tool request to content script
    BrowserToolRequest(shared_protocol::BrowserToolRequestPayload),
}

/// Forward browser tool request to background.js for execution (async version)
/// @deprecated Use exec_browser_tool() instead
pub async fn forward_browser_tool_request(
    payload: shared_protocol::BrowserToolRequestPayload,
) -> Result<(), String> {
    // Use the new bg:exec_tool API
    let _ = exec_browser_tool(payload).await?;
    Ok(())
}

/// Forward browser tool request to background.js for execution (sync, fire-and-forget)
/// @deprecated Use exec_browser_tool() with async spawn instead
pub fn forward_browser_tool_request_sync(
    payload: &shared_protocol::BrowserToolRequestPayload,
) -> Result<(), String> {
    // Keep legacy behavior for backward compatibility
    let message = BrowserToolMessage::BrowserToolRequest(payload.clone());
    crate::messaging::bridge::send_message_sync(&message)
}
