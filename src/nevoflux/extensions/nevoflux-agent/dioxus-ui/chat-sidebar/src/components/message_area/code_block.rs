/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

//! Code block component

use dioxus::prelude::*;
use super::copy_text_fallback;

/// Code block component with language label and copy button
#[component]
pub fn CodeBlock(language: String, code: String) -> Element {
    let mut copied = use_signal(|| false);
    let code_for_copy = code.clone();

    let handle_copy = move |_| {
        let code_clone = code_for_copy.clone();

        // Try synchronous execCommand fallback first (works in extension sidebar)
        if copy_text_fallback(&code_clone) {
            copied.set(true);
            spawn(async move {
                gloo::timers::future::TimeoutFuture::new(2000).await;
                copied.set(false);
            });
            return;
        }

        // Async Clipboard API fallback
        spawn(async move {
            if let Some(window) = web_sys::window() {
                let navigator = window.navigator();
                let clipboard = navigator.clipboard();
                if wasm_bindgen_futures::JsFuture::from(
                    clipboard.write_text(&code_clone)
                ).await.is_ok() {
                    copied.set(true);
                    gloo::timers::future::TimeoutFuture::new(2000).await;
                    copied.set(false);
                }
            }
        });
    };

    let copy_text = if copied() { "Copied!" } else { "Copy" };

    rsx! {
        div { class: "code-block",
            // Header with language and copy button
            div { class: "code-header",
                span { class: "code-language", "{language}" }
                button {
                    class: "code-copy-btn",
                    class: if copied() { "copied" },
                    onclick: handle_copy,
                    aria_label: "Copy code",
                    title: "Copy to clipboard",
                    "{copy_text}"
                }
            }

            // Code content
            div { class: "code-content",
                pre {
                    "{code}"
                }
            }
        }
    }
}
