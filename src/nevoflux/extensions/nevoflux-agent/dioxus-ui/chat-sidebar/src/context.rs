/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

//! Application context and state management
//!
//! Provides a global context using Dioxus signals that can be accessed
//! by any component in the tree.

use dioxus::prelude::*;

use crate::state::{
    AgentStatusState, AskUserState, ConnectionState, HistoryState, McpConfigState, Message,
    PendingFilePick, PermissionRequestState, PickedFile, SessionState, StreamingState, TabContext,
};
use shared_protocol::ChatMode;

/// Global application context
///
/// Contains all shared state as Dioxus signals that can be read and written
/// from any component using `use_app_context()`.
#[derive(Clone, Copy, PartialEq)]
pub struct AppContext {
    /// Session state
    pub session: Signal<SessionState>,
    /// Chat messages
    pub messages: Signal<Vec<Message>>,
    /// Currently streaming message
    pub streaming: Signal<Option<StreamingState>>,
    /// Agent status
    pub agent_status: Signal<AgentStatusState>,
    /// Connection status
    pub connection: Signal<ConnectionState>,
    /// Active permission request
    pub permission_request: Signal<Option<PermissionRequestState>>,
    /// Current tab context
    pub tab_context: Signal<TabContext>,
    /// Session history list
    pub history: Signal<HistoryState>,
    /// MCP server configuration state
    pub mcp_config: Signal<McpConfigState>,
    /// Whether to show the MCP config modal
    pub show_mcp_config: Signal<bool>,
    /// Pending AskUser request from agent
    pub ask_user: Signal<Option<AskUserState>>,
    /// Files picked via native file dialog
    pub picked_files: Signal<Vec<PickedFile>>,
    /// Pending file pick request
    pub pending_file_pick: Signal<Option<PendingFilePick>>,
    /// Current chat mode (chat, browser, agent)
    pub chat_mode: Signal<ChatMode>,
    /// Whether mock mode is enabled
    pub mock_enabled: bool,
}

/// Context provider component
///
/// Wraps children with the application context, initializing all state
/// and setting up message listeners.
#[component]
pub fn ContextProvider(#[props(default = false)] mock_enabled: bool, children: Element) -> Element {
    // Initialize all state signals
    let session = use_signal(SessionState::new);
    let messages = use_signal(Vec::<Message>::new);
    let streaming = use_signal(|| None::<StreamingState>);
    let agent_status = use_signal(AgentStatusState::default);
    let connection = use_signal(|| ConnectionState::Disconnected);
    let permission_request = use_signal(|| None::<PermissionRequestState>);
    let tab_context = use_signal(TabContext::default);
    let history = use_signal(HistoryState::default);
    let mcp_config = use_signal(McpConfigState::default);
    let show_mcp_config = use_signal(|| false);
    let ask_user = use_signal(|| None::<AskUserState>);
    let picked_files = use_signal(Vec::<PickedFile>::new);
    let pending_file_pick = use_signal(|| None::<PendingFilePick>);
    let chat_mode = use_signal(ChatMode::default);

    // Build context
    let mut ctx = AppContext {
        session,
        messages,
        streaming,
        agent_status,
        connection,
        permission_request,
        tab_context,
        history,
        mcp_config,
        show_mcp_config,
        ask_user,
        picked_files,
        pending_file_pick,
        chat_mode,
        mock_enabled,
    };

    // Provide context to children
    use_context_provider(|| ctx);

    // Initialize messaging on mount
    use_effect(move || {
        if mock_enabled {
            // Mock mode: simulate connection
            spawn(async move {
                crate::mock::init_mock_messaging(ctx).await;
            });
        } else {
            // Real mode: set up message listener and connect
            crate::messaging::init_message_listener(ctx);
            spawn(async move {
                // Send ping and request tab context
                let _ = crate::messaging::send_ping().await;
                let _ = crate::messaging::request_tab_context().await;
                // Request session list for history
                ctx.history.write().set_loading();
                let _ = crate::messaging::send_session_list(50, 0).await;
            });
        }
    });

    rsx! { {children} }
}

/// Hook to access the application context
///
/// Must be called from within a component that is a descendant of `ContextProvider`.
pub fn use_app_context() -> AppContext {
    use_context::<AppContext>()
}

/// Check if mock mode is enabled from URL parameters
pub fn is_mock_mode() -> bool {
    // Check URL parameter first
    let url_mock = web_sys::window()
        .and_then(|w| w.location().search().ok())
        .map(|s| s.contains("mock=true"))
        .unwrap_or(false);

    if url_mock {
        return true;
    }

    // Auto-detect: if browser.runtime API is not available, we're not in extension context
    // Fall back to mock mode for development
    let has_browser_api = js_sys::Reflect::get(
        &js_sys::global(),
        &wasm_bindgen::JsValue::from_str("browser"),
    )
    .ok()
    .map(|b| !b.is_undefined() && !b.is_null())
    .unwrap_or(false);

    if !has_browser_api {
        tracing::warn!("browser.runtime API not available - auto-enabling mock mode");
        tracing::info!("Tip: Add ?mock=true to URL for explicit mock mode");
        return true;
    }

    false
}
