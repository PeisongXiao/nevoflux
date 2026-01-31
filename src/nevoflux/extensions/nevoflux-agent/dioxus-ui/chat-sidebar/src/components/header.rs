/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

//! Header component

use dioxus::prelude::*;
use wasm_bindgen_futures::spawn_local;
use crate::context::use_app_context;
use crate::state::SessionSummary;

/// Header component with History, Maximize and More buttons
#[component]
pub fn Header() -> Element {
    let mut show_menu = use_signal(|| false);
    let mut show_history = use_signal(|| false);
    let mut ctx = use_app_context();

    // User info (Mock data for now, P2: fetch from account status)
    let username = "User";
    let user_initial = "U";

    let toggle_menu = move |_| {
        show_menu.set(!show_menu());
        show_history.set(false);
    };

    let toggle_history = move |_| {
        show_history.set(!show_history());
        show_menu.set(false);

        // Refresh history when opening
        if !show_history() {
            ctx.history.write().set_loading();
            spawn_local(async move {
                let _ = crate::messaging::send_session_list(50, 0).await;
            });
        }
    };

    let close_menu = move |_| {
        show_menu.set(false);
    };

    let close_history = move |_| {
        show_history.set(false);
    };

    let handle_maximize = move |_| {
        tracing::info!("Maximize requested");
        // P2: Implement maximize logic (e.g. open in new tab or expand sidebar)
    };

    let handle_config_mcp = move |_| {
        web_sys::console::log_1(&"[DEBUG] Configure MCP clicked".into());
        tracing::info!("Configure MCP requested");
        show_menu.set(false);
        // Open MCP config modal and request server list
        web_sys::console::log_1(&"[DEBUG] Setting show_mcp_config to true".into());
        ctx.show_mcp_config.set(true);
        ctx.mcp_config.write().set_loading();
        web_sys::console::log_1(&"[DEBUG] Spawning send_mcp_list".into());
        spawn_local(async move {
            let _ = crate::messaging::send_mcp_list().await;
        });
    };

    let handle_config_skills = move |_| {
        tracing::info!("Configure Skills requested");
        show_menu.set(false);
        // P2: Open Skills settings
    };

    let handle_new_chat = move |_| {
        tracing::info!("New chat requested");
        show_history.set(false);
        // Clear current messages to start fresh
        ctx.messages.set(Vec::new());
        ctx.streaming.set(None);
        ctx.agent_status.write().hide();
    };

    rsx! {
        header { class: "header",
            // Left side: History button
            div { class: "header-left",
                button {
                    class: "header-btn history-btn",
                    aria_label: "History",
                    title: "Conversation history",
                    onclick: toggle_history,
                    // Clock/history icon
                    svg {
                        width: "16",
                        height: "16",
                        view_box: "0 0 24 24",
                        fill: "none",
                        stroke: "currentColor",
                        stroke_width: "2",
                        stroke_linecap: "round",
                        stroke_linejoin: "round",
                        circle { cx: "12", cy: "12", r: "10" }
                        path { d: "M12 6v6l4 2" }
                    }
                }
            }

            // Right side: Action buttons
            div { class: "header-right",
                // Maximize button
                button {
                    class: "header-btn maximize-btn",
                    aria_label: "Maximize",
                    title: "Maximize",
                    onclick: handle_maximize,
                    // Box with arrow pointing out icon
                    svg {
                        width: "16",
                        height: "16",
                        view_box: "0 0 24 24",
                        fill: "none",
                        stroke: "currentColor",
                        stroke_width: "2",
                        stroke_linecap: "round",
                        stroke_linejoin: "round",
                        path { d: "M15 3h6v6" }
                        path { d: "M9 21H3v-6" }
                        path { d: "M21 3l-7 7" }
                        path { d: "M3 21l7-7" }
                    }
                }

                // More button
                button {
                    class: "header-btn more-btn",
                    aria_label: "More options",
                    title: "More options",
                    onclick: toggle_menu,
                    // Three dots icon
                    svg {
                        width: "16",
                        height: "16",
                        view_box: "0 0 24 24",
                        fill: "none",
                        stroke: "currentColor",
                        stroke_width: "2",
                        stroke_linecap: "round",
                        stroke_linejoin: "round",
                        circle { cx: "12", cy: "12", r: "1" }
                        circle { cx: "19", cy: "12", r: "1" }
                        circle { cx: "5", cy: "12", r: "1" }
                    }
                }
            }

            // History Dropdown
            if show_history() {
                div {
                    class: "menu-overlay",
                    onclick: close_history,
                    onkeydown: move |evt: KeyboardEvent| {
                        if evt.key() == Key::Escape {
                            show_history.set(false);
                        }
                    },
                    tabindex: "-1",
                }

                HistoryDropdown {
                    on_close: move |_| show_history.set(false),
                    on_new_chat: handle_new_chat,
                }
            }

            // Dropdown Menu Overlay (to close on click outside or Escape)
            if show_menu() {
                div {
                    class: "menu-overlay",
                    onclick: close_menu,
                    onkeydown: move |evt: KeyboardEvent| {
                        if evt.key() == Key::Escape {
                            show_menu.set(false);
                        }
                    },
                    tabindex: "-1",
                }

                // Dropdown Menu
                div {
                    class: "dropdown-menu",
                    role: "menu",
                    aria_label: "Settings menu",
                    onkeydown: move |evt: KeyboardEvent| {
                        if evt.key() == Key::Escape {
                            show_menu.set(false);
                        }
                    },

                    // User Info
                    div { class: "menu-item user-profile",
                        div { class: "user-avatar", aria_hidden: "true", "{user_initial}" }
                        span { "{username}" }
                    }

                    div { class: "menu-separator", role: "separator" }

                    // Configure MCP
                    button {
                        class: "menu-item",
                        role: "menuitem",
                        onclick: handle_config_mcp,
                        "Configure MCP"
                    }

                    // Configure Skills
                    button {
                        class: "menu-item",
                        role: "menuitem",
                        onclick: handle_config_skills,
                        "Configure Skills"
                    }
                }
            }
        }
    }
}

