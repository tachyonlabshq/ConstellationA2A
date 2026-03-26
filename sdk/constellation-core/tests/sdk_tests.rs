//! Integration-style tests for the Constellation SDK public API.
//!
//! These tests exercise the crate's public surface without a running Matrix server.

use constellation_core::{
    AgentConfig, AgentConfigBuilder, ConstellationMetadata, Message, Priority, Task, TaskResult,
    TaskStatus,
};
use constellation_core::message::{format_mention_message, parse_mentions};
use constellation_core::task::TaskManager;
use constellation_core::utils::{
    format_room_alias, generate_device_id, parse_homeserver_url, sanitize_username,
};

// ---------------------------------------------------------------------------
// AgentConfig + Builder
// ---------------------------------------------------------------------------

#[test]
fn config_builder_valid() {
    let config = AgentConfigBuilder::new()
        .homeserver_url("http://localhost:6167")
        .username("agent-researcher")
        .password("secret")
        .display_name("Research Agent")
        .auto_join_room("#agents:constellation.local")
        .build()
        .expect("valid config should build");

    assert_eq!(config.homeserver_url, "http://localhost:6167");
    assert_eq!(config.username, "agent-researcher");
    assert_eq!(config.password, "secret");
    assert_eq!(config.display_name.as_deref(), Some("Research Agent"));
    assert_eq!(config.auto_join_rooms, vec!["#agents:constellation.local"]);
    assert!(config.device_id.is_none());
}

#[test]
fn config_builder_missing_username() {
    let result = AgentConfigBuilder::new()
        .homeserver_url("http://localhost:6167")
        .password("secret")
        .build();
    assert!(result.is_err());
}

#[test]
fn config_builder_missing_password() {
    let result = AgentConfigBuilder::new()
        .homeserver_url("http://localhost:6167")
        .username("agent")
        .build();
    assert!(result.is_err());
}

#[test]
fn config_validate_empty_homeserver() {
    let config = AgentConfig {
        homeserver_url: String::new(),
        username: "agent".into(),
        password: "pass".into(),
        display_name: None,
        auto_join_rooms: vec![],
        device_id: None,
    };
    assert!(config.validate().is_err());
}

#[test]
fn config_builder_with_device_id() {
    let config = AgentConfigBuilder::new()
        .homeserver_url("http://localhost:6167")
        .username("bot")
        .password("pass")
        .device_id("MY_DEVICE")
        .build()
        .unwrap();
    assert_eq!(config.device_id.as_deref(), Some("MY_DEVICE"));
}

#[test]
fn config_builder_multiple_rooms() {
    let config = AgentConfigBuilder::new()
        .homeserver_url("http://localhost:6167")
        .username("bot")
        .password("pass")
        .auto_join_room("#room1:server")
        .auto_join_room("#room2:server")
        .build()
        .unwrap();
    assert_eq!(config.auto_join_rooms.len(), 2);
}

// ---------------------------------------------------------------------------
// Message
// ---------------------------------------------------------------------------

#[test]
fn message_text_no_metadata() {
    let msg = Message::text("Hello, world!");
    assert_eq!(msg.body, "Hello, world!");
    assert!(msg.metadata.is_none());
}

#[test]
fn message_with_metadata() {
    let task = Task::new("analysis", serde_json::json!({"file": "data.csv"}));
    let msg = Message::text("Analyze this").with_metadata(task.to_metadata());
    assert!(msg.metadata.is_some());
    let meta = msg.metadata.unwrap();
    assert_eq!(meta.task_type, "analysis");
    assert_eq!(meta.payload["file"], "data.csv");
}

// ---------------------------------------------------------------------------
// ConstellationMetadata serde round-trip
// ---------------------------------------------------------------------------

#[test]
fn metadata_serde_roundtrip() {
    let meta = ConstellationMetadata {
        task_id: "task-123".into(),
        task_type: "code-review".into(),
        payload: serde_json::json!({"pr": 42, "repo": "test"}),
        priority: Priority::High,
        reply_to_task: Some("task-000".into()),
        thread_id: Some("thread-abc".into()),
    };

    let json = serde_json::to_string(&meta).unwrap();
    let deserialized: ConstellationMetadata = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.task_id, "task-123");
    assert_eq!(deserialized.task_type, "code-review");
    assert_eq!(deserialized.priority, Priority::High);
    assert_eq!(deserialized.reply_to_task.as_deref(), Some("task-000"));
    assert_eq!(deserialized.thread_id.as_deref(), Some("thread-abc"));
    assert_eq!(deserialized.payload["pr"], 42);
}

