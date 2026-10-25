//! Renders a [`Config`] into the parameter file passed to nfqws2 as `@<file>`, one option per line.
//! Pure (no filesystem, no process), so the GUI can preview exactly what the daemon runs.

use crate::config::{Action, Config, Filter, Strategy};
use crate::registry::Platform;

/// Options every instance gets, independent of the config.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EngineOptions {
    pub platform: Platform,
    /// nfqueue number on Linux. Ignored elsewhere.
    pub queue_num: u16,
    /// Written by the engine so the daemon can supervise it.
    pub pidfile: String,
    /// `--debug=@<path>`; the file the GUI's log viewer tails.
    pub debug_log: Option<String>,
    /// zapret2's Lua library, in load order. Without it the engine dies on the first action with
    /// `desync function 'multisplit' does not exist`.
    pub lua_init: Vec<String>,
    /// Render for validation: `--intercept=0` loads the Lua and resolves every action, then exits.
    pub validate: bool,
    /// The learned-hosts file for `--hostlist-auto`. Chosen by the daemon, never by a config: the
    /// engine appends to it, and a config is untrusted input.
    pub auto_hostlist: Option<String>,
    /// The user's own list for `--hostlist`; the daemon's choice too.
    pub manual_hostlist: Option<String>,
}

impl EngineOptions {
    pub fn new(platform: Platform) -> Self {
        Self {
            platform,
            queue_num: 200,
            pidfile: String::new(),
            debug_log: None,
            lua_init: Vec::new(),
            validate: false,
            auto_hostlist: None,
            manual_hostlist: None,
        }
    }
}

/// The IP TTL the daemon's probes carry on Windows, which has no per-socket firewall mark. The
/// filter below keeps outgoing packets with this TTL out of the engine, so only a probe skips it.
/// Windows defaults to 128, so 127 is one nobody picks; a stray connection that does carry it
/// merely goes unprotected.
pub const PROBE_TTL: u8 = 127;

/// With the list file empty nfqws2 treats the profile as unfiltered and applies it to every host,
/// the opposite of "only my list". A domain that cannot exist keeps the list from ever being empty.
const LIST_SENTINEL: &str = "blkbstr.invalid";

/// Renders the parameter file passed to the engine as `@<file>`.
pub fn parameter_file(config: &Config, engine: &EngineOptions) -> String {
    let mut lines: Vec<String> = Vec::new();
    let auto = engine
        .auto_hostlist
        .as_deref()
        .filter(|_| config.uses_auto_hostlist());
    let manual = engine
        .manual_hostlist
        .as_deref()
        .filter(|_| config.uses_manual_hostlist());

    if engine.validate {
        lines.push("--intercept=0".into());
    }
    match engine.platform {
        Platform::Linux => lines.push(format!("--qnum={}", engine.queue_num)),
        // winws2 builds its own WinDivert filter from these; dvtws2 takes a divert port the daemon owns.
        // Outgoing only: SYN+ACK, FIN and RST come back without asking, and intercepting every
        // incoming segment of a download costs CPU that no shipped strategy needs.
        Platform::Windows => {
            let (tcp, udp) = intercept_ports(config);
            if !tcp.is_empty() {
                lines.push(format!("--wf-tcp-out={tcp}"));
            }
            if !udp.is_empty() {
                lines.push(format!("--wf-udp-out={udp}"));
            }
            // The failure detector reads what comes back (resets, retransmissions, redirects).
            if auto.is_some() {
                if !tcp.is_empty() {
                    lines.push(format!("--wf-tcp-in={tcp}"));
                }
                if !udp.is_empty() {
                    lines.push(format!("--wf-udp-in={udp}"));
                }
                // ANDed onto the generated filter. `ip` and `ipv6` guard each comparison because a
                // field the packet does not have must not read as "not 127".
                lines.push(format!(
                    "--wf-raw-filter=inbound or (ip and ip.TTL != {PROBE_TTL}) or (ipv6 and ipv6.HopLimit != {PROBE_TTL})"
                ));
            }
        }
        Platform::Bsd => {}
    }
    if !engine.pidfile.is_empty() {
        lines.push(format!("--pidfile={}", engine.pidfile));
    }
    if let Some(log) = &engine.debug_log {
        lines.push(format!("--debug=@{log}"));
    }
    for script in &engine.lua_init {
        lines.push(format!("--lua-init=@{script}"));
    }

    // nfqws2 has profile 1 open already, so `--new` for the first strategy would leave it empty,
    // and an empty profile matches every packet and passes it through untouched.
    for (n, strategy) in config.strategies.iter().filter(|s| s.enabled).enumerate() {
        lines.push(String::new());
        let flag = if n == 0 { "--name" } else { "--new" };
        lines.push(format!("{flag}={}", strategy.name));
        lines.extend(filter_lines(&strategy.filter));
        if strategy.takes_hostlists() {
            if let Some(path) = manual {
                lines.push(format!("--hostlist={path}"));
                lines.push(format!("--hostlist-domains={LIST_SENTINEL}"));
            }
            if let Some(path) = auto {
                lines.push(format!("--hostlist-auto={path}"));
            }
        }
        for action in &strategy.actions {
            lines.extend(action_lines(action));
        }
    }

    let mut out = lines.join("\n");
    out.push('\n');
    out
}

