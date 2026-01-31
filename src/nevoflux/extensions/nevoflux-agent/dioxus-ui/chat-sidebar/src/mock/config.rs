/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

//! Mock configuration

/// Mock mode configuration
#[derive(Debug, Clone)]
pub struct MockConfig {
    /// Whether mock mode is enabled
    pub enabled: bool,
    /// Delay before starting response (ms)
    pub response_delay_ms: u64,
    /// Delay per character for typing effect (ms)
    pub typing_speed_ms: u64,
    /// Whether to simulate errors
    pub simulate_errors: bool,
    /// Error probability (0.0 - 1.0)
    pub error_probability: f32,
    /// Whether to simulate permission requests
    pub simulate_permissions: bool,
    /// Whether to simulate tool execution
    pub simulate_tools: bool,
}

impl Default for MockConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            response_delay_ms: 500,
            typing_speed_ms: 20,
            simulate_errors: false,
            error_probability: 0.1,
            simulate_permissions: true,
            simulate_tools: true,
        }
    }
}

impl MockConfig {
    /// Parse configuration from URL parameters
    ///
    /// Supported parameters:
    /// - `mock=true` - Enable mock mode
    /// - `mock_errors=true` - Enable error simulation
    /// - `mock_permissions=false` - Disable permission simulation
    /// - `mock_tools=false` - Disable tool simulation
    pub fn from_url() -> Self {
        let search = web_sys::window()
            .and_then(|w| w.location().search().ok())
            .unwrap_or_default();

        Self {
            enabled: search.contains("mock=true"),
            simulate_errors: search.contains("mock_errors=true"),
            simulate_permissions: !search.contains("mock_permissions=false"),
            simulate_tools: !search.contains("mock_tools=false"),
            ..Default::default()
        }
    }
}
