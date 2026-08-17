//! Platform-agnostic configuration model.
//!
//! A [`Config`] is what the user names, saves, switches between and syncs. It mirrors zapret2's own
//! vocabulary: a [`Strategy`] is one nfqws2 *profile*, and its [`Action`]s are the ordered
//! `--lua-desync` calls inside it. It is not a command line — [`crate::render`] turns it into one,
//! and [`crate::registry`] reports what this build does not recognise.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Bumped whenever a config file needs migration. Files carrying a higher number are rejected
/// rather than silently misread.
pub const SCHEMA_VERSION: u32 = 1;

/// The `NFQWS2_COMPAT_VER` this build renders for. Upstream bumps it on every API break — v2
/// replaced the `stun_binding_req` payload, v3 restructured `desync.track` — so a config records
/// which one its actions were written against.
pub const NFQWS2_COMPAT_VER: u32 = 3;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_schema")]
    pub schema: u32,
    /// User-facing name; also the file stem under the configs directory.
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    #[serde(default = "default_compat")]
    pub compat: u32,
    #[serde(default)]
    pub strategies: Vec<Strategy>,
}

/// One nfqws2 profile: which traffic it claims, and what it does to it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Strategy {
    pub name: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub filter: Filter,
    /// Ordered. nfqws2 runs them in sequence, and orchestrators like `circular` depend on that
    /// order, so this is a list rather than a map.
    #[serde(default)]
    pub actions: Vec<Action>,
}

/// Profile-level filters — the `--filter-*`, `--hostlist*` and `--ipset*` options. Traffic that
/// does not match never reaches the profile's actions.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Filter {
    /// `ipv4` or `ipv6`; unset means both.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub l3: Option<String>,
    /// TCP ports: `443`, `80,443`, `1-1023`, `~22`, `*`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tcp: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub udp: Option<String>,
    /// Application protocols, e.g. `http`, `tls`, `quic`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub l7: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hostlist: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hostlist_exclude: Option<String>,
    /// Self-populating hostlist driven by nfqws2's own failure detector.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hostlist_auto: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ipset: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ipset_exclude: Option<String>,
}

/// One `--lua-desync` call: a function from zapret2's Lua libraries plus its arguments.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Action {
    /// Lua function name — `fake`, `multisplit`, `multidisorder`, `wsize`, `circular`, …
    pub function: String,
    /// `--payload=` filter applied to this action. Empty means whatever the profile already set.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub payload: Vec<String>,
    /// Arguments, rendered as `:key=value`. An empty value is a bare flag: `badsum` in
    /// `--lua-desync=fake:blob=fake_default_tls:badsum`.
    #[serde(default)]
    pub args: BTreeMap<String, String>,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ConfigError {
    #[error("config schema v{found} is newer than supported v{supported}")]
    UnsupportedSchema { found: u32, supported: u32 },
    #[error("config name must be a non-empty single path segment")]
    BadName,
    #[error("{field} {value:?} is not a bare identifier")]
    BadToken { field: &'static str, value: String },
    #[error("{field} {value:?} contains a character that would inject an nfqws2 option")]
    UnsafeValue { field: &'static str, value: String },
    #[error("config was written for nfqws2 compat v{found}, this build renders v{supported}")]
    CompatMismatch { found: u32, supported: u32 },
}

impl Config {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            schema: SCHEMA_VERSION,
            name: name.into(),
            notes: None,
            compat: NFQWS2_COMPAT_VER,
            strategies: Vec::new(),
        }
    }

    pub fn from_json(s: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(s)
    }

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Rejects configs that would be misread, escape the configs directory, or smuggle extra
    /// options into the rendered nfqws2 parameter file.
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.schema > SCHEMA_VERSION {
            return Err(ConfigError::UnsupportedSchema {
                found: self.schema,
                supported: SCHEMA_VERSION,
            });
        }
        // The name becomes a file stem, so anything that could traverse or reset the path is out.
        let bad = self.name.trim().is_empty()
            || self.name.contains(['/', '\\', '\0'])
            || self.name.starts_with('.');
        if bad {
            return Err(ConfigError::BadName);
        }
        for strategy in &self.strategies {
            strategy.validate()?;
        }
        Ok(())
    }

    /// Separate from [`Self::validate`] because a compat mismatch is a warning the user may
    /// choose to override, not a malformed file.
    pub fn compat_check(&self) -> Result<(), ConfigError> {
        if self.compat == NFQWS2_COMPAT_VER {
            Ok(())
        } else {
            Err(ConfigError::CompatMismatch {
                found: self.compat,
                supported: NFQWS2_COMPAT_VER,
            })
        }
    }
}

impl Strategy {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            enabled: true,
            filter: Filter::default(),
            actions: Vec::new(),
        }
    }

    fn validate(&self) -> Result<(), ConfigError> {
        check_token("strategy name", &self.name)?;
        self.filter.validate()?;
        for action in &self.actions {
            action.validate()?;
        }
        Ok(())
    }
}

impl Filter {
    fn validate(&self) -> Result<(), ConfigError> {
        for (field, value) in [
            ("filter.l3", &self.l3),
            ("filter.tcp", &self.tcp),
            ("filter.udp", &self.udp),
            ("filter.hostlist", &self.hostlist),
            ("filter.hostlist_exclude", &self.hostlist_exclude),
            ("filter.hostlist_auto", &self.hostlist_auto),
            ("filter.ipset", &self.ipset),
            ("filter.ipset_exclude", &self.ipset_exclude),
        ] {
            if let Some(value) = value {
                check_value(field, value)?;
            }
        }
        for proto in &self.l7 {
            check_token("filter.l7", proto)?;
        }
        Ok(())
    }
}

