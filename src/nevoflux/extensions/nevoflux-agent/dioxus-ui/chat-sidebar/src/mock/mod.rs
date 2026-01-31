/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

//! Mock mode for development and testing
//!
//! Provides simulated responses without requiring a native agent connection.

mod config;
mod provider;

pub use config::MockConfig;
pub use provider::{MockProvider, MockResponse, StreamChunk};

use crate::context::AppContext;
use crate::state::*;
use dioxus::prelude::*;
use gloo::timers::future::TimeoutFuture;
use std::sync::atomic::{AtomicBool, Ordering};

/// Global stop flag for mock mode
static MOCK_STOPPED: AtomicBool = AtomicBool::new(false);

/// Check if mock streaming was stopped
pub fn is_mock_stopped() -> bool {
    MOCK_STOPPED.load(Ordering::SeqCst)
}

/// Stop mock streaming
pub fn stop_mock_streaming() {
    MOCK_STOPPED.store(true, Ordering::SeqCst);
}

/// Reset mock stop flag
fn reset_mock_stop() {
    MOCK_STOPPED.store(false, Ordering::SeqCst);
}

/// Initialize mock messaging
pub async fn init_mock_messaging(mut ctx: AppContext) {
    tracing::info!("Mock mode enabled");

    // Simulate connection delay
    TimeoutFuture::new(500).await;
    ctx.connection.set(ConnectionState::Connected);
}

/// Send a mock message and generate response
pub async fn mock_send_message(mut ctx: AppContext, text: String) {
    let config = MockConfig::from_url();
    let provider = MockProvider::new(config.clone());

    // Reset stop flag
    reset_mock_stop();

    // Show thinking state
    ctx.agent_status.write().set_thinking();

    // Simulate thinking delay (broken into small chunks to allow early stop)
    for _ in 0..10 {
        if is_mock_stopped() {
            ctx.agent_status.write().hide();
            return;
        }
        TimeoutFuture::new(100).await;
    }

    // Check if stopped during thinking
    if is_mock_stopped() {
        ctx.agent_status.write().hide();
        return;
    }

    // Generate and handle response
    match provider.generate_response(&text) {
        MockResponse::Stream(chunks) => {
            mock_stream_response(ctx, chunks).await;
        }
        MockResponse::Error(error) => {
            ctx.agent_status.write().set_error(&error.message);
            ctx.messages.write().push(Message::error(
                error.code,
                error.message,
                error.recoverable,
            ));
            // Auto-hide after delay
            TimeoutFuture::new(3000).await;
            ctx.agent_status.write().hide();
        }
        MockResponse::WithPermission { permission, response } => {
            // Show permission request
            ctx.permission_request.set(Some(PermissionRequestState {
                request_id: permission.request_id,
                resource_type: permission.resource_type,
                action: permission.action,
                resource: permission.resource,
                requester: permission.requester.name,
                reason: permission.reason,
                timeout_ms: permission.timeout_ms,
                created_at: js_sys::Date::now() as u64,
            }));
            ctx.agent_status.write().set_waiting();

            // Wait for permission to be handled
            while ctx.permission_request.read().is_some() {
                TimeoutFuture::new(100).await;
                if is_mock_stopped() {
                    ctx.permission_request.set(None);
                    ctx.agent_status.write().hide();
                    return;
                }
            }

            // Continue with response
            mock_stream_response(ctx, response).await;
        }
    }
}

/// Stream mock response chunks
async fn mock_stream_response(mut ctx: AppContext, chunks: Vec<StreamChunk>) {
    let stream_id = uuid::Uuid::new_v4().to_string();

    // Start streaming - show Executing state
    ctx.streaming.set(Some(StreamingState::new(&stream_id)));
    ctx.agent_status.write().set_executing();

    // Stream each chunk with delay
    for chunk in chunks {
        // Check if stopped
        if is_mock_stopped() {
            break;
        }

        TimeoutFuture::new(chunk.delay_ms as u32).await;

        // Check again after delay
        if is_mock_stopped() {
            break;
        }

        ctx.streaming.with_mut(|s| {
            if let Some(ref mut stream) = s {
                stream.content.push_str(&chunk.delta);
            }
        });
    }

    // Finalize - save partial or full content
    let final_content = ctx
        .streaming
        .read()
        .as_ref()
        .map(|s| s.content.clone())
        .unwrap_or_default();

    ctx.streaming.set(None);

    // Only add message if there's content
    if !final_content.is_empty() {
        ctx.messages.write().push(Message::assistant_markdown(final_content));
    }

    if is_mock_stopped() {
        // Was stopped - hide status immediately
        ctx.agent_status.write().hide();
    } else {
        // Completed normally
        ctx.agent_status.write().set_complete();
        // Auto-hide after delay
        TimeoutFuture::new(2000).await;
        ctx.agent_status.write().hide();
    }
}
