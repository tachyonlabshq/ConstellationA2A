//! Additional tests for AgentConfig and AgentConfigBuilder.

use constellation_core::{AgentConfig, AgentConfigBuilder};

// ---------------------------------------------------------------------------
// AgentConfig::default()
// ---------------------------------------------------------------------------

#[test]
fn default_config_has_expected_values() {
    let config = AgentConfig::default();
    assert_eq!(config.homeserver_url, "http://localhost:6167");
    assert!(config.username.is_empty());
    assert!(config.password.is_empty());
    assert!(config.display_name.is_none());
    assert!(config.auto_join_rooms.is_empty());
    assert!(config.device_id.is_none());
}

#[test]
fn default_config_fails_validation() {
    let config = AgentConfig::default();
    assert!(config.validate().is_err(), "default config has empty username/password");
}

// ---------------------------------------------------------------------------
// Builder chaining
// ---------------------------------------------------------------------------

#[test]
fn builder_chaining_returns_self() {
    // Verify all builder methods can be chained in a single expression.
    let config = AgentConfigBuilder::new()
        .homeserver_url("http://matrix.example.com:8448")
        .username("chained-agent")
        .password("chain-pass")
        .display_name("Chained Agent")
        .device_id("DEVICE_CHAIN")
        .auto_join_room("#room-a:example.com")
        .auto_join_room("#room-b:example.com")
        .build()
        .expect("fully chained builder should succeed");

    assert_eq!(config.homeserver_url, "http://matrix.example.com:8448");
    assert_eq!(config.username, "chained-agent");
    assert_eq!(config.password, "chain-pass");
    assert_eq!(config.display_name.as_deref(), Some("Chained Agent"));
    assert_eq!(config.device_id.as_deref(), Some("DEVICE_CHAIN"));
    assert_eq!(config.auto_join_rooms.len(), 2);
    assert_eq!(config.auto_join_rooms[0], "#room-a:example.com");
    assert_eq!(config.auto_join_rooms[1], "#room-b:example.com");
}

#[test]
fn builder_auto_join_rooms_replaces_individual_rooms() {
    let config = AgentConfigBuilder::new()
        .username("agent")
        .password("pass")
        .auto_join_room("#individual:server")
        .auto_join_rooms(vec!["#bulk-a:server".into(), "#bulk-b:server".into()])
        .build()
        .unwrap();

    // auto_join_rooms() should replace, not append
    assert_eq!(config.auto_join_rooms.len(), 2);
    assert_eq!(config.auto_join_rooms[0], "#bulk-a:server");
}

// ---------------------------------------------------------------------------
// Builder with all optional fields
// ---------------------------------------------------------------------------

#[test]
fn builder_with_all_optional_fields() {
    let config = AgentConfigBuilder::new()
        .homeserver_url("https://matrix.prod.example.com")
        .username("full-agent")
        .password("full-pass")
        .display_name("Full Agent Display Name")
        .device_id("FULL_DEVICE_ID_123")
        .auto_join_rooms(vec![
            "#ops:prod.example.com".into(),
            "#alerts:prod.example.com".into(),
            "#general:prod.example.com".into(),
        ])
        .build()
        .expect("builder with all fields should succeed");

    assert_eq!(config.homeserver_url, "https://matrix.prod.example.com");
    assert_eq!(config.username, "full-agent");
    assert_eq!(config.password, "full-pass");
    assert_eq!(config.display_name.as_deref(), Some("Full Agent Display Name"));
    assert_eq!(config.device_id.as_deref(), Some("FULL_DEVICE_ID_123"));
    assert_eq!(config.auto_join_rooms.len(), 3);
}

#[test]
fn builder_minimal_required_fields_only() {
    let config = AgentConfigBuilder::new()
        .username("min-agent")
        .password("min-pass")
        .build()
        .expect("minimal builder should succeed");

    // Should use defaults for everything else
    assert_eq!(config.homeserver_url, "http://localhost:6167");
    assert!(config.display_name.is_none());
    assert!(config.device_id.is_none());
    assert!(config.auto_join_rooms.is_empty());
}

// ---------------------------------------------------------------------------
// Clone trait
// ---------------------------------------------------------------------------

#[test]
fn config_clone_is_independent() {
    let original = AgentConfigBuilder::new()
        .homeserver_url("http://original:6167")
        .username("original-agent")
        .password("original-pass")
        .display_name("Original")
        .device_id("ORIG_DEVICE")
        .auto_join_room("#room:server")
        .build()
        .unwrap();

    let cloned = original.clone();

    // Cloned values should match
    assert_eq!(cloned.homeserver_url, original.homeserver_url);
    assert_eq!(cloned.username, original.username);
    assert_eq!(cloned.password, original.password);
    assert_eq!(cloned.display_name, original.display_name);
    assert_eq!(cloned.device_id, original.device_id);
    assert_eq!(cloned.auto_join_rooms, original.auto_join_rooms);
}

#[test]
fn config_clone_does_not_alias() {
    let original = AgentConfigBuilder::new()
        .username("agent")
        .password("pass")
        .auto_join_room("#room:server")
        .build()
        .unwrap();

    let mut cloned = original.clone();
    cloned.username = "modified-agent".into();
    cloned.auto_join_rooms.push("#extra:server".into());

    // Original should be unaffected
    assert_eq!(original.username, "agent");
    assert_eq!(original.auto_join_rooms.len(), 1);
    assert_eq!(cloned.username, "modified-agent");
    assert_eq!(cloned.auto_join_rooms.len(), 2);
}

// ---------------------------------------------------------------------------
// Builder Default trait
// ---------------------------------------------------------------------------

#[test]
fn builder_default_matches_new() {
    let from_new = AgentConfigBuilder::new()
        .username("test")
        .password("pass")
        .build()
        .unwrap();

    let from_default = AgentConfigBuilder::default()
        .username("test")
        .password("pass")
        .build()
        .unwrap();

    assert_eq!(from_new.homeserver_url, from_default.homeserver_url);
    assert_eq!(from_new.display_name, from_default.display_name);
    assert_eq!(from_new.device_id, from_default.device_id);
    assert_eq!(from_new.auto_join_rooms, from_default.auto_join_rooms);
}