impl Action {
    pub fn new(function: impl Into<String>) -> Self {
        Self {
            function: function.into(),
            payload: Vec::new(),
            args: BTreeMap::new(),
        }
    }

    pub fn with(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.args.insert(key.into(), value.into());
        self
    }

    fn validate(&self) -> Result<(), ConfigError> {
        check_token("action function", &self.function)?;
        for payload in &self.payload {
            check_token("action payload", payload)?;
        }
        for (key, value) in &self.args {
            check_token("action argument", key)?;
            // `:` separates arguments inside --lua-desync, so a value containing one would be
            // read as an extra argument.
            if value.contains(':') {
                return Err(ConfigError::UnsafeValue {
                    field: "action argument value",
                    value: value.clone(),
                });
            }
            check_value("action argument value", value)?;
        }
        Ok(())
    }
}

/// Bare identifier: what can appear as an option or function name without quoting.
fn check_token(field: &'static str, value: &str) -> Result<(), ConfigError> {
    let ok = !value.is_empty()
        && value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.'));
    if ok {
        Ok(())
    } else {
        Err(ConfigError::BadToken {
            field,
            value: value.to_owned(),
        })
    }
}

/// nfqws2 reads its options from a file, one per line, so any newline in a user-supplied value
/// injects an arbitrary option into a process that manipulates the firewall. Configs arrive by
/// sync and by import, so they are untrusted input.
fn check_value(field: &'static str, value: &str) -> Result<(), ConfigError> {
    if value.contains(['\n', '\r', '\0']) {
        Err(ConfigError::UnsafeValue {
            field,
            value: value.to_owned(),
        })
    } else {
        Ok(())
    }
}

fn default_schema() -> u32 {
    SCHEMA_VERSION
}

fn default_compat() -> u32 {
    NFQWS2_COMPAT_VER
}

fn default_true() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Config {
        let mut cfg = Config::new("home-isp");
        let mut https = Strategy::new("https");
        https.filter = Filter {
            tcp: Some("443".into()),
            l7: vec!["tls".into()],
            hostlist: Some("/etc/blkbstr/blocked.txt".into()),
            ..Filter::default()
        };
        https.actions = vec![
            Action::new("fake")
                .with("blob", "fake_default_tls")
                .with("badsum", "")
                .with("strategy", "1"),
            Action::new("multidisorder").with("strategy", "2"),
        ];
        cfg.strategies.push(https);
        cfg
    }

    #[test]
    fn roundtrips() {
        let cfg = sample();
        assert_eq!(Config::from_json(&cfg.to_json().unwrap()).unwrap(), cfg);
        cfg.validate().unwrap();
    }

    #[test]
    fn action_order_is_preserved() {
        let back = Config::from_json(&sample().to_json().unwrap()).unwrap();
        let functions: Vec<_> = back.strategies[0]
            .actions
            .iter()
            .map(|a| a.function.as_str())
            .collect();
        assert_eq!(functions, ["fake", "multidisorder"]);
    }

    #[test]
    fn defaults_fill_in_for_a_minimal_file() {
        let cfg = Config::from_json(r#"{"name":"x","strategies":[{"name":"s"}]}"#).unwrap();
        assert_eq!(cfg.schema, SCHEMA_VERSION);
        assert_eq!(cfg.compat, NFQWS2_COMPAT_VER);
        assert!(cfg.strategies[0].enabled);
        cfg.validate().unwrap();
    }

    #[test]
    fn rejects_traversal_and_future_schemas() {
        for name in ["", "../etc", "a/b", ".hidden"] {
            let mut c = Config::new("ok");
            c.name = name.into();
            assert_eq!(c.validate(), Err(ConfigError::BadName), "{name:?}");
        }
        let mut c = Config::new("ok");
        c.schema = SCHEMA_VERSION + 1;
        assert!(c.validate().is_err());
    }

    #[test]
    fn rejects_values_that_would_inject_an_option() {
        // A newline in a hostlist path would add a line to the nfqws2 parameter file.
        let mut cfg = Config::new("x");
        let mut s = Strategy::new("s");
        s.filter.hostlist = Some("/tmp/a\n--qnum=999".into());
        cfg.strategies.push(s);
        assert!(matches!(
            cfg.validate(),
            Err(ConfigError::UnsafeValue { .. })
        ));

        // A colon in an argument value would be read as a further --lua-desync argument.
        let mut cfg = Config::new("x");
        let mut s = Strategy::new("s");
        s.actions.push(Action::new("fake").with("blob", "a:badsum"));
        cfg.strategies.push(s);
        assert!(matches!(
            cfg.validate(),
            Err(ConfigError::UnsafeValue { .. })
        ));

        // Function names reach the command line directly.
        let mut cfg = Config::new("x");
        let mut s = Strategy::new("s");
        s.actions.push(Action::new("fake --qnum=1"));
        cfg.strategies.push(s);
        assert!(matches!(cfg.validate(), Err(ConfigError::BadToken { .. })));
    }

    #[test]
    fn compat_mismatch_is_separate_from_validity() {
        let mut cfg = sample();
        cfg.compat = NFQWS2_COMPAT_VER - 1;
        cfg.validate().unwrap();
        assert!(cfg.compat_check().is_err());
    }
}
