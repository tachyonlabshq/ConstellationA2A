//! Message types and mention helpers for the Constellation protocol.
//!
//! This module defines the core data structures for sending and receiving messages,
//! structured tasks, and constellation metadata. It also provides utilities for
//! parsing @-mentions from message bodies and formatting HTML mention links.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::utils::html_escape;

/// Priority level for tasks, from lowest to highest urgency.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Priority {
    Low,
    Normal,
    High,
    Critical,
}

impl Default for Priority {
    fn default() -> Self {
        Self::Normal
    }
}

/// Status of a task through its lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TaskStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
}

/// The `ai.constellation.metadata` block embedded in Matrix message event content.
///
/// This structured metadata enables machine-readable task routing alongside
/// the human-readable message body and HTML @-mentions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstellationMetadata {
    /// Unique identifier for this task.
    pub task_id: String,
    /// The type/category of the task (e.g. "analysis", "code-review").
    pub task_type: String,
    /// Arbitrary JSON payload with task-specific data.
    #[serde(default)]
    pub payload: serde_json::Value,
    /// Priority level for task scheduling.
    #[serde(default)]
    pub priority: Priority,
    /// If this is a reply to another task, that task's ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reply_to_task: Option<String>,
    /// Thread ID for grouping related tasks together.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thread_id: Option<String>,
}

/// An outgoing message to be sent to a room.
///
/// # Examples
///
/// ```
/// use constellation_core::Message;
///
/// // Simple text message
/// let msg = Message::text("Hello, agents!");
///
/// // Message with task metadata
/// use constellation_core::{Task, Priority};
/// let task = Task::new("analysis", serde_json::json!({"file": "data.csv"}))
///     .with_priority(Priority::High);
/// let msg = Message::text("Please analyze this").with_metadata(task.to_metadata());
/// ```
#[derive(Debug, Clone)]
pub struct Message {
    /// The plain-text body of the message.
    pub body: String,
    /// Optional constellation metadata to embed in the event content.
    pub metadata: Option<ConstellationMetadata>,
}

impl Message {
    /// Create a plain text message with no metadata.
    pub fn text(body: impl Into<String>) -> Self {
        Self {
            body: body.into(),
            metadata: None,
        }
    }

    /// Attach [`ConstellationMetadata`] to this message (builder pattern).
    pub fn with_metadata(mut self, metadata: ConstellationMetadata) -> Self {
        self.metadata = Some(metadata);
        self
    }
}

/// Event received when this agent is @-mentioned in a message.
///
/// Delivered to handlers registered via [`ConstellationAgent::on_mention`](crate::ConstellationAgent::on_mention).
#[derive(Debug, Clone)]
pub struct MentionEvent {
    /// The Matrix user ID of the sender (e.g. `@agent-a:constellation.local`).
    pub sender: String,
    /// The room ID where the mention occurred.
    pub room_id: String,
    /// The plain-text body of the message.
    pub body: String,
    /// Parsed constellation metadata, if the message included it.
    pub metadata: Option<ConstellationMetadata>,
    /// All agent user IDs mentioned in this message.
    pub mentioned_agents: Vec<String>,
}

/// Event received for any message in a joined room.
///
/// Delivered to handlers registered via [`ConstellationAgent::on_message`](crate::ConstellationAgent::on_message).
#[derive(Debug, Clone)]
pub struct MessageEvent {
    /// The Matrix user ID of the sender.
    pub sender: String,
    /// The room ID where the message was sent.
    pub room_id: String,
    /// The plain-text body of the message.
    pub body: String,
    /// The full event as a JSON value for access to all fields.
    pub raw_event: serde_json::Value,
}

/// Event received when a structured task message arrives.
///
/// Delivered to handlers registered via [`ConstellationAgent::on_task`](crate::ConstellationAgent::on_task).
/// Only fires for messages containing valid `ai.constellation.metadata`.
#[derive(Debug, Clone)]
pub struct TaskEvent {
    /// The Matrix user ID of the sender.
    pub sender: String,
    /// The room ID where the task was sent.
    pub room_id: String,
    /// The unique task identifier from the metadata.
    pub task_id: String,
    /// The task type/category from the metadata.
    pub task_type: String,
    /// The task payload from the metadata.
    pub payload: serde_json::Value,
    /// The task priority from the metadata.
    pub priority: Priority,
}

