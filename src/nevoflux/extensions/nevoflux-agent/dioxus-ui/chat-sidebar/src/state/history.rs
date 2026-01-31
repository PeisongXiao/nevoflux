/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

//! History session state management

/// Summary of a historical session
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SessionSummary {
    /// Session ID
    pub id: String,
    /// Session title (may be auto-generated from first message)
    pub title: Option<String>,
    /// Last update timestamp
    pub updated_at: u64,
    /// Number of messages in the session
    pub message_count: u32,
}

impl SessionSummary {
    /// Get display title (falls back to truncated ID if no title)
    pub fn display_title(&self) -> String {
        self.title.clone().unwrap_or_else(|| {
            if self.id.len() > 20 {
                format!("{}...", &self.id[..20])
            } else {
                self.id.clone()
            }
        })
    }

    /// Get relative time string (e.g., "2 hours ago")
    pub fn relative_time(&self) -> String {
        let now = js_sys::Date::now() as u64;
        let diff_ms = now.saturating_sub(self.updated_at * 1000); // updated_at is in seconds
        let diff_secs = diff_ms / 1000;

        if diff_secs < 60 {
            "Just now".to_string()
        } else if diff_secs < 3600 {
            let mins = diff_secs / 60;
            format!("{} min{} ago", mins, if mins == 1 { "" } else { "s" })
        } else if diff_secs < 86400 {
            let hours = diff_secs / 3600;
            format!("{} hour{} ago", hours, if hours == 1 { "" } else { "s" })
        } else {
            let days = diff_secs / 86400;
            format!("{} day{} ago", days, if days == 1 { "" } else { "s" })
        }
    }
}

/// History list state
#[derive(Debug, Clone, Default)]
pub struct HistoryState {
    /// List of session summaries
    pub sessions: Vec<SessionSummary>,
    /// Total number of sessions available
    pub total: u32,
    /// Whether we're currently loading
    pub loading: bool,
    /// Error message if loading failed
    pub error: Option<String>,
}

impl HistoryState {
    /// Create new empty history state
    pub fn new() -> Self {
        Self::default()
    }

    /// Set loading state
    pub fn set_loading(&mut self) {
        self.loading = true;
        self.error = None;
    }

    /// Set loaded sessions
    pub fn set_sessions(&mut self, sessions: Vec<SessionSummary>, total: u32) {
        self.sessions = sessions;
        self.total = total;
        self.loading = false;
        self.error = None;
    }

    /// Set error state
    pub fn set_error(&mut self, error: String) {
        self.loading = false;
        self.error = Some(error);
    }

    /// Check if there are any sessions
    pub fn has_sessions(&self) -> bool {
        !self.sessions.is_empty()
    }
}
