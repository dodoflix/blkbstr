//! Runs and supervises the zapret2 packet manipulator.
//!
//! Order matters and is the same on every path: validate the config, write the parameter file,
//! ask the engine to check it with `--dry-run`, install the firewall rules, then start the engine.
//! Rules go in last because rules pointing at a queue nothing reads is the state that breaks a
//! user's network, and they come out first on the way down.

use crate::firewall::{self, Firewall, InterceptSpec};
use anyhow::{bail, Context, Result};
use blkbstr_core::config::{Config, Strategy};
use blkbstr_core::protocol::EngineStatus;
use blkbstr_core::registry::Platform;
use blkbstr_core::render::{self, EngineOptions};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

/// Where the rendered parameter file and the engine's log are written.
const RUNTIME_DIR: &str = "/run/blkbstr";

pub struct Engine {
    platform: Platform,
    binary: PathBuf,
    child: Option<Child>,
    firewall: Firewall,
    status: EngineStatus,
    /// Set when the engine was started with `ephemeral`, so a restart drops it.
    ephemeral: bool,
}

impl Engine {
    pub fn new(platform: Platform) -> Result<Self> {
        let binary = locate(platform.engine_binary())?;
        tracing::info!(binary = %binary.display(), "found engine");
        // Rules from a run that was killed rather than stopped outlive the process that made them.
        Firewall::clear_stale();
        Ok(Self {
            platform,
            binary,
            child: None,
            firewall: Firewall::new(),
            status: EngineStatus::default(),
            ephemeral: false,
        })
    }

    pub fn status(&mut self) -> EngineStatus {
        self.reap();
        self.status.clone()
    }

    /// Notices an engine that exited on its own. Called before reporting status so the GUI does
    /// not show "active" for a process that died ten minutes ago.
    fn reap(&mut self) {
        let Some(child) = self.child.as_mut() else {
            return;
        };
        match child.try_wait() {
            Ok(Some(exit)) => {
                tracing::error!(%exit, "engine exited on its own");
                self.child = None;
                self.status = EngineStatus {
                    last_error: Some(format!("engine exited: {exit}")),
                    ..EngineStatus::default()
                };
                if let Err(e) = self.firewall.teardown() {
                    tracing::error!(error = %e, "could not remove rules after the engine died");
                }
            }
            Ok(None) => {}
            Err(e) => tracing::warn!(error = %e, "could not check on the engine"),
        }
    }

    pub fn start(&mut self, config: &Config, ephemeral: bool) -> Result<()> {
        config.validate()?;
        if self.child.is_some() {
            self.stop().context("restarting")?;
        }

        std::fs::create_dir_all(RUNTIME_DIR).with_context(|| format!("creating {RUNTIME_DIR}"))?;

        let params_path = format!("{RUNTIME_DIR}/nfqws2.conf");
        let options = EngineOptions {
            platform: self.platform,
            queue_num: 200,
            pidfile: format!("{RUNTIME_DIR}/engine.pid"),
            debug_log: Some(format!(
                "{}/engine.log",
                blkbstr_core::paths::log_dir().display()
            )),
        };
        std::fs::write(&params_path, render::parameter_file(config, &options))
            .with_context(|| format!("writing {params_path}"))?;

        self.dry_run(&params_path)?;

        // Rules last, so a config the engine rejects never reaches the network stack.
        let spec = intercept_spec(config, options.queue_num)?;
        self.firewall.apply(&spec)?;

        let child = Command::new(&self.binary)
            .args(render::run_args(&params_path))
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .with_context(|| format!("starting {}", self.binary.display()))?;

        let pid = child.id();
        self.child = Some(child);
        self.ephemeral = ephemeral;
        self.status = EngineStatus {
            running: true,
            active_config: Some(config.name.clone()),
            ephemeral,
            pid: Some(pid),
            started_at: now(),
            engine_version: self.version(),
            last_error: None,
        };
        tracing::info!(config = %config.name, pid, ephemeral, "engine started");
        Ok(())
    }

    /// `--dry-run` checks the options and that referenced files exist, without opening NFQUEUE.
    /// It does not validate the Lua, so a config can pass here and still fail at runtime.
    fn dry_run(&self, params_path: &str) -> Result<()> {
        let out = Command::new(&self.binary)
            .args(render::dry_run_args(params_path))
            .output()
            .with_context(|| format!("running {} --dry-run", self.binary.display()))?;
        if !out.status.success() {
            bail!(
                "the engine rejected this config: {}",
                String::from_utf8_lossy(&out.stderr).trim()
            );
        }
        Ok(())
    }

