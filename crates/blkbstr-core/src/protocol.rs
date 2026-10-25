//! GUI and daemon wire protocol: newline-delimited JSON over a local socket. Request and response
//! only; logs are not streamed, the GUI reads the files under [`crate::paths::log_dir`].

use crate::config::Config;
use serde::{Deserialize, Serialize};
use std::io::{BufRead, Write};

/// Every request carries this; the daemon refuses mismatched majors rather than guessing.
pub const PROTOCOL_VERSION: u32 = 2;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum Request {
    /// Liveness check; also how the GUI learns whether the daemon is installed.
    Ping {
        protocol: u32,
    },
    Status,
    /// Apply a config and start the engine. `ephemeral` makes it a trial that is not persisted.
    Start {
        config: Box<Config>,
        #[serde(default)]
        ephemeral: bool,
    },
    /// Keep a trial run: cancels the automatic revert and persists the config for boot.
    Confirm,
    /// Stop the engine, leaving the firewall in its pre-start state.
    Stop,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "result", rename_all = "snake_case")]
pub enum Response {
    Pong {
        daemon_version: String,
        protocol: u32,
    },
    Status(EngineStatus),
    Ok,
    Error {
        code: ErrorCode,
        message: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    /// Daemon and GUI disagree on [`PROTOCOL_VERSION`]; one of them needs updating.
    ProtocolMismatch,
    /// Malformed request or invalid config.
    BadRequest,
    /// The engine itself failed (missing binary, firewall rejected the rules, ...).
    EngineFailed,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct EngineStatus {
    pub running: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_config: Option<String>,
    /// True when the active config was started with `ephemeral` and will not survive a restart.
    #[serde(default)]
    pub ephemeral: bool,
    /// Seconds until an unconfirmed trial run is undone; `None` once kept or when not a trial.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revert_in_seconds: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pid: Option<u32>,
    /// Unix epoch seconds the engine started at.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub started_at: Option<u64>,
    /// Version reported by the zapret2 engine binary, when it could be determined.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub engine_version: Option<String>,
    /// Why the engine stopped by itself. Outlives the process so the GUI can explain a crash.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_error: Option<String>,
}

/// Writes one framed message. Both sides must flush before waiting for a reply or they deadlock.
pub fn write_message<T: Serialize>(w: &mut impl Write, msg: &T) -> std::io::Result<()> {
    serde_json::to_writer(&mut *w, msg)?;
    w.write_all(b"\n")?;
    w.flush()
}

/// Reads one framed message. `Ok(None)` means the peer closed the connection cleanly.
pub fn read_message<T: for<'de> Deserialize<'de>>(
    r: &mut impl BufRead,
) -> std::io::Result<Option<T>> {
    let mut line = String::new();
    if r.read_line(&mut line)? == 0 {
        return Ok(None);
    }
    serde_json::from_str(&line)
        .map(Some)
        .map_err(std::io::Error::other)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::BufReader;

    #[test]
    fn frames_survive_a_roundtrip() {
        let mut buf = Vec::new();
        write_message(
            &mut buf,
            &Request::Ping {
                protocol: PROTOCOL_VERSION,
            },
        )
        .unwrap();
        write_message(
            &mut buf,
            &Request::Start {
                config: Box::new(Config::new("x")),
                ephemeral: true,
            },
        )
        .unwrap();

        let mut r = BufReader::new(&buf[..]);
        assert_eq!(
            read_message::<Request>(&mut r).unwrap(),
            Some(Request::Ping {
                protocol: PROTOCOL_VERSION
            })
        );
        assert!(matches!(
            read_message::<Request>(&mut r).unwrap(),
            Some(Request::Start {
                ephemeral: true,
                ..
            })
        ));
        assert_eq!(read_message::<Request>(&mut r).unwrap(), None);
    }

    #[test]
    fn error_responses_are_tagged() {
        let json = serde_json::to_string(&Response::Error {
            code: ErrorCode::EngineFailed,
            message: "nfqws2 not found".into(),
        })
        .unwrap();
        assert!(json.contains(r#""result":"error""#), "{json}");
        assert!(json.contains(r#""code":"engine_failed""#), "{json}");
    }
}
