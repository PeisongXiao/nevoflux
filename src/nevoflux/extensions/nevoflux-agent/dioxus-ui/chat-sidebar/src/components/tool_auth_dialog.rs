/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

//! Tool authorization dialog for human-in-the-loop tool approval

use dioxus::prelude::*;
use shared_protocol::AuthScope;
use crate::context::use_app_context;
use crate::state::get_tool_icon;
use wasm_bindgen_futures::spawn_local;

/// Tool authorization dialog shown when a tool requires user approval
#[component]
pub fn ToolAuthDialog() -> Element {
    let mut ctx = use_app_context();
    let pending = ctx.pending_tool_auth.read();

    let Some(request) = pending.as_ref() else {
        return rsx! {};
    };

    let tool_icon = get_tool_icon(&request.tool);
    let detail = request.detail.clone();
    let tool_id = request.tool_id.clone();
    let options = request.options.clone();

    rsx! {
        div {
            class: "tool-auth-overlay",
            role: "dialog",
            aria_modal: "true",
            aria_label: "Tool Authorization Required",

            div { class: "tool-auth-dialog",
                // Header
                div { class: "tool-auth-header",
                    span { class: "tool-auth-lock", "\u{1F512}" }
                    span { class: "tool-auth-title", "Authorization Required" }
                }

                // Tool details card
                div { class: "tool-auth-card",
                    span { class: "tool-auth-icon", "{tool_icon}" }
                    span { class: "tool-auth-detail", "{detail}" }
                }

                // Authorization options
                div { class: "tool-auth-options",
                    for (i, option) in options.iter().enumerate() {
                        {
                            let tool_id = tool_id.clone();
                            let scope = option.scope.clone();
                            let label = option.label.clone();
                            let is_deny = option.scope == AuthScope::Deny;
                            let is_persistent = matches!(option.scope, AuthScope::Always | AuthScope::Session);

                            rsx! {
                                button {
                                    class: if is_deny { "tool-auth-btn tool-auth-btn--deny" }
                                           else if is_persistent { "tool-auth-btn tool-auth-btn--persistent" }
                                           else { "tool-auth-btn" },
                                    onclick: {
                                        let tool_id = tool_id.clone();
                                        let scope = scope.clone();
                                        move |_| {
                                            let tool_id = tool_id.clone();
                                            let scope = scope.clone();
                                            // Clear dialog immediately
                                            ctx.pending_tool_auth.set(None);
                                            // Send response
                                            spawn_local(async move {
                                                if let Err(e) = crate::messaging::send_tool_auth_response(
                                                    tool_id, i as u32, scope,
                                                ).await {
                                                    tracing::error!("Failed to send tool auth response: {}", e);
                                                }
                                            });
                                        }
                                    },
                                    "{label}"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