/// A task to be created and sent to a room.
///
/// Use the builder methods to configure the task before passing it to
/// [`ConstellationAgent::create_task`](crate::ConstellationAgent::create_task).
///
/// # Examples
///
/// ```
/// use constellation_core::{Task, Priority};
///
/// let task = Task::new("analysis", serde_json::json!({"file": "data.csv"}))
///     .with_priority(Priority::High)
///     .with_thread("thread-123");
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    /// Unique task identifier (auto-generated UUID if not specified).
    #[serde(default = "generate_task_id")]
    pub id: String,
    /// The type/category of the task.
    pub task_type: String,
    /// Arbitrary JSON payload with task-specific data.
    #[serde(default)]
    pub payload: serde_json::Value,
    /// Priority level for task scheduling.
    #[serde(default)]
    pub priority: Priority,
    /// If this task is a reply to another, that task's ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reply_to_task: Option<String>,
    /// Thread ID for grouping related tasks.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thread_id: Option<String>,
}

fn generate_task_id() -> String {
    Uuid::new_v4().to_string()
}

impl Task {
    /// Create a new task with a type and payload. A UUID is generated automatically.
    pub fn new(task_type: impl Into<String>, payload: serde_json::Value) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            task_type: task_type.into(),
            payload,
            priority: Priority::Normal,
            reply_to_task: None,
            thread_id: None,
        }
    }

    /// Set the priority level (builder pattern).
    pub fn with_priority(mut self, priority: Priority) -> Self {
        self.priority = priority;
        self
    }

    /// Set the thread ID for grouping related tasks (builder pattern).
    pub fn with_thread(mut self, thread_id: impl Into<String>) -> Self {
        self.thread_id = Some(thread_id.into());
        self
    }

    /// Mark this task as a reply to another task (builder pattern).
    pub fn replying_to(mut self, task_id: impl Into<String>) -> Self {
        self.reply_to_task = Some(task_id.into());
        self
    }

    /// Convert this task into [`ConstellationMetadata`] for embedding in a message.
    pub fn to_metadata(&self) -> ConstellationMetadata {
        ConstellationMetadata {
            task_id: self.id.clone(),
            task_type: self.task_type.clone(),
            payload: self.payload.clone(),
            priority: self.priority,
            reply_to_task: self.reply_to_task.clone(),
            thread_id: self.thread_id.clone(),
        }
    }
}

/// The result of completing a task, sent back to the originating room.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    /// The ID of the task being completed.
    pub task_id: String,
    /// Whether the task completed successfully or failed.
    pub status: TaskStatus,
    /// Arbitrary JSON result data.
    #[serde(default)]
    pub result_data: serde_json::Value,
}

impl TaskResult {
    /// Create a successful task result.
    pub fn success(task_id: impl Into<String>, data: serde_json::Value) -> Self {
        Self {
            task_id: task_id.into(),
            status: TaskStatus::Completed,
            result_data: data,
        }
    }

    /// Create a failed task result.
    pub fn failure(task_id: impl Into<String>, data: serde_json::Value) -> Self {
        Self {
            task_id: task_id.into(),
            status: TaskStatus::Failed,
            result_data: data,
        }
    }
}

// ---------------------------------------------------------------------------
// Mention helpers
// ---------------------------------------------------------------------------

