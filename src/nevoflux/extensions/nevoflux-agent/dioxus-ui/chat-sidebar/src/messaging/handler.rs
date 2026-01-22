/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

//! Handles incoming messages from background script
//!
//! Receives ChatMessage from Agent (via background.js) and InternalMessage
//! from background.js itself.

use dioxus::prelude::*;
use crate::context::AppContext;
use crate::messaging::bridge::*;
use crate::messaging::sender::*;
use crate::state::*;
use shared_protocol::*;
use wasm_bindgen::prelude::*;
use wasm_bindgen::closure::Closure;

/// Combined incoming message type
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(untagged)]
pub enum IncomingMessage {
    /// Agent protocol message (Chat channel - bidirectional)
    AgentMessage(ChatMessage),
    /// Extension internal message
    Internal(InternalMessage),
}

/// Initialize message listener
pub fn init_message_listener(ctx: AppContext) {
    let closure = Closure::<dyn Fn(JsValue, JsValue, JsValue) -> JsValue>::new(
        move |msg: JsValue, _sender: JsValue, _send_response: JsValue| {
            // Parse incoming message
            match from_js_value::<IncomingMessage>(msg) {
                Ok(incoming) => {
                    handle_incoming(ctx, incoming);
                }
                Err(e) => {
                    tracing::warn!("Failed to parse message: {}", e);
                }
            }

            // Return undefined (no async response)
            JsValue::UNDEFINED
        },
    );

    runtime_add_listener(&closure);

    // Keep closure alive for lifetime of app
    closure.forget();
}

/// Route incoming message to appropriate handler
fn handle_incoming(ctx: AppContext, message: IncomingMessage) {
    match message {
        IncomingMessage::AgentMessage(chat_msg) => {
            handle_chat_message(ctx, chat_msg);
        }
        IncomingMessage::Internal(internal) => {
            handle_internal_message(ctx, internal);
        }
    }
}

/// Handle Chat channel messages (bidirectional Agent <-> Sidebar)
///
/// Most messages from Agent are ToSidebar direction, but we also handle
/// BrowserToolRequest specially by executing via bg:exec_tool and sending response.
fn handle_chat_message(ctx: AppContext, message: ChatMessage) {
    match message {
        // ========== Agent -> Sidebar messages ==========
        ChatMessage::StreamChunk(payload) => {
            handle_stream_chunk(ctx, payload);
        }
        ChatMessage::StreamEnd(payload) => {
            handle_stream_end(ctx, payload);
        }
        ChatMessage::AgentState(payload) => {
            handle_agent_state(ctx, payload);
        }
        ChatMessage::PermissionRequest(payload) => {
            handle_permission_request(ctx, payload);
        }
        ChatMessage::Error(payload) => {
            handle_error(ctx, payload);
        }
        ChatMessage::ContentBlock(payload) => {
            handle_content_block(ctx, payload);
        }
        ChatMessage::AccountStatus(_payload) => {
            // P2: Handle account status
            tracing::debug!("Received account status - not yet implemented");
        }
        ChatMessage::SystemResponse(_payload) => {
            // Handle system command responses
            tracing::debug!("Received system response");
        }
        ChatMessage::BrowserToolRequest(payload) => {
            // Execute browser tool via bg:exec_tool and send response back to agent
            handle_browser_tool_request(payload);
        }

        // ========== Sidebar -> Agent messages (should not be received) ==========
        ChatMessage::ChatMessage(_) |
        ChatMessage::SkillCommand(_) |
        ChatMessage::StopGeneration(_) |
        ChatMessage::PermissionResponse(_) |
        ChatMessage::PluginCommand(_) |
        ChatMessage::SystemCommand(_) |
        ChatMessage::BrowserToolResponse(_) => {
            tracing::warn!("Received unexpected ToAgent message in sidebar");
        }
    }
}

/// Handle BrowserToolRequest from Agent
///
/// 1. Call bg:exec_tool to execute the browser tool
/// 2. Build response payload from result
/// 3. Send response via bg:send_to_agent
fn handle_browser_tool_request(payload: BrowserToolRequestPayload) {
    tracing::info!(
        "Executing browser tool: {} (action={:?}, tab_id={:?})",
        payload.request_id,
        payload.action,
        payload.tab_id
    );

    // Spawn async task to execute tool and send response
    dioxus::prelude::spawn(async move {
        // Execute browser tool via bg:exec_tool
        match crate::messaging::exec_browser_tool(payload.clone()).await {
            Ok(response) => {
                tracing::info!(
                    "Browser tool {} completed (success={})",
                    response.request_id,
                    response.success
                );

                // Send response back to agent via bg:send_to_agent
                if let Err(e) = crate::messaging::send_browser_tool_response(response).await {
                    tracing::error!("Failed to send browser tool response to agent: {}", e);
                }
            }
            Err(e) => {
                tracing::error!("Browser tool execution failed: {}", e);

                // Send error response back to agent
                let error_response = BrowserToolResponsePayload {
                    request_id: payload.request_id,
                    session_id: payload.session_id,
                    success: false,
                    result: None,
                    error: Some(BrowserToolError {
                        code: -1,
                        message: e,
                        recoverable: true,
                    }),
                };

                if let Err(e) = crate::messaging::send_browser_tool_response(error_response).await {
                    tracing::error!("Failed to send browser tool error response: {}", e);
                }
            }
        }
    });
}