    fn version(&self) -> Option<String> {
        let out = Command::new(&self.binary).arg("--version").output().ok()?;
        let text = String::from_utf8_lossy(&out.stdout);
        text.lines().next().map(|l| l.trim().to_owned())
    }

    pub fn stop(&mut self) -> Result<()> {
        // Rules first: with them gone, traffic flows normally even if killing the engine drags.
        let rules = self.firewall.teardown();

        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
            tracing::info!("engine stopped");
        }
        self.status = EngineStatus::default();
        self.ephemeral = false;
        rules
    }
}

/// Ports the config actually cares about decide what gets queued. Intercepting more than the
/// strategies match is pure CPU cost.
fn intercept_spec(config: &Config, queue_num: u16) -> Result<InterceptSpec> {
    let enabled = || config.strategies.iter().filter(|s| s.enabled);
    let mut spec = InterceptSpec::new(firewall::default_route_iface()?, queue_num);
    spec.tcp_ports = collect_ports(enabled(), |s| s.filter.tcp.as_deref());
    spec.udp_ports = collect_ports(enabled(), |s| s.filter.udp.as_deref());
    if spec.tcp_ports.is_empty() && spec.udp_ports.is_empty() {
        bail!("no enabled strategy selects any TCP or UDP port, so nothing would be intercepted");
    }
    Ok(spec)
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

/// Looks for the engine next to the daemon, in the usual install locations, then on PATH.
fn locate(binary: &str) -> Result<PathBuf> {
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            candidates.push(dir.join(binary));
        }
    }
    candidates.extend(
        [
            "/opt/zapret2",
            "/usr/local/bin",
            "/usr/bin",
            "/usr/local/libexec",
        ]
        .iter()
        .map(|d| Path::new(d).join(binary)),
    );
    if let Some(found) = candidates.iter().find(|p| p.is_file()) {
        return Ok(found.clone());
    }

    let path = Command::new("sh")
        .arg("-c")
        .arg(format!("command -v {binary}"))
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_owned())
        .filter(|p| !p.is_empty());
    match path {
        Some(p) => Ok(PathBuf::from(p)),
        None => bail!(
            "{binary} not found. Install zapret2 (https://github.com/bol-van/zapret2) or place \
             the binary in /opt/zapret2"
        ),
    }
}

fn now() -> Option<u64> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .map(|d| d.as_secs())
}

#[cfg(test)]
mod tests {
    use super::*;
    use blkbstr_core::config::Filter;

    fn strategy(name: &str, tcp: Option<&str>, udp: Option<&str>) -> Strategy {
        let mut s = Strategy::new(name);
        s.filter = Filter {
            tcp: tcp.map(str::to_owned),
            udp: udp.map(str::to_owned),
            ..Filter::default()
        };
        s
    }

    #[test]
    fn ports_are_merged_deduplicated_and_sorted() {
        let mut cfg = Config::new("t");
        cfg.strategies = vec![
            strategy("a", Some("443"), Some("443")),
            strategy("b", Some("80,443"), None),
        ];
        assert_eq!(
            collect_ports(cfg.strategies.iter(), |s| s.filter.tcp.as_deref()),
            "443,80"
        );
        assert_eq!(
            collect_ports(cfg.strategies.iter(), |s| s.filter.udp.as_deref()),
            "443"
        );
    }

    #[test]
    fn wildcards_and_negations_are_not_treated_as_ports() {
        let cfg = [
            strategy("a", Some("*"), None),
            strategy("b", Some("~22,443"), None),
        ];
        assert_eq!(
            collect_ports(cfg.iter(), |s| s.filter.tcp.as_deref()),
            "443"
        );
    }

    #[test]
    fn disabled_strategies_contribute_no_ports() {
        let mut cfg = Config::new("t");
        let mut off = strategy("off", Some("8080"), None);
        off.enabled = false;
        cfg.strategies = vec![off, strategy("on", Some("443"), None)];
        let enabled = cfg.strategies.iter().filter(|s| s.enabled);
        assert_eq!(collect_ports(enabled, |s| s.filter.tcp.as_deref()), "443");
    }

    #[test]
    fn a_config_that_selects_nothing_is_refused() {
        let mut cfg = Config::new("t");
        cfg.strategies = vec![strategy("a", None, None)];
        // Fails either on the port check or earlier on there being no default route in CI; both
        // are refusals, which is the point.
        assert!(intercept_spec(&cfg, 200).is_err());
    }

    #[test]
    fn missing_engine_binary_names_itself() {
        let e = locate("nfqws2-definitely-not-installed").unwrap_err();
        assert!(e.to_string().contains("nfqws2-definitely-not-installed"));
    }
}
