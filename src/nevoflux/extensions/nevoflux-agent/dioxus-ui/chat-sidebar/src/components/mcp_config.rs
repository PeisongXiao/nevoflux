/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

//! MCP Server Configuration UI Components

use dioxus::prelude::*;
use wasm_bindgen_futures::spawn_local;
use crate::context::use_app_context;
use crate::state::{McpConnectionStatus, McpServer, McpServerConfig};

/// Full-screen MCP configuration modal
#[component]
pub fn McpConfigModal() -> Element {
    let ctx = use_app_context();
    let show = ctx.show_mcp_config.read();

    if !*show {
        return rsx! {};
    }

    let mcp_state = ctx.mcp_config.read();
    let is_editing = mcp_state.is_editing();

    rsx! {
        div {
            class: "mcp-config-modal",
            role: "dialog",
            aria_modal: "true",
            aria_label: "MCP Server Configuration",

            // Header
            McpConfigHeader {}

            // Content area
            div { class: "mcp-config-content",
                if is_editing {
                    McpServerForm {}
                } else {
                    McpServerList {}
                }
            }
        }
    }
}

/// Header for the MCP config modal
#[component]
fn McpConfigHeader() -> Element {
    let mut ctx = use_app_context();
    let mcp_state = ctx.mcp_config.read();
    let is_editing = mcp_state.is_editing();
    let is_adding = mcp_state.is_adding;

    let title = if is_adding {
        "Add MCP Server"
    } else if is_editing {
        "Edit MCP Server"
    } else {
        "MCP Servers"
    };

    let handle_close = move |_| {
        if is_editing {
            ctx.mcp_config.write().cancel_edit();
        } else {
            ctx.show_mcp_config.set(false);
        }
    };

    let handle_back = move |_| {
        ctx.mcp_config.write().cancel_edit();
    };

    rsx! {
        header { class: "mcp-config-header",
            div { class: "mcp-header-left",
                if is_editing {
                    button {
                        class: "mcp-back-btn",
                        onclick: handle_back,
                        aria_label: "Back to server list",
                        // Back arrow icon
                        svg {
                            width: "20",
                            height: "20",
                            view_box: "0 0 24 24",
                            fill: "none",
                            stroke: "currentColor",
                            stroke_width: "2",
                            stroke_linecap: "round",
                            stroke_linejoin: "round",
                            path { d: "M19 12H5" }
                            path { d: "M12 19l-7-7 7-7" }
                        }
                    }
                }
                h2 { class: "mcp-config-title", "{title}" }
            }

            button {
                class: "mcp-close-btn",
                onclick: handle_close,
                aria_label: "Close",
                // X icon
                svg {
                    width: "20",
                    height: "20",
                    view_box: "0 0 24 24",
                    fill: "none",
                    stroke: "currentColor",
                    stroke_width: "2",
                    stroke_linecap: "round",
                    stroke_linejoin: "round",
                    path { d: "M18 6L6 18" }
                    path { d: "M6 6l12 12" }
                }
            }
        }
    }
}

