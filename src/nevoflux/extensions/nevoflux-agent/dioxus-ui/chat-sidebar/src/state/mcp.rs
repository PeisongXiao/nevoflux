/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

//! MCP (Model Context Protocol) server configuration state

/// Configuration for an MCP server
#[derive(Debug, Clone, PartialEq, Default)]
pub struct McpServerConfig {
    /// Server name (unique identifier)
    pub name: String,
    /// Command to run the server
    pub command: String,
    /// Arguments to pass to the command
    pub args: Vec<String>,
    /// Whether the server is enabled
    pub enabled: bool,
    /// Environment variables (key, value pairs)
    pub env: Vec<(String, String)>,
}

impl McpServerConfig {
    /// Create a new MCP server config with just a name
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            command: String::new(),
            args: Vec::new(),
            enabled: true,
            env: Vec::new(),
        }
    }

    /// Set the command
    pub fn with_command(mut self, command: impl Into<String>) -> Self {
        self.command = command.into();
        self
    }

    /// Set the arguments
    pub fn with_args(mut self, args: Vec<String>) -> Self {
        self.args = args;
        self
    }

    /// Set enabled state
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Set environment variables
    pub fn with_env(mut self, env: Vec<(String, String)>) -> Self {
        self.env = env;
        self
    }
}

/// Connection status for an MCP server
#[derive(Debug, Clone, PartialEq, Default)]
pub enum McpConnectionStatus {
    /// Server is not connected
    #[default]
    Disconnected,
    /// Server is currently connecting
    Connecting,
    /// Server is connected
    Connected,
    /// Server connection failed with error
    Error(String),
}

impl McpConnectionStatus {
    /// Check if the server is connected
    pub fn is_connected(&self) -> bool {
        matches!(self, Self::Connected)
    }

    /// Check if the server has an error
    pub fn is_error(&self) -> bool {
        matches!(self, Self::Error(_))
    }

    /// Get the error message if any
    pub fn error_message(&self) -> Option<&str> {
        match self {
            Self::Error(msg) => Some(msg),
            _ => None,
        }
    }
}

/// MCP server with configuration and status
#[derive(Debug, Clone, PartialEq)]
pub struct McpServer {
    /// Server configuration
    pub config: McpServerConfig,
    /// Current connection status
    pub status: McpConnectionStatus,
}

impl McpServer {
    /// Create a new MCP server from config
    pub fn new(config: McpServerConfig) -> Self {
        Self {
            config,
            status: McpConnectionStatus::Disconnected,
        }
    }

    /// Create from config with a specific status
    pub fn with_status(config: McpServerConfig, status: McpConnectionStatus) -> Self {
        Self { config, status }
    }
}

/// MCP configuration state for the sidebar
#[derive(Debug, Clone, Default)]
pub struct McpConfigState {
    /// List of configured MCP servers
    pub servers: Vec<McpServer>,
    /// Whether the config is currently loading
    pub loading: bool,
    /// Error message if loading/operation failed
    pub error: Option<String>,
    /// Server config being edited (None = list view, Some = form view)
    pub editing: Option<McpServerConfig>,
    /// Whether we're adding a new server (true) or editing existing (false)
    pub is_adding: bool,
    /// Test result: (server name, success, message)
    pub test_result: Option<(String, bool, String)>,
}

impl McpConfigState {
    /// Create a new empty MCP config state
    pub fn new() -> Self {
        Self::default()
    }

    /// Set loading state
    pub fn set_loading(&mut self) {
        self.loading = true;
        self.error = None;
    }

    /// Set servers from response
    pub fn set_servers(&mut self, servers: Vec<McpServer>) {
        self.servers = servers;
        self.loading = false;
        self.error = None;
    }

    /// Set error state
    pub fn set_error(&mut self, error: String) {
        self.loading = false;
        self.error = Some(error);
    }

    /// Start adding a new server
    pub fn start_add(&mut self) {
        self.editing = Some(McpServerConfig::default());
        self.is_adding = true;
        self.test_result = None;
    }

    /// Start editing an existing server
    pub fn start_edit(&mut self, server_name: &str) {
        if let Some(server) = self.servers.iter().find(|s| s.config.name == server_name) {
            self.editing = Some(server.config.clone());
            self.is_adding = false;
            self.test_result = None;
        }
    }

    /// Cancel editing/adding
    pub fn cancel_edit(&mut self) {
        self.editing = None;
        self.is_adding = false;
        self.test_result = None;
    }

    /// Set test result
    pub fn set_test_result(&mut self, name: String, success: bool, message: String) {
        self.test_result = Some((name, success, message));
    }

    /// Clear test result
    pub fn clear_test_result(&mut self) {
        self.test_result = None;
    }

    /// Find a server by name
    pub fn find_server(&self, name: &str) -> Option<&McpServer> {
        self.servers.iter().find(|s| s.config.name == name)
    }

    /// Update server status by name
    pub fn update_status(&mut self, name: &str, status: McpConnectionStatus) {
        if let Some(server) = self.servers.iter_mut().find(|s| s.config.name == name) {
            server.status = status;
        }
    }

    /// Check if currently in form view
    pub fn is_editing(&self) -> bool {
        self.editing.is_some()
    }
}