/// Extract Matrix user IDs mentioned in a message body via `@localpart:server` patterns.
///
/// Parses the plain-text body for Matrix-style mentions. The localpart may contain
/// alphanumeric characters, hyphens, underscores, and dots. The server part may
/// contain alphanumeric characters, hyphens, dots, and an optional port.
///
/// # Examples
///
/// ```
/// use constellation_core::message::parse_mentions;
///
/// let mentions = parse_mentions("Hey @agent-a:server.local, ask @agent-b:server.local");
/// assert_eq!(mentions, vec![
///     "@agent-a:server.local".to_string(),
///     "@agent-b:server.local".to_string(),
/// ]);
/// ```
pub fn parse_mentions(body: &str) -> Vec<String> {
    let mut mentions = Vec::new();
    let mut i = 0;
    let chars: Vec<char> = body.chars().collect();
    while i < chars.len() {
        if chars[i] == '@' {
            let start = i;
            i += 1;
            // Consume localpart (alphanumeric, -, _, .)
            while i < chars.len()
                && (chars[i].is_alphanumeric()
                    || chars[i] == '-'
                    || chars[i] == '_'
                    || chars[i] == '.')
            {
                i += 1;
            }
            // Expect ':'
            if i < chars.len() && chars[i] == ':' {
                i += 1;
                // Consume server name (alphanumeric, ., -)
                while i < chars.len()
                    && (chars[i].is_alphanumeric() || chars[i] == '.' || chars[i] == '-')
                {
                    i += 1;
                }
                // Optionally consume port (:digits)
                if i < chars.len() && chars[i] == ':' {
                    let port_start = i;
                    i += 1;
                    while i < chars.len() && chars[i].is_ascii_digit() {
                        i += 1;
                    }
                    // If no digits followed the colon, backtrack.
                    if i == port_start + 1 {
                        i = port_start;
                    }
                }
                let mention: String = chars[start..i].iter().collect();
                // Must have at least @x:y
                if mention.len() > 3 {
                    mentions.push(mention);
                }
            }
        } else {
            i += 1;
        }
    }
    mentions
}

/// Build an HTML-formatted mention link for a Matrix user.
///
/// Returns a tuple of `(plain_text, html)` suitable for the Matrix message
/// `body` and `formatted_body` fields respectively.
pub fn format_mention(user_id: &str, display_name: &str) -> (String, String) {
    let plain = display_name.to_string();
    let escaped_name = html_escape(display_name);
    let html = format!(
        "<a href=\"https://matrix.to/#/{user_id}\">{escaped_name}</a>"
    );
    (plain, html)
}

/// Build a full message body with a leading @-mention.
///
/// Returns a tuple of `(plain_text, html)` where the mention is an HTML link
/// and the message body is HTML-escaped to prevent injection.
pub fn format_mention_message(
    user_id: &str,
    display_name: &str,
    message: &str,
) -> (String, String) {
    let plain = format!("{display_name} {message}");
    let escaped_name = html_escape(display_name);
    let escaped_msg = html_escape(message);
    let html = format!(
        "<a href=\"https://matrix.to/#/{user_id}\">{escaped_name}</a> {escaped_msg}"
    );
    (plain, html)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_mentions() {
        let body = "@agent-a:constellation.local can you help @agent-b:constellation.local?";
        let mentions = parse_mentions(body);
        assert_eq!(mentions.len(), 2);
        assert_eq!(mentions[0], "@agent-a:constellation.local");
        assert_eq!(mentions[1], "@agent-b:constellation.local");
    }

    #[test]
    fn test_parse_mentions_none() {
        let mentions = parse_mentions("hello world, no mentions here");
        assert!(mentions.is_empty());
    }

    #[test]
    fn test_parse_mentions_with_port() {
        let mentions = parse_mentions("@agent:localhost:8448 do something");
        assert_eq!(mentions.len(), 1);
        assert_eq!(mentions[0], "@agent:localhost:8448");
    }

    #[test]
    fn test_parse_mentions_port_no_digits_ignored() {
        // A trailing colon with no digits should not be included.
        let mentions = parse_mentions("@agent:server: next word");
        assert_eq!(mentions.len(), 1);
        assert_eq!(mentions[0], "@agent:server");
    }

    #[test]
    fn test_format_mention_message() {
        let (plain, html) = format_mention_message(
            "@agent-b:constellation.local",
            "@agent-b",
            "analyze this data",
        );
        assert_eq!(plain, "@agent-b analyze this data");
        assert!(html.contains("matrix.to"));
        assert!(html.contains("@agent-b:constellation.local"));
    }

    #[test]
    fn test_format_mention_message_escapes_html() {
        let (_, html) = format_mention_message(
            "@agent:server",
            "@agent",
            "<script>alert('xss')</script>",
        );
        assert!(!html.contains("<script>"));
        assert!(html.contains("&lt;script&gt;"));
    }
}