/// List of configured MCP servers
#[component]
fn McpServerList() -> Element {
    let mut ctx = use_app_context();
    let mcp_state = ctx.mcp_config.read();

    let handle_add = move |_| {
        ctx.mcp_config.write().start_add();
    };

    rsx! {
        div { class: "mcp-server-list",
            // Add button
            button {
                class: "mcp-add-server-btn",
                onclick: handle_add,
                // Plus icon
                svg {
                    width: "20",
                    height: "20",
                    view_box: "0 0 24 24",
                    fill: "none",
                    stroke: "currentColor",
                    stroke_width: "2",
                    stroke_linecap: "round",
                    stroke_linejoin: "round",
                    path { d: "M12 5v14" }
                    path { d: "M5 12h14" }
                }
                span { "Add Server" }
            }

            // Loading state
            if mcp_state.loading {
                div { class: "mcp-loading",
                    span { class: "loading-spinner" }
                    span { "Loading servers..." }
                }
            }

            // Error state
            if let Some(ref error) = mcp_state.error {
                div { class: "mcp-error",
                    span { class: "mcp-error-icon", "!" }
                    span { "{error}" }
                }
            }

            // Server cards
            if !mcp_state.loading && mcp_state.error.is_none() {
                if mcp_state.servers.is_empty() {
                    div { class: "mcp-empty-state",
                        p { "No MCP servers configured." }
                        p { class: "mcp-empty-hint", "Add a server to extend the agent's capabilities." }
                    }
                } else {
                    div { class: "mcp-server-cards",
                        for server in mcp_state.servers.iter() {
                            McpServerCard {
                                key: "{server.config.name}",
                                server: server.clone(),
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Individual server card component
#[component]
fn McpServerCard(server: McpServer) -> Element {
    let mut ctx = use_app_context();
    let server_name = server.config.name.clone();
    let is_connected = server.status.is_connected();
    let is_enabled = server.config.enabled;

    let status_class = match &server.status {
        McpConnectionStatus::Connected => "connected",
        McpConnectionStatus::Connecting => "connecting",
        McpConnectionStatus::Disconnected => "disconnected",
        McpConnectionStatus::Error(_) => "error",
    };

    let status_text = match &server.status {
        McpConnectionStatus::Connected => "Connected",
        McpConnectionStatus::Connecting => "Connecting...",
        McpConnectionStatus::Disconnected => "Disconnected",
        McpConnectionStatus::Error(msg) => msg.as_str(),
    };

    let handle_edit = {
        let name = server_name.clone();
        move |_| {
            ctx.mcp_config.write().start_edit(&name);
        }
    };

    let handle_delete = {
        let name = server_name.clone();
        move |_| {
            let name = name.clone();
            spawn_local(async move {
                let _ = crate::messaging::send_mcp_delete(&name).await;
            });
        }
    };

    let handle_test = {
        let name = server_name.clone();
        move |_| {
            let name = name.clone();
            spawn_local(async move {
                let _ = crate::messaging::send_mcp_test(&name).await;
            });
        }
    };

    let handle_toggle_connect = {
        let name = server_name.clone();
        let connected = is_connected;
        move |_| {
            let name = name.clone();
            spawn_local(async move {
                if connected {
                    let _ = crate::messaging::send_mcp_disconnect(&name).await;
                } else {
                    let _ = crate::messaging::send_mcp_connect(&name).await;
                }
            });
        }
    };

    let handle_toggle_enabled = {
        let config = server.config.clone();
        move |_| {
            let config = config.clone();
            spawn_local(async move {
                let _ = crate::messaging::send_mcp_update(
                    &config.name,
                    &config.command,
                    config.args.clone(),
                    !config.enabled,
                    config.env.clone(),
                ).await;
            });
        }
    };

    // Get test result for this server
    let test_result = ctx.mcp_config.read().test_result.clone();
    let show_test_result = test_result
        .as_ref()
        .map(|(name, _, _)| name == &server_name)
        .unwrap_or(false);

    rsx! {
        div {
            class: "mcp-server-card",
            class: if !is_enabled { "disabled" },

            // Header row with name and status
            div { class: "mcp-card-header",
                div { class: "mcp-card-name-row",
                    span { class: "mcp-server-name", "{server.config.name}" }
                    span {
                        class: "mcp-status-badge {status_class}",
                        "{status_text}"
                    }
                }

                // Enable/Disable toggle
                label { class: "mcp-toggle-label",
                    input {
                        r#type: "checkbox",
                        checked: is_enabled,
                        onchange: handle_toggle_enabled,
                        class: "mcp-toggle-input",
                    }
                    span { class: "mcp-toggle-switch" }
                }
            }

            // Command info
            div { class: "mcp-card-command",
                code { "{server.config.command}" }
                if !server.config.args.is_empty() {
                    span { class: "mcp-card-args",
                        " {server.config.args.join(\" \")}"
                    }
                }
            }

            // Test result (if any)
            if show_test_result {
                if let Some((_, success, message)) = &test_result {
                    div {
                        class: "mcp-test-result",
                        class: if *success { "success" } else { "failure" },
                        "{message}"
                    }
                }
            }

            // Action buttons
            div { class: "mcp-card-actions",
                button {
                    class: "mcp-action-btn",
                    onclick: handle_test,
                    title: "Test connection",
                    // Test/beaker icon
                    svg {
                        width: "16",
                        height: "16",
                        view_box: "0 0 24 24",
                        fill: "none",
                        stroke: "currentColor",
                        stroke_width: "2",
                        stroke_linecap: "round",
                        stroke_linejoin: "round",
                        path { d: "M9 3h6" }
                        path { d: "M9 3v6l-4 8h14l-4-8V3" }
                    }
                }

                button {
                    class: "mcp-action-btn",
                    class: if is_connected { "connected" },
                    onclick: handle_toggle_connect,
                    title: if is_connected { "Disconnect" } else { "Connect" },
                    // Power/plug icon
                    svg {
                        width: "16",
                        height: "16",
                        view_box: "0 0 24 24",
                        fill: "none",
                        stroke: "currentColor",
                        stroke_width: "2",
                        stroke_linecap: "round",
                        stroke_linejoin: "round",
                        circle { cx: "12", cy: "12", r: "3" }
                        path { d: "M12 2v4" }
                        path { d: "M12 18v4" }
                        path { d: "M4.93 4.93l2.83 2.83" }
                        path { d: "M16.24 16.24l2.83 2.83" }
                        path { d: "M2 12h4" }
                        path { d: "M18 12h4" }
                        path { d: "M4.93 19.07l2.83-2.83" }
                        path { d: "M16.24 7.76l2.83-2.83" }
                    }
                }

                button {
                    class: "mcp-action-btn",
                    onclick: handle_edit,
                    title: "Edit server",
                    // Edit/pencil icon
                    svg {
                        width: "16",
                        height: "16",
                        view_box: "0 0 24 24",
                        fill: "none",
                        stroke: "currentColor",
                        stroke_width: "2",
                        stroke_linecap: "round",
                        stroke_linejoin: "round",
                        path { d: "M17 3a2.85 2.85 0 1 1 4 4L7.5 20.5 2 22l1.5-5.5L17 3z" }
                    }
                }

                button {
                    class: "mcp-action-btn danger",
                    onclick: handle_delete,
                    title: "Delete server",
                    // Trash icon
                    svg {
                        width: "16",
                        height: "16",
                        view_box: "0 0 24 24",
                        fill: "none",
                        stroke: "currentColor",
                        stroke_width: "2",
                        stroke_linecap: "round",
                        stroke_linejoin: "round",
                        path { d: "M3 6h18" }
                        path { d: "M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6" }
                        path { d: "M8 6V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2" }
                    }
                }
            }
        }
    }
}

/// Form for adding/editing MCP server configuration
#[component]
fn McpServerForm() -> Element {
    let mut ctx = use_app_context();
    let mcp_state = ctx.mcp_config.read();
    let is_adding = mcp_state.is_adding;

    // Get the config being edited
    let editing_config = mcp_state.editing.clone().unwrap_or_default();

    // Local form state
    let mut name = use_signal(|| editing_config.name.clone());
    let mut command = use_signal(|| editing_config.command.clone());
    let mut args = use_signal(|| editing_config.args.join(" "));
    let mut enabled = use_signal(|| editing_config.enabled);
    let mut env_entries = use_signal(|| {
        editing_config.env.iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect::<Vec<_>>()
    });

    let handle_submit = move |evt: Event<FormData>| {
        evt.stop_propagation();

        let name_val = name.read().clone();
        let command_val = command.read().clone();
        let args_val: Vec<String> = args.read()
            .split_whitespace()
            .map(|s| s.to_string())
            .collect();
        let enabled_val = *enabled.read();
        let env_val: Vec<(String, String)> = env_entries.read()
            .iter()
            .filter(|(k, _)| !k.is_empty())
            .cloned()
            .collect();

        spawn_local(async move {
            if is_adding {
                let _ = crate::messaging::send_mcp_add(
                    &name_val,
                    &command_val,
                    args_val,
                    enabled_val,
                    env_val,
                ).await;
            } else {
                let _ = crate::messaging::send_mcp_update(
                    &name_val,
                    &command_val,
                    args_val,
                    enabled_val,
                    env_val,
                ).await;
            }
        });
    };

    let handle_cancel = move |_| {
        ctx.mcp_config.write().cancel_edit();
    };

    let handle_add_env = move |_| {
        env_entries.write().push((String::new(), String::new()));
    };

    let entries_len = env_entries.read().len();

    rsx! {
        form {
            class: "mcp-server-form",
            onsubmit: handle_submit,

            // Name field
            div { class: "mcp-form-field",
                label { r#for: "mcp-name", "Server Name" }
                input {
                    id: "mcp-name",
                    r#type: "text",
                    required: true,
                    placeholder: "my-mcp-server",
                    value: "{name}",
                    disabled: !is_adding,
                    oninput: move |evt| name.set(evt.value().clone()),
                }
                if !is_adding {
                    span { class: "mcp-form-hint", "Name cannot be changed after creation" }
                }
            }

            // Command field
            div { class: "mcp-form-field",
                label { r#for: "mcp-command", "Command" }
                input {
                    id: "mcp-command",
                    r#type: "text",
                    required: true,
                    placeholder: "npx or /path/to/executable",
                    value: "{command}",
                    oninput: move |evt| command.set(evt.value().clone()),
                }
                span { class: "mcp-form-hint", "Command to run the MCP server" }
            }

            // Arguments field
            div { class: "mcp-form-field",
                label { r#for: "mcp-args", "Arguments" }
                input {
                    id: "mcp-args",
                    r#type: "text",
                    placeholder: "-y @modelcontextprotocol/server-filesystem",
                    value: "{args}",
                    oninput: move |evt| args.set(evt.value().clone()),
                }
                span { class: "mcp-form-hint", "Space-separated arguments" }
            }

            // Enabled checkbox
            div { class: "mcp-form-field mcp-form-checkbox",
                label { class: "mcp-checkbox-label",
                    input {
                        r#type: "checkbox",
                        checked: "{enabled}",
                        onchange: move |evt| enabled.set(evt.checked()),
                    }
                    span { "Enable this server" }
                }
            }

            // Environment variables
            div { class: "mcp-form-field",
                label { "Environment Variables" }
                div { class: "mcp-env-editor",
                    for i in 0..entries_len {
                        McpEnvEntry {
                            key: "{i}",
                            index: i,
                            env_entries: env_entries,
                        }
                    }
                    button {
                        r#type: "button",
                        class: "mcp-add-env-btn",
                        onclick: handle_add_env,
                        "+ Add Variable"
                    }
                }
            }

            // Form actions
            div { class: "mcp-form-actions",
                button {
                    r#type: "button",
                    class: "mcp-btn mcp-btn-secondary",
                    onclick: handle_cancel,
                    "Cancel"
                }
                button {
                    r#type: "submit",
                    class: "mcp-btn mcp-btn-primary",
                    if is_adding { "Add Server" } else { "Save Changes" }
                }
            }
        }
    }
}

/// Single environment variable entry in the form
#[component]
fn McpEnvEntry(index: usize, mut env_entries: Signal<Vec<(String, String)>>) -> Element {
    let entries = env_entries.read();
    let (key, value) = entries.get(index).cloned().unwrap_or_default();

    let handle_key_change = move |evt: Event<FormData>| {
        let new_key = evt.value().clone();
        let mut entries = env_entries.write();
        if let Some(entry) = entries.get_mut(index) {
            entry.0 = new_key;
        }
    };

    let handle_value_change = move |evt: Event<FormData>| {
        let new_value = evt.value().clone();
        let mut entries = env_entries.write();
        if let Some(entry) = entries.get_mut(index) {
            entry.1 = new_value;
        }
    };

    let handle_remove = move |_| {
        env_entries.write().remove(index);
    };

    rsx! {
        div { class: "mcp-env-entry",
            input {
                r#type: "text",
                placeholder: "KEY",
                value: "{key}",
                oninput: handle_key_change,
                class: "mcp-env-key",
            }
            span { class: "mcp-env-equals", "=" }
            input {
                r#type: "text",
                placeholder: "value",
                value: "{value}",
                oninput: handle_value_change,
                class: "mcp-env-value",
            }
            button {
                r#type: "button",
                class: "mcp-env-remove",
                onclick: handle_remove,
                aria_label: "Remove variable",
                // X icon
                svg {
                    width: "14",
                    height: "14",
                    view_box: "0 0 24 24",
                    fill: "none",
                    stroke: "currentColor",
                    stroke_width: "2",
                    stroke_linecap: "round",
                    stroke_linejoin: "round",
                    path { d: "M18 6L6 18" }
                    path { d: "M6 6l12 12" }
                }
            }
        }
    }
}
