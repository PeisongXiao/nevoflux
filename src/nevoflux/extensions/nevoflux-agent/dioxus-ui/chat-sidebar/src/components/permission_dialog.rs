/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

//! Permission request dialog component (P0 - Human in the Loop)

use dioxus::prelude::*;
use shared_protocol::PermissionScope;
use crate::context::use_app_context;
use crate::state::permission::{format_countdown, get_resource_icon};

/// Permission dialog component
#[component]
pub fn PermissionDialog() -> Element {
    let mut ctx = use_app_context();
    let request = ctx.permission_request.read();

    // Don't render if no request
    let Some(ref req) = *request else {
        return rsx! {};
    };

    // Countdown state
    let remaining_secs = req.remaining_secs();
    let selected_scope = use_signal(|| PermissionScope::Once);

    // Check for expiry
    if req.is_expired() {
        // Auto-deny on timeout
        let request_id = req.request_id.clone();
        let mock_enabled = ctx.mock_enabled;
        wasm_bindgen_futures::spawn_local(async move {
            if !mock_enabled {
                let _ = crate::messaging::send_permission_response(request_id, false, None).await;
            }
            ctx.permission_request.set(None);
            ctx.agent_status.write().set_error("Permission request timed out");
        });
        return rsx! {};
    }

    // Clone values for closures
    let request_id = req.request_id.clone();
    let resource_icon = get_resource_icon(&req.resource_type);
    let action_text = format!("{:?} {:?}", req.action, req.resource_type);
    let resource = req.resource.clone();
    let requester = req.requester.clone();
    let reason = req.reason.clone();

    // Handle deny
    let handle_deny = {
        let request_id = request_id.clone();
        let mock_enabled = ctx.mock_enabled;
        move |_| {
            let request_id = request_id.clone();
            wasm_bindgen_futures::spawn_local(async move {
                if !mock_enabled {
                    let _ = crate::messaging::send_permission_response(request_id, false, None).await;
                }
            });

            // Clear request and update status
            ctx.permission_request.set(None);
            ctx.agent_status.write().set_error("Permission denied");
        }
    };

    // Handle allow
    let handle_allow = {
        let request_id = request_id.clone();
        let mock_enabled = ctx.mock_enabled;
        move |_| {
            let request_id = request_id.clone();
            let scope = Some(selected_scope());
            wasm_bindgen_futures::spawn_local(async move {
                if !mock_enabled {
                    let _ = crate::messaging::send_permission_response(request_id, true, scope).await;
                }
            });

            // Clear request and update status
            ctx.permission_request.set(None);
            ctx.agent_status.write().set_executing();
        }
    };

    // Focus trap: handle Escape key to deny and close
    let handle_keydown = {
        let request_id = request_id.clone();
        let mock_enabled = ctx.mock_enabled;
        move |evt: KeyboardEvent| {
            if evt.key() == Key::Escape {
                // Deny on Escape
                let request_id = request_id.clone();
                wasm_bindgen_futures::spawn_local(async move {
                    if !mock_enabled {
                        let _ = crate::messaging::send_permission_response(request_id, false, None).await;
                    }
                });
                ctx.permission_request.set(None);
                ctx.agent_status.write().set_error("Permission denied");
            }
        }
    };

    rsx! {
        // Modal overlay
        div {
            class: "modal-overlay",
            onkeydown: handle_keydown,

            // Dialog
            div {
                class: "permission-dialog",
                role: "alertdialog",
                aria_modal: "true",
                aria_labelledby: "perm-title",
                tabindex: "-1",

                // Title
                h2 { id: "perm-title", class: "dialog-title",
                    span { class: "warning-icon", "⚠" }
                    "Permission Required"
                }

                // Resource card
                div { class: "resource-card",
                    span { class: "resource-icon", "{resource_icon}" }
                    div { class: "resource-info",
                        div { class: "resource-action", "{action_text}" }
                        div { class: "resource-path", "{resource}" }
                    }
                }

                // Request metadata
                div { class: "request-meta",
                    p { "Requested by: {requester}" }
                    p { class: "request-reason", "{reason}" }
                }

                // Scope selector
                fieldset { class: "scope-selector",
                    legend { class: "sr-only", "Permission scope" }

                    ScopeOption {
                        value: PermissionScope::Once,
                        label: "Allow once",
                        selected: selected_scope,
                    }
                    ScopeOption {
                        value: PermissionScope::Session,
                        label: "Allow for this session",
                        selected: selected_scope,
                    }
                    ScopeOption {
                        value: PermissionScope::Always,
                        label: "Always allow this type",
                        selected: selected_scope,
                    }
                }

                // Action buttons
                div { class: "dialog-actions",
                    button {
                        class: "btn-secondary",
                        onclick: handle_deny,
                        "Deny"
                    }
                    button {
                        class: "btn-primary",
                        onclick: handle_allow,
                        "Allow"
                    }
                }

                // Countdown
                div { class: "countdown",
                    "{format_countdown(remaining_secs)}"
                }
            }
        }
    }
}

/// Scope option radio button
#[component]
fn ScopeOption(
    value: PermissionScope,
    label: &'static str,
    mut selected: Signal<PermissionScope>,
) -> Element {
    let is_selected = selected() == value;

    rsx! {
        label {
            class: "scope-option",
            class: if is_selected { "selected" },

            input {
                r#type: "radio",
                name: "scope",
                checked: is_selected,
                onchange: move |_| selected.set(value.clone()),
            }
            span { "{label}" }
        }
    }
}
