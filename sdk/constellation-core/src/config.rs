//! Configuration types for Constellation agents.
//!
//! Use [`AgentConfigBuilder`] for a fluent API to construct agent configurations,
//! or create an [`AgentConfig`] directly.

use crate::error::{ConstellationError, Result};

/// Configuration for a Constellation agent's connection to a Matrix homeserver.
///
/// # Required fields
///
/// - `homeserver_url` — URL of the Matrix homeserver (e.g. `http://localhost:6167`)
/// - `username` — Matrix localpart for authentication
/// - `password` — Password for authentication
///
/// # Examples
///
/// ```
/// use constellation_core::AgentConfigBuilder;
///
/// let config = AgentConfigBuilder::new()
///     .homeserver_url("http://localhost:6167")
///     .username("agent-researcher")
///     .password("secret")
///     .display_name("Research Agent")
///     .auto_join_room("#agents:constellation.local")
///     .build()
///     .unwrap();
/// ```
#[derive(Debug, Clone)]
pub struct AgentConfig {
    /// URL of the Matrix homeserver to connect to.
    pub homeserver_url: String,
    /// Matrix username (localpart) for authentication.
    pub username: String,
    /// Password for Matrix authentication.
    pub password: String,
    /// Optional display name shown to other agents in rooms.
    pub display_name: Option<String>,
    /// Room aliases to automatically join on connect.
    pub auto_join_rooms: Vec<String>,
    /// Optional device ID for session persistence across restarts.
    pub device_id: Option<String>,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            homeserver_url: "http://localhost:6167".to_string(),
            username: String::new(),
            password: String::new(),
            display_name: None,
            auto_join_rooms: Vec::new(),
            device_id: None,
        }
    }
}

impl AgentConfig {
    /// Validate that all required fields are populated.
    ///
    /// # Errors
    ///
    /// Returns [`ConstellationError::Config`] if `username`, `password`, or
    /// `homeserver_url` is empty.
    pub fn validate(&self) -> Result<()> {
        if self.username.is_empty() {
            return Err(ConstellationError::Config(
                "username is required".to_string(),
            ));
        }
        if self.password.is_empty() {
            return Err(ConstellationError::Config(
                "password is required".to_string(),
            ));
        }
        if self.homeserver_url.is_empty() {
            return Err(ConstellationError::Config(
                "homeserver_url is required".to_string(),
            ));
        }
        Ok(())
    }
}

/// Builder for constructing an [`AgentConfig`] with a fluent API.
///
/// All fields have sensible defaults; only `username` and `password` are required
/// for [`build`](Self::build) to succeed.
pub struct AgentConfigBuilder {
    config: AgentConfig,
}

impl AgentConfigBuilder {
    /// Create a new builder with default values.
    pub fn new() -> Self {
        Self {
            config: AgentConfig::default(),
        }
    }

    /// Set the Matrix homeserver URL (default: `http://localhost:6167`).
    pub fn homeserver_url(mut self, url: impl Into<String>) -> Self {
        self.config.homeserver_url = url.into();
        self
    }

    /// Set the Matrix username (localpart) for authentication.
    pub fn username(mut self, username: impl Into<String>) -> Self {
        self.config.username = username.into();
        self
    }

    /// Set the password for Matrix authentication.
    pub fn password(mut self, password: impl Into<String>) -> Self {
        self.config.password = password.into();
        self
    }

    /// Set a display name for this agent.
    pub fn display_name(mut self, name: impl Into<String>) -> Self {
        self.config.display_name = Some(name.into());
        self
    }

    /// Add a single room alias to auto-join on connect.
    pub fn auto_join_room(mut self, room: impl Into<String>) -> Self {
        self.config.auto_join_rooms.push(room.into());
        self
    }

    /// Set the full list of room aliases to auto-join on connect.
    pub fn auto_join_rooms(mut self, rooms: Vec<String>) -> Self {
        self.config.auto_join_rooms = rooms;
        self
    }

    /// Set a device ID for session persistence across agent restarts.
    pub fn device_id(mut self, id: impl Into<String>) -> Self {
        self.config.device_id = Some(id.into());
        self
    }

    /// Validate and build the final [`AgentConfig`].
    ///
    /// # Errors
    ///
    /// Returns an error if required fields are missing.
    pub fn build(self) -> Result<AgentConfig> {
        self.config.validate()?;
        Ok(self.config)
    }
}

impl Default for AgentConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}
