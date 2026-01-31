/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

//! AskUser state management
//!
//! Handles the state for when the agent asks the user a question.

/// State for a pending AskUser request
#[derive(Debug, Clone, PartialEq)]
pub struct AskUserState {
    /// Request ID for responding
    pub request_id: String,
    /// The question to ask the user
    pub question: String,
    /// Available options (may be empty)
    pub options: Vec<String>,
    /// Whether custom input is allowed
    pub allow_custom: bool,
    /// Timeout in milliseconds
    pub timeout_ms: u64,
}

impl AskUserState {
    /// Create a new AskUser state
    pub fn new(
        request_id: String,
        question: String,
        options: Vec<String>,
        allow_custom: bool,
        timeout_ms: u64,
    ) -> Self {
        Self {
            request_id,
            question,
            options,
            allow_custom,
            timeout_ms,
        }
    }
}
