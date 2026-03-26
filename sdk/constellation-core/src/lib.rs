//! # constellation-core
//!
//! Core Rust SDK for Constellation A2A — agent-to-agent communication over Matrix.
//!
//! Provides [`ConstellationAgent`] for connecting to a Matrix homeserver, joining rooms,
//! sending messages with @-mentions, and dispatching structured task events between agents.
//!
//! # Quick start
//!
//! ```no_run
//! use constellation_core::{AgentConfigBuilder, ConstellationAgent, Message};
//!
//! # async fn example() -> constellation_core::Result<()> {
//! let config = AgentConfigBuilder::new()
//!     .homeserver_url("http://localhost:6167")
//!     .username("agent-researcher")
//!     .password("secret")
//!     .display_name("Research Agent")
//!     .auto_join_room("#agents:constellation.local")
//!     .build()?;
//!
//! let mut agent = ConstellationAgent::new(config)?;
//!
//! agent.on_mention(|event| {
//!     println!("Mentioned by {}: {}", event.sender, event.body);
//! }).await;
//!
//! agent.connect().await?;
//! agent.run().await?;
//! # Ok(())
//! # }
//! ```

pub mod agent;
pub mod config;
pub mod error;
pub mod message;
pub mod room;
pub mod task;
pub mod utils;

// Re-export primary types at crate root for convenience.
pub use agent::ConstellationAgent;
pub use config::{AgentConfig, AgentConfigBuilder};
pub use error::{ConstellationError, Result};
pub use message::{
    ConstellationMetadata, MentionEvent, Message, MessageEvent, Priority, Task, TaskEvent,
    TaskResult, TaskStatus,
};
pub use room::RoomHandle;
pub use task::TaskManager;
pub use utils::{format_room_alias, generate_device_id, parse_homeserver_url, sanitize_username};
