/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

//! Agent status state

use shared_protocol::AgentState;

/// Agent execution status for UI display
#[derive(Debug, Clone, Default)]
pub struct AgentStatusState {
    /// Current agent state
    pub state: AgentState,
    /// Currently executing tool
    pub current_tool: Option<ToolDisplayInfo>,
    /// Step progress information
    pub step: Option<StepDisplayInfo>,
    /// Error message if in error state
    pub error_message: Option<String>,
    /// Whether the status bar should be visible
    pub visible: bool,
}

/// Tool information for display
#[derive(Debug, Clone)]
pub struct ToolDisplayInfo {
    /// Tool name
    pub name: String,
    /// Display icon (emoji)
    pub icon: &'static str,
    /// Optional description or target
    pub description: Option<String>,
}

/// Step progress information
#[derive(Debug, Clone)]
pub struct StepDisplayInfo {
    /// Current step number
    pub current: u32,
    /// Total steps
    pub total: u32,
}

impl AgentStatusState {
    /// Check if agent is actively working
    pub fn is_active(&self) -> bool {
        matches!(
            self.state,
            AgentState::Thinking
                | AgentState::Executing
                | AgentState::ExecutingTool
                | AgentState::Waiting
                | AgentState::WaitingResult
                | AgentState::WaitingConfirmation
        )
    }

    /// Get state display label
    pub fn state_label(&self) -> &'static str {
        match self.state {
            AgentState::Idle => "Ready",
            AgentState::Thinking => "Thinking...",
            AgentState::Executing | AgentState::ExecutingTool => "Executing",
            AgentState::Waiting | AgentState::WaitingResult => "Waiting",
            AgentState::WaitingConfirmation => "Waiting for confirmation",
            AgentState::Complete => "Complete",
            AgentState::Error => "Error",
        }
    }

    /// Set to thinking state
    pub fn set_thinking(&mut self) {
        self.state = AgentState::Thinking;
        self.current_tool = None;
        self.step = None;
        self.error_message = None;
        self.visible = true;
    }

    /// Set to executing state
    pub fn set_executing(&mut self) {
        self.state = AgentState::Executing;
        self.visible = true;
    }

    /// Set to waiting state
    pub fn set_waiting(&mut self) {
        self.state = AgentState::Waiting;
        self.visible = true;
    }

    /// Set to complete state
    pub fn set_complete(&mut self) {
        self.state = AgentState::Complete;
        self.visible = true;
    }

    /// Set to error state
    pub fn set_error(&mut self, message: &str) {
        self.state = AgentState::Error;
        self.error_message = Some(message.to_string());
        self.visible = true;
    }

    /// Hide the status bar and reset to idle state
    pub fn hide(&mut self) {
        self.state = AgentState::Idle;
        self.current_tool = None;
        self.step = None;
        self.error_message = None;
        self.visible = false;
    }

    /// Reset to initial state (for tab switching)
    pub fn reset(&mut self) {
        self.state = AgentState::Idle;
        self.current_tool = None;
        self.step = None;
        self.error_message = None;
        self.visible = false;
    }
}

/// Get display icon for a tool name
pub fn get_tool_icon(name: &str) -> &'static str {
    match name {
        // === CLI / Agent file tools ===
        "Read" | "read" => "\u{1F4C4}",              // 📄
        "Write" | "Edit" | "write" | "edit" => "\u{270F}\u{FE0F}", // ✏️
        "NotebookEdit" => "\u{1F4D3}",               // 📓

        // === Shell ===
        "Bash" | "bash" | "browser_eval_js" => "\u{1F4BB}", // 💻

        // === Search ===
        "Grep" | "Glob" | "grep" | "glob"
        | "browser_get_elements" | "browser_find_elements"
        | "browser_element_info" => "\u{1F50D}",     // 🔍

        // === Web ===
        "WebFetch" | "WebSearch" | "web_fetch" | "web_search"
        | "browser_navigate" | "navigate" | "goto" | "open_url" => "\u{1F310}", // 🌐

        // === Agent tools ===
        "Task" | "think" => "\u{1F4AD}",             // 💭
        "plan" => "\u{1F4DD}",                       // 📝
        "switch_model" => "\u{1F504}",               // 🔄
        "ask_user" => "\u{2753}",                    // ❓
        "memory_search" => "\u{1F9E0}",              // 🧠
        "skill_load" => "\u{1F4E6}",                 // 📦
        "tool_search" | "tool_call_dynamic" => "\u{1F50E}", // 🔎
        "subagent_spawn" | "subagent_status" | "subagent_wait"
        | "subagent_kill" | "subagent_list" => "\u{1F916}", // 🤖

        // === Browser tools ===
        "browser_click" | "browser_click_by_id"
        | "click_element" | "click" => "\u{1F5B1}\u{FE0F}", // 🖱️
        "browser_type" | "browser_type_by_id"
        | "browser_fill" | "browser_fill_by_id"
        | "type_text" | "type" | "input" => "\u{2328}\u{FE0F}", // ⌨️
        "browser_screenshot" | "screenshot" | "capture" => "\u{1F4F7}", // 📷
        "browser_scroll" | "scroll" | "scroll_page" => "\u{2195}\u{FE0F}", // ↕️
        "browser_get_content" | "browser_get_markdown"
        | "extract_content" | "get_text" => "\u{1F4CB}", // 📋
        "browser_wait_for" | "wait" | "sleep" | "waitForStable" => "\u{23F1}\u{FE0F}", // ⏱️

        _ => "\u{2699}\u{FE0F}",                    // ⚙️
    }
}

/// Extract a human-readable target from tool arguments JSON
pub fn extract_tool_target(name: &str, arguments: &str) -> Option<String> {
    let args: serde_json::Value = serde_json::from_str(arguments).ok()?;
    match name {
        "Read" | "Write" | "Edit" => args.get("file_path")?.as_str().map(shorten_path),
        "Bash" => args.get("command")?.as_str().map(|s| truncate_str(s, 40)),
        "Grep" | "Glob" => args.get("pattern")?.as_str().map(|s| truncate_str(s, 30)),
        "click" | "click_element" => args.get("selector")?.as_str().map(|s| truncate_str(s, 30)),
        "navigate" | "goto" | "open_url" => args.get("url")?.as_str().map(|s| truncate_str(s, 40)),
        "screenshot" | "capture" => Some("viewport".to_string()),
        _ => None,
    }
}

/// Shorten a file path to its last 2 components
fn shorten_path(path: &str) -> String {
    let parts: Vec<&str> = path.rsplit('/').take(2).collect();
    if parts.len() == 2 && !parts[1].is_empty() {
        format!(".../{}/{}", parts[1], parts[0])
    } else {
        path.to_string()
    }
}

/// Truncate a string to max chars with ellipsis (UTF-8 safe)
fn truncate_str(s: &str, max: usize) -> String {
    let char_count = s.chars().count();
    if char_count <= max {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max.saturating_sub(3)).collect();
        format!("{truncated}...")
    }
}
