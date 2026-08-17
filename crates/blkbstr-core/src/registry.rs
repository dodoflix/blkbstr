//! What a config asks for versus what this build knows about.
//!
//! A profile's actions are portable across platforms by construction, because they are Lua rather
//! than compiled-in flags. Only the *interception* layer differs (`--qnum` on Linux, `--wf-*` on
//! Windows, `--port` on BSD), and none of that is user config — the daemon supplies it.
//!
//! So what is worth checking here is whether the Lua functions a config names actually exist. A
//! misspelled `multispilt` is otherwise a strategy that silently does nothing.

use crate::config::Config;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Platform {
    Linux,
    Windows,
    /// FreeBSD and OpenBSD, both served by `dvtws2` over ipfw/pf.
    Bsd,
}

impl Platform {
    /// The platform this binary was built for. macOS is absent on purpose: upstream does not
    /// support it and does not expect to, because Apple removed `ipdivert` from the kernel and
    /// there is no replacement packet-interception facility.
    pub const fn current() -> Option<Self> {
        if cfg!(target_os = "linux") {
            Some(Platform::Linux)
        } else if cfg!(target_os = "windows") {
            Some(Platform::Windows)
        } else if cfg!(any(target_os = "freebsd", target_os = "openbsd")) {
            Some(Platform::Bsd)
        } else {
            None
        }
    }

    /// The zapret2 packet manipulator for this platform.
    pub const fn engine_binary(self) -> &'static str {
        match self {
            Platform::Linux => "nfqws2",
            Platform::Windows => "winws2",
            Platform::Bsd => "dvtws2",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Category {
    Base,
    HttpFooling,
    WindowSize,
    Fake,
    TcpSegmentation,
    UdpFooling,
    Other,
    State,
    Detection,
    Orchestrator,
}

pub struct FunctionSpec {
    pub name: &'static str,
    pub category: Category,
}

/// The `zapret-antidpi.lua` surface, as documented in the zapret2 manual. Extending it is how a
/// new upstream Lua function becomes selectable in the GUI; nothing here is rendered by name, so
/// an out-of-date list costs a spurious warning rather than a broken config.
pub const FUNCTIONS: &[FunctionSpec] = &[
    spec("drop", Category::Base),
    spec("send", Category::Base),
    spec("pktmod", Category::Base),
    spec("http_hostcase", Category::HttpFooling),
    spec("http_domcase", Category::HttpFooling),
    spec("http_methodeol", Category::HttpFooling),
    spec("http_unixeol", Category::HttpFooling),
    spec("wsize", Category::WindowSize),
    spec("wssize", Category::WindowSize),
    spec("syndata", Category::Fake),
    spec("tls_client_hello_clone", Category::Fake),
    spec("fake", Category::Fake),
    spec("rst", Category::Fake),
    spec("multisplit", Category::TcpSegmentation),
    spec("multidisorder", Category::TcpSegmentation),
    spec("multidisorder_legacy", Category::TcpSegmentation),
    spec("fakedsplit", Category::TcpSegmentation),
    spec("fakeddisorder", Category::TcpSegmentation),
    spec("hostfakesplit", Category::TcpSegmentation),
    spec("tcpseg", Category::TcpSegmentation),
    spec("oob", Category::TcpSegmentation),
    spec("udplen", Category::UdpFooling),
    spec("dht_dn", Category::UdpFooling),
    spec("synack", Category::Other),
    spec("synack_split", Category::Other),
    spec("automate_conn_record", Category::State),
    spec("standard_hostkey", Category::State),
    spec("automate_host_record", Category::State),
    spec("automate_failure_counter", Category::Detection),
    spec("automate_failure_counter_reset", Category::Detection),
    spec("automate_failure_check", Category::Detection),
    spec("standard_success_detector", Category::Detection),
    spec("standard_failure_detector", Category::Detection),
    spec("circular", Category::Orchestrator),
    spec("repeater", Category::Orchestrator),
    spec("condition", Category::Orchestrator),
];

const fn spec(name: &'static str, category: Category) -> FunctionSpec {
    FunctionSpec { name, category }
}

pub fn function(name: &str) -> Option<&'static FunctionSpec> {
    FUNCTIONS.iter().find(|f| f.name == name)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Warning {
    /// Strategy the warning came from, or `None` for config-wide ones.
    pub strategy: Option<String>,
    pub message: String,
}

/// Reports what will not do what the config appears to say. Never an error: a config that warns
/// still runs, and nfqws2's own `--dry-run` is the authority on whether it loads.
pub fn lint(config: &Config) -> Vec<Warning> {
    let mut out = Vec::new();

    if let Err(e) = config.compat_check() {
        out.push(Warning {
            strategy: None,
            message: format!("{e}; action arguments may have changed meaning"),
        });
    }

    for strategy in config.strategies.iter().filter(|s| s.enabled) {
        if strategy.actions.is_empty() {
            out.push(Warning {
                strategy: Some(strategy.name.clone()),
                message: "no actions, so this strategy matches traffic and does nothing to it"
                    .into(),
            });
        }
        for action in &strategy.actions {
            if function(&action.function).is_none() {
                out.push(Warning {
                    strategy: Some(strategy.name.clone()),
                    message: format!(
                        "`{}` is not a Lua function this build knows; it is passed through and \
                         will fail at runtime if upstream does not define it",
                        action.function
                    ),
                });
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Action, Strategy};

    fn config_with(functions: &[&str]) -> Config {
        let mut cfg = Config::new("t");
        let mut s = Strategy::new("s");
        s.actions = functions.iter().map(|f| Action::new(*f)).collect();
        cfg.strategies.push(s);
        cfg
    }

    #[test]
    fn flags_unknown_functions_only() {
        let warnings = lint(&config_with(&["fake", "multisplit", "multispilt"]));
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].message.contains("multispilt"), "{warnings:?}");
    }

    #[test]
    fn flags_a_strategy_that_does_nothing() {
        let mut cfg = Config::new("t");
        cfg.strategies.push(Strategy::new("empty"));
        assert_eq!(lint(&cfg).len(), 1);
    }

    #[test]
    fn disabled_strategies_are_not_linted() {
        let mut cfg = config_with(&["nonsense"]);
        cfg.strategies[0].enabled = false;
        assert!(lint(&cfg).is_empty());
    }

    #[test]
    fn compat_mismatch_warns_once_for_the_whole_config() {
        let mut cfg = config_with(&["fake"]);
        cfg.compat = 1;
        let warnings = lint(&cfg);
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].strategy, None);
    }

    #[test]
    fn function_table_has_no_duplicates() {
        let mut names: Vec<_> = FUNCTIONS.iter().map(|f| f.name).collect();
        let total = names.len();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), total);
    }

    #[test]
    fn each_platform_has_its_own_engine_binary() {
        let mut bins: Vec<_> = [Platform::Linux, Platform::Windows, Platform::Bsd]
            .iter()
            .map(|p| p.engine_binary())
            .collect();
        bins.sort_unstable();
        bins.dedup();
        assert_eq!(bins.len(), 3);
    }
}
