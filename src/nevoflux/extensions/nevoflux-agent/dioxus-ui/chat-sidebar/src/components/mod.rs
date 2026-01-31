/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

//! UI components for the Chat Sidebar

mod header;
mod message_area;
mod input_area;
mod agent_status;
mod permission_dialog;
mod mcp_config;
mod ask_user_dialog;

pub use header::Header;
pub use message_area::{MessageArea, MessageBubble, WelcomeScreen};
pub use input_area::InputArea;
pub use agent_status::AgentStatusBar;
pub use permission_dialog::PermissionDialog;
pub use mcp_config::McpConfigModal;
pub use ask_user_dialog::AskUserDialog;