/// The TCP and UDP ports the enabled strategies match, each merged and sorted. Intercepting more
/// is pure CPU cost, so this is what both the nftables rules and winws2's WinDivert filter use.
pub fn intercept_ports(config: &Config) -> (String, String) {
    let enabled = || config.strategies.iter().filter(|s| s.enabled);
    (
        collect_ports(enabled(), |s| s.filter.tcp.as_deref()),
        collect_ports(enabled(), |s| s.filter.udp.as_deref()),
    )
}

fn collect_ports<'a>(
    strategies: impl Iterator<Item = &'a Strategy>,
    pick: impl Fn(&'a Strategy) -> Option<&'a str>,
) -> String {
    let mut ports: Vec<&str> = strategies
        .filter_map(pick)
        .flat_map(|p| p.split(','))
        .map(str::trim)
        // `~` negates and `*` means everything; neither is a port nftables can put in a set.
        .filter(|p| !p.is_empty() && !p.starts_with('~') && *p != "*")
        .collect();
    ports.sort_unstable();
    ports.dedup();
    ports.join(",")
}

fn filter_lines(filter: &Filter) -> Vec<String> {
    let mut lines = Vec::new();
    for (flag, value) in [
        ("--filter-l3", &filter.l3),
        ("--filter-tcp", &filter.tcp),
        ("--filter-udp", &filter.udp),
        ("--hostlist", &filter.hostlist),
        ("--hostlist-exclude", &filter.hostlist_exclude),
        ("--hostlist-auto", &filter.hostlist_auto),
        ("--ipset", &filter.ipset),
        ("--ipset-exclude", &filter.ipset_exclude),
    ] {
        if let Some(value) = value {
            lines.push(format!("{flag}={value}"));
        }
    }
    if !filter.l7.is_empty() {
        lines.push(format!("--filter-l7={}", filter.l7.join(",")));
    }
    lines
}

fn action_lines(action: &Action) -> Vec<String> {
    let mut lines = Vec::new();
    if !action.payload.is_empty() {
        lines.push(format!("--payload={}", action.payload.join(",")));
    }

    // `--lua-desync=fn:key=value:bare_flag`; an empty value is a bare flag, e.g. `badsum`.
    let mut call = format!("--lua-desync={}", action.function);
    for (key, value) in &action.args {
        if value.is_empty() {
            call.push_str(&format!(":{key}"));
        } else {
            call.push_str(&format!(":{key}={value}"));
        }
    }
    lines.push(call);
    lines
}

/// Arguments for any invocation. `@<file>` must be the only argument: nfqws2 silently ignores
/// anything after it, so flags go inside the file. `--daemon` is absent because the daemon
/// supervises the engine as a child, and a forked process cannot be supervised.
pub fn run_args(parameter_file_path: &str) -> Vec<String> {
    vec![format!("@{parameter_file_path}")]
}

/// A minimal working config: TLS on 443 with a fake ClientHello, falling back to `multidisorder`.
pub fn starter_config(name: &str) -> Config {
    let mut config = Config::new(name);
    let mut https = Strategy::new("https");
    https.filter = Filter {
        tcp: Some("443".into()),
        l7: vec!["tls".into()],
        ..Filter::default()
    };
    https.actions = vec![
        Action::new("fake")
            .with("blob", "fake_default_tls")
            .with("badsum", ""),
        // Without `pos`, multidisorder splits after one byte.
        Action::new("multidisorder").with("pos", "1,midsld"),
    ];
    for action in &mut https.actions {
        action.payload = vec!["tls_client_hello".into()];
    }
    config.strategies.push(https);
    config
}

#[cfg(test)]
mod tests {
    use super::*;

