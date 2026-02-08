/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

//! Context bar showing current tab information

use dioxus::prelude::*;
use crate::context::use_app_context;
use crate::state::TabContext;
use crate::utils::{truncate, extract_domain};

/// Context bar component showing current tab info
#[component]
pub fn ContextBar() -> Element {
    let mut ctx = use_app_context();
    // Clone upfront and drop the read guard immediately to avoid holding
    // a borrow across the entire render (which can cause AlreadyBorrowed
    // panics when the signal is written from a JS callback).
    let tab = ctx.tab_context.read().clone();

    // Don't show if no context
    if tab.url.is_empty() {
        return rsx! {};
    }

    let favicon = tab.favicon_url.clone();
    let title = truncate(&tab.title, 40);
    let domain = extract_domain(&tab.url);

    let handle_remove = move |_| {
        ctx.tab_context.set(TabContext::default());
    };

    rsx! {
        div { class: "context-bar",
            // Favicon
            if let Some(ref favicon) = favicon {
                img {
                    class: "context-favicon",
                    src: "{favicon}",
                    alt: "",
                    width: "16",
                    height: "16",
                }
            }

            // Title (truncated)
            span { class: "context-title", "{title}" }

            // Domain
            span { class: "context-domain", "{domain}" }

            // Remove button
            button {
                class: "context-remove",
                onclick: handle_remove,
                aria_label: "Remove context",
                title: "Remove tab context",
                "×"
            }
        }
    }
}
