/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

//! Message area components

mod welcome_screen;
mod message_list;
mod message_bubble;
mod error_card;
mod code_block;
mod activity_feed;
mod live_tool_feed;

pub use welcome_screen::WelcomeScreen;
pub use message_list::MessageList;
pub use message_bubble::MessageBubble;
pub use message_bubble::render_simple_markdown;
pub use error_card::ErrorCard;
pub use code_block::CodeBlock;
pub use activity_feed::ActivityFeed;
pub use live_tool_feed::LiveToolFeed;

use dioxus::prelude::*;
use crate::context::use_app_context;

/// Message area container - shows welcome screen or message list
#[component]
pub fn MessageArea() -> Element {
    let ctx = use_app_context();
    let messages = ctx.messages.read();
    let streaming = ctx.streaming.read();
    let is_empty = messages.is_empty() && streaming.is_none();

    rsx! {
        div { class: "message-area",
            if is_empty {
                WelcomeScreen {}
            } else {
                MessageList {}
            }
        }
    }
}