// ============================================
// Stream Handlers
// ============================================

fn handle_stream_chunk(mut ctx: AppContext, payload: StreamChunkPayload) {
    let mut streaming = ctx.streaming.write();
    match &mut *streaming {
        Some(ref mut stream) if stream.id == payload.stream_id => {
            stream.content.push_str(&payload.delta);
        }
        _ => {
            // Start new stream
            *streaming = Some(StreamingState {
                id: payload.stream_id,
                content: payload.delta,
                format: payload.format,
            });
        }
    }
}

fn handle_stream_end(mut ctx: AppContext, payload: StreamEndPayload) {
    // Finalize stream into message
    let final_content = {
        let mut streaming = ctx.streaming.write();
        if let Some(stream) = streaming.take() {
            if stream.id == payload.stream_id {
                Some((stream.content, stream.format))
            } else {
                None
            }
        } else {
            None
        }
    };

    if let Some((content, format)) = final_content {
        let message = match format {
            StreamFormat::Markdown => Message::assistant_markdown(content),
            _ => Message::assistant(content),
        };

        ctx.messages.write().push(message);
    }
}

// ============================================
// Agent State Handler
// ============================================

fn handle_agent_state(mut ctx: AppContext, payload: AgentStatePayload) {
    let mut status = ctx.agent_status.write();
    status.state = payload.state.clone();

    // Update tool info
    status.current_tool = payload.tool.map(|t| ToolDisplayInfo {
        name: t.name.clone(),
        icon: get_tool_icon(&t.name),
        description: t.target,
    });

    // Update step info
    status.step = payload.step.map(|s| StepDisplayInfo {
        current: s.current,
        total: s.total,
    });

    // Visibility
    status.visible = !matches!(payload.state, AgentState::Complete);
}

// ============================================
// Permission Request Handler
// ============================================

fn handle_permission_request(mut ctx: AppContext, payload: PermissionRequestPayload) {
    // Set agent to waiting state
    {
        let mut status = ctx.agent_status.write();
        status.state = AgentState::Waiting;
        status.visible = true;
    }

    // Show permission dialog
    ctx.permission_request.set(Some(PermissionRequestState {
        request_id: payload.request_id,
        resource_type: payload.resource_type,
        action: payload.action,
        resource: payload.resource,
        requester: payload.requester.name,
        reason: payload.reason,
        timeout_ms: payload.timeout_ms,
        created_at: js_sys::Date::now() as u64,
    }));
}

// ============================================
// Error Handler
// ============================================

fn handle_error(mut ctx: AppContext, payload: ErrorPayload) {
    // Update agent status
    {
        let mut status = ctx.agent_status.write();
        status.state = AgentState::Error;
        status.error_message = Some(payload.message.clone());
        status.visible = true;
    }

    // Clear any streaming
    ctx.streaming.set(None);

    // Add error message to chat
    ctx.messages.write().push(Message::error(
        payload.code,
        payload.message,
        payload.recoverable,
    ));
}

// ============================================
// Content Block Handler
// ============================================

fn handle_content_block(mut ctx: AppContext, payload: ContentBlockPayload) {
    let message = match payload.content_type {
        ContentType::Text => Message::assistant(
            payload.content.as_str().unwrap_or_default().to_string(),
        ),
        ContentType::Markdown => Message::assistant_markdown(
            payload.content.as_str().unwrap_or_default().to_string(),
        ),
        ContentType::Code => {
            let language = payload
                .metadata
                .as_ref()
                .and_then(|m| m.get("language"))
                .and_then(|v| v.as_str())
                .unwrap_or("text")
                .to_string();
            Message::code(language, payload.content.as_str().unwrap_or_default())
        }
        ContentType::A2ui | ContentType::Image => {
            // P2: placeholder for now
            Message::assistant("[Content block not yet supported]")
        }
    };

    ctx.messages.write().push(message);
}

// ============================================
// Internal Message Handler
// ============================================

fn handle_internal_message(mut ctx: AppContext, message: InternalMessage) {
    match message {
        InternalMessage::Pong { .. } => {
            ctx.connection.set(ConnectionState::Connected);
        }
        InternalMessage::ConnectionStatus { connected } => {
            if connected {
                ctx.connection.set(ConnectionState::Connected);
            } else {
                ctx.connection.set(ConnectionState::Disconnected);
            }
        }
        InternalMessage::TabContextUpdate(payload) => {
            ctx.tab_context.set(TabContext {
                tab_id: payload.tab_id,
                url: payload.url,
                title: payload.title,
                favicon_url: payload.favicon_url,
            });
        }
        // Outgoing-only messages (shouldn't receive)
        InternalMessage::Ping { .. } => {
            tracing::warn!("Received unexpected outgoing Ping message");
        }
    }
}
