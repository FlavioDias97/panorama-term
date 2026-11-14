// The desktop tunnel must never expose the sidecar's other local APIs.
pub(super) fn allowed(method: &str, target: &str, websocket: bool) -> bool {
    let path = target.split('?').next().unwrap_or("");
    match (method, path, websocket) {
        ("GET", "/ui" | "/pty", true) => true,
        ("GET", "/kill" | "/run/commands" | "/run/status" | "/run/restart" | "/run/start" | "/run/stop", false) => true,
        ("POST", "/agent/send", false) => true,
        _ => false,
    }
}

// Remote displays may send input, but the host owns the PTY dimensions.
pub(crate) fn pty_message_allowed(text: &str) -> bool {
    serde_json::from_str::<serde_json::Value>(text)
        .ok()
        .and_then(|message| message.get("t").and_then(|t| t.as_str()).map(str::to_owned))
        .is_some_and(|kind| kind != "resize")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allows_only_frontend_routes_and_their_transports() {
        for path in ["/ui", "/pty?tileId=x"] {
            assert!(allowed("GET", path, true));
            assert!(!allowed("GET", path, false));
        }
        for path in ["/kill", "/run/commands", "/run/status", "/run/restart", "/run/start", "/run/stop"] {
            assert!(allowed("GET", path, false));
            assert!(!allowed("POST", path, false));
        }
        assert!(allowed("POST", "/agent/send?tileId=x", false));
        for path in ["/mobile/state", "/mobile/pair", "/run/logs", "/capture", "/pty/../mobile/state", "http://localhost/pty"] {
            assert!(!allowed("GET", path, false));
            assert!(!allowed("GET", path, true));
        }
    }

    #[test]
    fn remote_pty_cannot_resize() {
        assert!(!pty_message_allowed(r#"{"t":"resize","cols":120,"rows":40}"#));
        assert!(pty_message_allowed(r#"{"t":"in","d":"hello"}"#));
        assert!(!pty_message_allowed("invalid"));
    }
}