#[test]
fn metadata_serde_defaults() {
    let json = r#"{"task_id":"t1","task_type":"test"}"#;
    let meta: ConstellationMetadata = serde_json::from_str(json).unwrap();
    assert_eq!(meta.priority, Priority::Normal); // default
    assert!(meta.reply_to_task.is_none());
    assert!(meta.thread_id.is_none());
    assert_eq!(meta.payload, serde_json::Value::Null); // default
}

// ---------------------------------------------------------------------------
// Task + TaskResult
// ---------------------------------------------------------------------------

#[test]
fn task_builder_pattern() {
    let task = Task::new("analysis", serde_json::json!({"key": "value"}))
        .with_priority(Priority::Critical)
        .with_thread("thread-1")
        .replying_to("parent-task");

    assert_eq!(task.task_type, "analysis");
    assert_eq!(task.priority, Priority::Critical);
    assert_eq!(task.thread_id.as_deref(), Some("thread-1"));
    assert_eq!(task.reply_to_task.as_deref(), Some("parent-task"));
    assert!(!task.id.is_empty()); // UUID auto-generated
}

#[test]
fn task_to_metadata_conversion() {
    let task = Task::new("review", serde_json::json!(null))
        .with_priority(Priority::Low);
    let meta = task.to_metadata();

    assert_eq!(meta.task_id, task.id);
    assert_eq!(meta.task_type, "review");
    assert_eq!(meta.priority, Priority::Low);
}

#[test]
fn task_result_success() {
    let result = TaskResult::success("t1", serde_json::json!({"answer": 42}));
    assert_eq!(result.task_id, "t1");
    assert_eq!(result.status, TaskStatus::Completed);
    assert_eq!(result.result_data["answer"], 42);
}

#[test]
fn task_result_failure() {
    let result = TaskResult::failure("t2", serde_json::json!({"error": "timeout"}));
    assert_eq!(result.status, TaskStatus::Failed);
}

// ---------------------------------------------------------------------------
// TaskManager
// ---------------------------------------------------------------------------

#[test]
fn task_manager_lifecycle() {
    let mut mgr = TaskManager::new();
    assert!(mgr.is_empty());

    let id = mgr.create("task-1", "analysis", serde_json::json!({}), "!room:test");
    assert_eq!(mgr.len(), 1);
    assert_eq!(mgr.get_status(&id), Some(TaskStatus::Pending));
    assert_eq!(mgr.list_pending().len(), 1);

    mgr.update_status(&id, TaskStatus::InProgress).unwrap();
    assert_eq!(mgr.get_status(&id), Some(TaskStatus::InProgress));
    assert!(mgr.list_pending().is_empty());

    let result = TaskResult::success(&id, serde_json::json!({"done": true}));
    mgr.complete(&id, result).unwrap();
    assert_eq!(mgr.get_status(&id), Some(TaskStatus::Completed));

    let record = mgr.get(&id).unwrap();
    assert_eq!(record.result.as_ref().unwrap()["done"], true);
}

#[test]
fn task_manager_not_found() {
    let mut mgr = TaskManager::new();
    assert!(mgr.update_status("ghost", TaskStatus::Failed).is_err());
    assert!(mgr.complete("ghost", TaskResult::failure("ghost", serde_json::json!(null))).is_err());
    assert!(mgr.get("ghost").is_none());
}

#[test]
fn task_manager_remove() {
    let mut mgr = TaskManager::new();
    let id = mgr.create("t", "test", serde_json::json!(null), "!r:t");
    assert!(mgr.remove(&id).is_some());
    assert!(mgr.is_empty());
    assert!(mgr.remove(&id).is_none());
}