    fn engine() -> EngineOptions {
        EngineOptions {
            platform: Platform::Linux,
            queue_num: 200,
            pidfile: "/run/blkbstr/engine.pid".into(),
            debug_log: Some("/var/log/blkbstr/engine.log".into()),
            lua_init: vec!["/opt/zapret2/lua/zapret-lib.lua".into()],
            validate: false,
            auto_hostlist: None,
            manual_hostlist: None,
        }
    }

    #[test]
    fn renders_a_starter_config() {
        let out = parameter_file(&starter_config("t"), &engine());
        let lines: Vec<_> = out.lines().filter(|l| !l.is_empty()).collect();
        assert_eq!(
            lines,
            [
                "--qnum=200",
                "--pidfile=/run/blkbstr/engine.pid",
                "--debug=@/var/log/blkbstr/engine.log",
                "--lua-init=@/opt/zapret2/lua/zapret-lib.lua",
                "--name=https",
                "--filter-tcp=443",
                "--filter-l7=tls",
                "--payload=tls_client_hello",
                "--lua-desync=fake:badsum:blob=fake_default_tls",
                "--payload=tls_client_hello",
                "--lua-desync=multidisorder:pos=1,midsld",
            ]
        );
    }

    /// nfqws2 ignores anything after `@<file>`, so `--intercept=0` in argv would start it for real.
    #[test]
    fn validation_goes_in_the_file_not_in_argv() {
        let out = parameter_file(
            &starter_config("t"),
            &EngineOptions {
                validate: true,
                ..engine()
            },
        );
        assert_eq!(out.lines().next(), Some("--intercept=0"));
        assert_eq!(run_args("/run/blkbstr/nfqws2.conf").len(), 1);
    }

    #[test]
    fn empty_argument_values_render_as_bare_flags() {
        let action = Action::new("fake").with("badsum", "").with("strategy", "1");
        assert_eq!(
            action_lines(&action),
            ["--lua-desync=fake:badsum:strategy=1"]
        );
    }

    #[test]
    fn action_order_survives_rendering() {
        let mut config = Config::new("t");
        let mut s = Strategy::new("s");
        s.actions = vec![
            Action::new("circular"),
            Action::new("fake"),
            Action::new("multisplit"),
        ];
        config.strategies.push(s);

        let out = parameter_file(&config, &engine());
        let calls: Vec<_> = out
            .lines()
            .filter(|l| l.starts_with("--lua-desync"))
            .collect();
        assert_eq!(
            calls,
            [
                "--lua-desync=circular",
                "--lua-desync=fake",
                "--lua-desync=multisplit"
            ]
        );
    }

    #[test]
    fn disabled_strategies_are_not_rendered() {
        let mut config = starter_config("t");
        config.strategies[0].enabled = false;
        let out = parameter_file(&config, &engine());
        assert!(!out.contains("--name=") && !out.contains("--new="), "{out}");
    }

    #[test]
    fn windows_declares_its_ports_instead_of_firewall_rules() {
        let mut config = starter_config("t");
        config.strategies[0].filter.udp = Some("443".into());
        let out = parameter_file(
            &config,
            &EngineOptions {
                platform: Platform::Windows,
                ..engine()
            },
        );
        assert!(out.contains("--wf-tcp-out=443\n"), "{out}");
        assert!(out.contains("--wf-udp-out=443\n"), "{out}");
        // They are global options, so they must precede the first profile.
        assert!(out.find("--wf-tcp-out").unwrap() < out.find("--name=").unwrap());

        let linux = parameter_file(&config, &engine());
        assert!(!linux.contains("--wf-"), "{linux}");
    }

    #[test]
    fn the_learned_list_goes_on_strategies_without_a_list_of_their_own() {
        let options = EngineOptions {
            auto_hostlist: Some("/var/lib/blkbstr/autohostlist.txt".into()),
            ..engine()
        };
        let mut config = starter_config("t");
        let mut listed = Strategy::new("listed");
        listed.filter.hostlist = Some("/etc/blkbstr/blocked.txt".into());
        config.strategies.push(listed);

        let out = parameter_file(&config, &options);
        assert_eq!(out.matches("--hostlist-auto=").count(), 1, "{out}");
        let (first, second) = out.split_once("--new=listed").unwrap();
        assert!(first.contains("--hostlist-auto=/var/lib/blkbstr/autohostlist.txt"));
        assert!(!second.contains("--hostlist-auto"));

        config.auto_hostlist = false;
        assert!(!parameter_file(&config, &options).contains("--hostlist-auto"));
        // No file chosen by the daemon, no list: a config cannot ask for one.
        config.auto_hostlist = true;
        assert!(!parameter_file(&config, &engine()).contains("--hostlist-auto"));
    }

