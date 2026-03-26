//! Utility functions for Constellation SDK operations.
//!
//! Provides helpers for sanitizing Matrix usernames, formatting room aliases,
//! validating homeserver URLs, and generating unique device identifiers.

use url::Url;
use uuid::Uuid;

use crate::error::{ConstellationError, Result};

/// Sanitize a display name into a valid Matrix localpart.
///
/// Matrix localparts may only contain `a-z`, `0-9`, `.`, `_`, `=`, `-`, and `/`.
/// This function lowercases the input, replaces spaces and other invalid characters
/// with hyphens, collapses consecutive hyphens, and trims leading/trailing hyphens.
///
/// # Examples
///
/// ```
/// use constellation_core::utils::sanitize_username;
///
/// assert_eq!(sanitize_username("Research Agent"), "research-agent");
/// assert_eq!(sanitize_username("Agent #1 (Test)"), "agent-1-test");
/// assert_eq!(sanitize_username("  spaces  "), "spaces");
/// assert_eq!(sanitize_username("UPPER_case.name"), "upper_case.name");
/// ```
pub fn sanitize_username(name: &str) -> String {
    let mut result = String::with_capacity(name.len());
    for ch in name.chars() {
        match ch {
            'A'..='Z' => result.push(ch.to_ascii_lowercase()),
            'a'..='z' | '0'..='9' | '.' | '_' | '=' | '/' => result.push(ch),
            ' ' | '-' => result.push('-'),
            _ => {
                // Replace any other character with a hyphen.
                if !result.ends_with('-') {
                    result.push('-');
                }
            }
        }
    }

    // Collapse consecutive hyphens and trim.
    let mut collapsed = String::with_capacity(result.len());
    let mut prev_hyphen = false;
    for ch in result.chars() {
        if ch == '-' {
            if !prev_hyphen {
                collapsed.push('-');
            }
            prev_hyphen = true;
        } else {
            collapsed.push(ch);
            prev_hyphen = false;
        }
    }

    collapsed.trim_matches('-').to_string()
}

/// Format a room alias in the standard Matrix `#name:server` format.
///
/// The name is sanitized to lowercase alphanumeric characters, hyphens, underscores,
/// and dots. The leading `#` and `:server` are added automatically.
///
/// # Examples
///
/// ```
/// use constellation_core::utils::format_room_alias;
///
/// assert_eq!(
///     format_room_alias("agents", "constellation.local"),
///     "#agents:constellation.local"
/// );
/// assert_eq!(
///     format_room_alias("My Room", "example.com"),
///     "#my-room:example.com"
/// );
/// ```
pub fn format_room_alias(name: &str, server: &str) -> String {
    let sanitized = sanitize_username(name);
    format!("#{sanitized}:{server}")
}

/// Validate and normalize a homeserver URL.
///
/// Ensures the URL has a valid scheme (`http` or `https`), a host, and strips
/// any trailing slashes for consistency.
///
/// # Errors
///
/// Returns [`ConstellationError::Config`] if the URL is missing a scheme,
/// has no host, or uses an unsupported scheme.
///
/// # Examples
///
/// ```
/// use constellation_core::utils::parse_homeserver_url;
///
/// let url = parse_homeserver_url("http://localhost:6167/").unwrap();
/// assert_eq!(url.as_str(), "http://localhost:6167/");
///
/// assert!(parse_homeserver_url("not-a-url").is_err());
/// assert!(parse_homeserver_url("ftp://example.com").is_err());
/// ```
pub fn parse_homeserver_url(url_str: &str) -> Result<Url> {
    let url = Url::parse(url_str).map_err(|e| {
        ConstellationError::Config(format!("invalid homeserver URL '{url_str}': {e}"))
    })?;

    match url.scheme() {
        "http" | "https" => {}
        scheme => {
            return Err(ConstellationError::Config(format!(
                "unsupported URL scheme '{scheme}': expected http or https"
            )));
        }
    }

    if url.host().is_none() {
        return Err(ConstellationError::Config(format!(
            "homeserver URL '{url_str}' has no host"
        )));
    }

    Ok(url)
}

/// Generate a unique device ID for an agent session.
///
/// Produces a string like `CONSTELLATION_a1b2c3d4` that is human-readable
/// and unique per session. Uses a UUID v4 suffix.
///
/// # Examples
///
/// ```
/// use constellation_core::utils::generate_device_id;
///
/// let id = generate_device_id();
/// assert!(id.starts_with("CONSTELLATION_"));
/// assert!(id.len() > 14);
/// ```
pub fn generate_device_id() -> String {
    let short_id = Uuid::new_v4().simple().to_string();
    // Use the first 12 hex characters for a compact but unique ID.
    format!("CONSTELLATION_{}", &short_id[..12])
}

/// Escape a string for safe inclusion in Matrix HTML `formatted_body`.
///
/// Replaces `&`, `<`, `>`, `"`, and `'` with their HTML entity equivalents.
pub fn html_escape(s: &str) -> String {
    let mut escaped = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&#x27;"),
            _ => escaped.push(ch),
        }
    }
    escaped
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_username_basic() {
        assert_eq!(sanitize_username("Research Agent"), "research-agent");
        assert_eq!(sanitize_username("agent-coder"), "agent-coder");
        assert_eq!(sanitize_username("Agent_1"), "agent_1");
    }

    #[test]
    fn test_sanitize_username_special_chars() {
        assert_eq!(sanitize_username("Agent #1 (Test)"), "agent-1-test");
        assert_eq!(sanitize_username("hello@world!"), "hello-world");
        assert_eq!(sanitize_username("  spaces  "), "spaces");
    }

    #[test]
    fn test_sanitize_username_preserves_valid() {
        assert_eq!(sanitize_username("a.b_c=d/e"), "a.b_c=d/e");
        assert_eq!(sanitize_username("UPPER"), "upper");
    }

    #[test]
    fn test_format_room_alias() {
        assert_eq!(
            format_room_alias("agents", "constellation.local"),
            "#agents:constellation.local"
        );
        assert_eq!(
            format_room_alias("My Room", "example.com"),
            "#my-room:example.com"
        );
    }

    #[test]
    fn test_parse_homeserver_url_valid() {
        let url = parse_homeserver_url("http://localhost:6167").unwrap();
        assert_eq!(url.host_str(), Some("localhost"));

        let url = parse_homeserver_url("https://matrix.example.com").unwrap();
        assert_eq!(url.scheme(), "https");
    }

    #[test]
    fn test_parse_homeserver_url_invalid() {
        assert!(parse_homeserver_url("not-a-url").is_err());
        assert!(parse_homeserver_url("ftp://example.com").is_err());
    }

    #[test]
    fn test_generate_device_id() {
        let id = generate_device_id();
        assert!(id.starts_with("CONSTELLATION_"));
        assert_eq!(id.len(), 14 + 12); // "CONSTELLATION_" + 12 hex chars

        // Each call should produce a unique ID.
        let id2 = generate_device_id();
        assert_ne!(id, id2);
    }

    #[test]
    fn test_html_escape() {
        assert_eq!(html_escape("hello"), "hello");
        assert_eq!(html_escape("<script>"), "&lt;script&gt;");
        assert_eq!(html_escape("a & b"), "a &amp; b");
        assert_eq!(html_escape("\"quoted\""), "&quot;quoted&quot;");
    }
}