#[test]
fn task_manager_list_by_status() {
    let mut mgr = TaskManager::new();
    mgr.create("t1", "a", serde_json::json!(null), "!r:t");
    mgr.create("t2", "b", serde_json::json!(null), "!r:t");
    mgr.create("t3", "c", serde_json::json!(null), "!r:t");
    mgr.update_status("t2", TaskStatus::InProgress).unwrap();

    assert_eq!(mgr.list_pending().len(), 2);
    assert_eq!(mgr.list_by_status(TaskStatus::InProgress).len(), 1);
    assert_eq!(mgr.list_by_status(TaskStatus::Completed).len(), 0);
}

// ---------------------------------------------------------------------------
// Enums
// ---------------------------------------------------------------------------

#[test]
fn priority_default() {
    assert_eq!(Priority::default(), Priority::Normal);
}

#[test]
fn task_status_serde() {
    let json = serde_json::to_string(&TaskStatus::InProgress).unwrap();
    assert_eq!(json, "\"inprogress\"");
    let status: TaskStatus = serde_json::from_str(&json).unwrap();
    assert_eq!(status, TaskStatus::InProgress);
}

#[test]
fn priority_serde() {
    let json = serde_json::to_string(&Priority::Critical).unwrap();
    assert_eq!(json, "\"critical\"");
    let p: Priority = serde_json::from_str(&json).unwrap();
    assert_eq!(p, Priority::Critical);
}

// ---------------------------------------------------------------------------
// Mention parsing
// ---------------------------------------------------------------------------

#[test]
fn parse_mentions_multiple() {
    let body = "Hey @alice:server.com and @bob:server.com, check this out";
    let mentions = parse_mentions(body);
    assert_eq!(mentions.len(), 2);
    assert_eq!(mentions[0], "@alice:server.com");
    assert_eq!(mentions[1], "@bob:server.com");
}

#[test]
fn parse_mentions_with_port() {
    let mentions = parse_mentions("@agent:localhost:8448 please help");
    assert_eq!(mentions, vec!["@agent:localhost:8448"]);
}

#[test]
fn parse_mentions_empty() {
    assert!(parse_mentions("no mentions here").is_empty());
    assert!(parse_mentions("").is_empty());
}

#[test]
fn parse_mentions_at_sign_alone() {
    assert!(parse_mentions("email@ is not a mention").is_empty());
}

// ---------------------------------------------------------------------------
// Mention formatting
// ---------------------------------------------------------------------------

#[test]
fn format_mention_message_basic() {
    let (plain, html) = format_mention_message(
        "@bot:constellation.local",
        "@bot",
        "do the thing",
    );
    assert_eq!(plain, "@bot do the thing");
    assert!(html.contains("https://matrix.to/#/@bot:constellation.local"));
    assert!(html.contains("do the thing"));
}

#[test]
fn format_mention_message_xss_prevention() {
    let (_, html) = format_mention_message(
        "@agent:server",
        "@agent",
        "<img onerror=alert(1) src=x>",
    );
    assert!(!html.contains("<img"));
    assert!(html.contains("&lt;img"));
}

// ---------------------------------------------------------------------------
// Utils
// ---------------------------------------------------------------------------

#[test]
fn sanitize_username_various() {
    assert_eq!(sanitize_username("Research Agent"), "research-agent");
    assert_eq!(sanitize_username("Agent #1 (Test)"), "agent-1-test");
    assert_eq!(sanitize_username("UPPER_case.name"), "upper_case.name");
    assert_eq!(sanitize_username("  spaces  "), "spaces");
    assert_eq!(sanitize_username(""), "");
}

#[test]
fn format_room_alias_basic() {
    assert_eq!(
        format_room_alias("constellation", "constellation.local"),
        "#constellation:constellation.local"
    );
}

#[test]
fn parse_homeserver_url_valid() {
    let url = parse_homeserver_url("http://localhost:6167").unwrap();
    assert_eq!(url.host_str(), Some("localhost"));
    assert_eq!(url.port(), Some(6167));
}

#[test]
fn parse_homeserver_url_rejects_ftp() {
    assert!(parse_homeserver_url("ftp://example.com").is_err());
}

#[test]
fn parse_homeserver_url_rejects_garbage() {
    assert!(parse_homeserver_url("not a url").is_err());
}

#[test]
fn generate_device_id_format() {
    let id = generate_device_id();
    assert!(id.starts_with("CONSTELLATION_"));
    assert_eq!(id.len(), 26); // 14 prefix + 12 hex

    let id2 = generate_device_id();
    assert_ne!(id, id2);
}
