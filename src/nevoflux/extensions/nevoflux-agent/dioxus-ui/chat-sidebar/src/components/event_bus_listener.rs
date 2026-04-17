/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use dioxus::prelude::*;
use crate::context::AppContext;

/// Auto-dismiss delay in milliseconds.
const TOAST_TTL_MS: u32 = 5000;

#[component]
pub fn EventBusListener() -> Element {
    let mut ctx = use_context::<AppContext>();
    let notifications = ctx.event_notifications.read();
    let visible: Vec<_> = notifications.iter().rev().take(3).collect();
    let count = notifications.len();

    // Schedule auto-dismiss whenever a new notification arrives.
    use_effect(move || {
        if count == 0 {
            return;
        }
        spawn(async move {
            crate::messaging::sleep_ms(TOAST_TTL_MS).await;
            let mut notifs = ctx.event_notifications.write();
            // Remove the oldest notification (the one that just expired).
            if !notifs.is_empty() {
                notifs.remove(0);
            }
        });
    });

    rsx! {
        if !visible.is_empty() {
            div { class: "nevo-event-toasts",
                for notif in visible {
                    div {
                        class: "nevo-event-toast",
                        key: "{notif.id}",
                        div { class: "nevo-event-toast-title", "{notif.title}" }
                        if !notif.body.is_empty() {
                            div { class: "nevo-event-toast-body", "{notif.body}" }
                        }
                    }
                }
            }
        }
    }
}