/// History dropdown component
#[component]
fn HistoryDropdown(
    on_close: EventHandler<()>,
    on_new_chat: EventHandler<MouseEvent>,
) -> Element {
    let ctx = use_app_context();
    let history = ctx.history.read();

    rsx! {
        div {
            class: "dropdown-menu history-dropdown",
            role: "menu",
            aria_label: "Conversation history",
            onkeydown: move |evt: KeyboardEvent| {
                if evt.key() == Key::Escape {
                    on_close.call(());
                }
            },

            // Header with New Chat button
            div { class: "history-dropdown-header",
                span { class: "history-dropdown-title", "History" }
                button {
                    class: "new-chat-btn",
                    onclick: on_new_chat,
                    // Plus icon
                    svg {
                        width: "14",
                        height: "14",
                        view_box: "0 0 24 24",
                        fill: "none",
                        stroke: "currentColor",
                        stroke_width: "2",
                        stroke_linecap: "round",
                        stroke_linejoin: "round",
                        path { d: "M12 5v14" }
                        path { d: "M5 12h14" }
                    }
                    span { "New" }
                }
            }

            div { class: "menu-separator", role: "separator" }

            // Content
            if history.loading {
                div { class: "history-dropdown-loading",
                    span { class: "loading-spinner" }
                    span { "Loading..." }
                }
            } else if let Some(ref error) = history.error {
                div { class: "history-dropdown-error",
                    "Error: {error}"
                }
            } else if history.sessions.is_empty() {
                div { class: "history-dropdown-empty",
                    "No conversations yet"
                }
            } else {
                div { class: "history-dropdown-list",
                    for session in history.sessions.iter().take(10) {
                        HistoryDropdownItem {
                            session: session.clone(),
                            on_select: move |_| on_close.call(()),
                        }
                    }
                }
            }
        }
    }
}

/// Single history dropdown item
#[component]
fn HistoryDropdownItem(
    session: SessionSummary,
    on_select: EventHandler<()>,
) -> Element {
    let ctx = use_app_context();
    let session_id = session.id.clone();
    let display_title = session.display_title();
    let relative_time = session.relative_time();

    let handle_click = move |_| {
        let source_id = session_id.clone();
        let tab_context = ctx.tab_context.read();
        let target_id = tab_context.zen_sync_id.clone();
        drop(tab_context);

        if let Some(target_id) = target_id {
            tracing::info!("Cloning session {} to {}", source_id, target_id);
            spawn_local(async move {
                if let Err(e) = crate::messaging::send_session_clone(&source_id, &target_id).await {
                    tracing::error!("Failed to clone session: {}", e);
                }
            });
            on_select.call(());
        } else {
            tracing::warn!("No zen_sync_id for current tab, cannot restore session");
        }
    };

    rsx! {
        button {
            class: "menu-item history-dropdown-item",
            role: "menuitem",
            onclick: handle_click,

            div { class: "history-dropdown-item-content",
                span { class: "history-dropdown-item-title", "{display_title}" }
                span { class: "history-dropdown-item-time", "{relative_time}" }
            }
        }
    }
}
