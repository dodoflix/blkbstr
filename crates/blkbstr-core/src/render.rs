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
        }
    }
}

/// Renders the parameter file passed to the engine as `@<file>`.
pub fn parameter_file(config: &Config, engine: &EngineOptions) -> String {
    let mut lines: Vec<String> = Vec::new();

    if engine.validate {
        lines.push("--intercept=0".into());
    }
    match engine.platform {
        Platform::Linux => lines.push(format!("--qnum={}", engine.queue_num)),
        // winws2 builds its own WinDivert filter; dvtws2 takes a divert port the daemon owns.
        Platform::Windows | Platform::Bsd => {}
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
        for action in &strategy.actions {
            lines.extend(action_lines(action));
        }
    }

    let mut out = lines.join("\n");
    out.push('\n');
    out
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