    #[test]
    fn the_own_list_is_never_empty_to_the_engine() {
        let options = EngineOptions {
            manual_hostlist: Some("/var/lib/blkbstr/hostlist.txt".into()),
            ..engine()
        };
        let mut config = starter_config("t");
        // Off by default: nothing is rendered until the config asks for it.
        assert!(!parameter_file(&config, &options).contains("--hostlist"));

        config.manual_hostlist = true;
        let out = parameter_file(&config, &options);
        assert!(
            out.contains("--hostlist=/var/lib/blkbstr/hostlist.txt\n"),
            "{out}"
        );
        assert!(
            out.contains("--hostlist-domains=blkbstr.invalid\n"),
            "{out}"
        );
        assert!(!out.contains("--hostlist-auto"), "{out}");

        // Both lists: a host on either one gets the strategy.
        config.auto_hostlist = true;
        let both = EngineOptions {
            auto_hostlist: Some("/var/lib/blkbstr/autohostlist.txt".into()),
            ..options
        };
        let out = parameter_file(&config, &both);
        assert!(
            out.contains("--hostlist=") && out.contains("--hostlist-auto="),
            "{out}"
        );
    }

    #[test]
    fn windows_reads_incoming_traffic_only_when_the_learned_list_needs_it() {
        let windows = |auto: Option<&str>| EngineOptions {
            platform: Platform::Windows,
            auto_hostlist: auto.map(str::to_owned),
            ..engine()
        };
        let config = starter_config("t");
        let with = parameter_file(&config, &windows(Some("C:/a.txt")));
        assert!(with.contains("--wf-tcp-in=443\n"), "{with}");
        // The daemon's probes carry this TTL; the engine must not capture them going out.
        assert!(
            with.contains("ip.TTL != 127") && with.contains("ipv6.HopLimit != 127"),
            "{with}"
        );
        assert!(with.contains("--wf-raw-filter=inbound or"), "{with}");
        let without = parameter_file(&config, &windows(None));
        assert!(
            !without.contains("--wf-tcp-in") && !without.contains("--wf-raw-filter"),
            "{without}"
        );
        assert!(!parameter_file(&config, &engine()).contains("--wf-raw-filter"));
    }

    #[test]
    fn ports_are_merged_deduplicated_and_sorted() {
        let mut config = Config::new("t");
        config.strategies = vec![
            ports("a", Some("443"), Some("443")),
            ports("b", Some("80,443"), None),
        ];
        assert_eq!(intercept_ports(&config), ("443,80".into(), "443".into()));
    }

    #[test]
    fn wildcards_and_negations_are_not_treated_as_ports() {
        let mut config = Config::new("t");
        config.strategies = vec![
            ports("a", Some("*"), None),
            ports("b", Some("~22,443"), None),
        ];
        assert_eq!(intercept_ports(&config).0, "443");
    }

    #[test]
    fn disabled_strategies_contribute_no_ports() {
        let mut config = Config::new("t");
        let mut off = ports("off", Some("8080"), None);
        off.enabled = false;
        config.strategies = vec![off, ports("on", Some("443"), None)];
        assert_eq!(intercept_ports(&config).0, "443");
    }

    fn ports(name: &str, tcp: Option<&str>, udp: Option<&str>) -> Strategy {
        let mut s = Strategy::new(name);
        s.filter = Filter {
            tcp: tcp.map(str::to_owned),
            udp: udp.map(str::to_owned),
            ..Filter::default()
        };
        s
    }

    #[test]
    fn queue_number_is_linux_only() {
        let config = starter_config("t");
        for platform in [Platform::Windows, Platform::Bsd] {
            let out = parameter_file(
                &config,
                &EngineOptions {
                    platform,
                    ..engine()
                },
            );
            assert!(!out.contains("--qnum"), "{platform:?}: {out}");
        }
    }

    #[test]
    fn every_rendered_line_is_a_single_option() {
        // One line, one option; a failure here means a value is smuggling extra options in.
        let config = starter_config("t");
        config.validate().unwrap();
        for line in parameter_file(&config, &engine()).lines() {
            assert!(
                line.is_empty() || line.starts_with("--"),
                "unexpected line {line:?}"
            );
        }
    }
}
