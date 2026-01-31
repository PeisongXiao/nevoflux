/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

//! Agent status bar component

use dioxus::prelude::*;
use shared_protocol::AgentState;
use crate::context::use_app_context;

/// Agent status bar component
#[component]
pub fn AgentStatusBar() -> Element {
    let ctx = use_app_context();
    let status = ctx.agent_status.read();

    // Don't render if not visible
    if !status.visible {
        return rsx! {};
    }

    let state_class = match status.state {
        AgentState::Idle => "idle",
        AgentState::Thinking => "thinking",
        AgentState::Executing | AgentState::ExecutingTool => "executing",
        AgentState::Waiting | AgentState::WaitingResult | AgentState::WaitingConfirmation => "waiting",
        AgentState::Complete => "complete",
        AgentState::Error => "error",
    };

    rsx! {
        div {
            class: "agent-status-bar {state_class}",
            role: "status",
            aria_live: "polite",
            aria_atomic: "true",

            // Left side: indicator + label + tool
            div { class: "status-left",
                StatusIndicator { state: status.state.clone() }
                span { class: "status-label", "{status.state_label()}" }

                // Current tool info
                if let Some(ref tool) = status.current_tool {
                    span { class: "current-tool",
                        span { class: "tool-icon", "{tool.icon}" }
                        span { class: "tool-name", "{tool.name}" }
                    }
                }
            }

            // Right side: step progress + stop button
            div { class: "status-right",
                // Step progress
                if let Some(ref step) = status.step {
                    span { class: "step-progress",
                        "Step {step.current}/{step.total}"
                    }
                }

                // Stop button (only when active)
                if status.is_active() {
                    StopButton {}
                }
            }
        }
    }
}

/// Status indicator with animation
#[component]
fn StatusIndicator(state: AgentState) -> Element {
    rsx! {
        span { class: "status-indicator",
            match state {
                AgentState::Idle => rsx! {
                    span { class: "indicator-dot idle" }
                },
                AgentState::Thinking => rsx! {
                    span { class: "indicator-dot pulsing" }
                },
                AgentState::Executing | AgentState::ExecutingTool => rsx! {
                    span { class: "indicator-spinner" }
                },
                AgentState::Waiting | AgentState::WaitingResult => rsx! {
                    span { class: "indicator-dot waiting" }
                },
                AgentState::WaitingConfirmation => rsx! {
                    span { class: "indicator-dot waiting" }
                },
                AgentState::Complete => rsx! {
                    span { class: "indicator-check", "✓" }
                },
                AgentState::Error => rsx! {
                    span { class: "indicator-error", "✗" }
                },
            }
        }
    }
}

/// Stop button component
#[component]
fn StopButton() -> Element {
    let mut ctx = use_app_context();

    let handle_stop = move |_| {
        if ctx.mock_enabled {
            // Mock mode: use mock stop function
            crate::mock::stop_mock_streaming();
        } else {
            // Real mode: send stop message to agent
            let session_id = ctx.tab_context.read().zen_sync_id.clone()
                .unwrap_or_else(|| ctx.session.read().id.clone());
            spawn(async move {
                let _ = crate::messaging::send_stop_generation(&session_id).await;
            });
            // Update status immediately in real mode
            ctx.agent_status.write().hide();
        }
    };

    rsx! {
        button {
            class: "stop-button",
            onclick: handle_stop,
            aria_label: "Stop generation",
            title: "Stop",
            "Stop"
        }
    }
}
