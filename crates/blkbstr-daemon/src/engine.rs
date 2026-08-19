//! Runs and supervises the zapret2 packet manipulator.
//!
//! Order matters and is the same on every path: validate the config, write the parameter file,
//! ask the engine to check it with `--intercept=0`, install the firewall rules, then start it.
//! Rules go in last because rules pointing at a queue nothing reads is the state that breaks a
//! user's network, and they come out first on the way down.

use crate::firewall::{self, Firewall, InterceptSpec};
use crate::supervisor::{Decision, Supervisor};
use anyhow::{bail, Context, Result};
use blkbstr_core::config::{Config, Strategy};
use blkbstr_core::detect;
use blkbstr_core::paths;
use blkbstr_core::protocol::EngineStatus;
use blkbstr_core::registry::Platform;
use blkbstr_core::render::{self, EngineOptions};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// What is running, and what to bring back if it dies.
struct Active {
    config: Config,
    ephemeral: bool,
    started_at: Option<u64>,
    /// Set while waiting out a restart backoff; the engine is down until this passes.
    restart_at: Option<Instant>,
    /// Dead-man's switch for a trial run. It lives here rather than in the GUI because the case it
    /// exists for is the one where the GUI cannot help: a strategy that takes the network down
    /// with it, or a client that crashed while the rules were up.
    revert_at: Option<Instant>,
}

/// Long enough to open a browser and look at a site, short enough that walking away from a broken
/// network fixes it. `BLKBSTR_REVERT_SECONDS` shortens it, the same way the `BLKBSTR_*` path
/// overrides let the daemon be exercised without being installed.
fn revert_after() -> Duration {
    std::env::var("BLKBSTR_REVERT_SECONDS")
        .ok()
        .and_then(|s| s.parse().ok())
        .map_or(Duration::from_secs(120), Duration::from_secs)
}

pub struct Engine {
    platform: Platform,
    binary: PathBuf,
    version: Option<String>,
    child: Option<Child>,
    firewall: Firewall,
    active: Option<Active>,
    supervisor: Supervisor,
    last_error: Option<String>,
}

impl Engine {
    pub fn new(platform: Platform) -> Result<Self> {
        let binary = locate(platform.engine_binary())?;
        let version = read_version(&binary);
        tracing::info!(binary = %binary.display(), version = ?version, "found engine");
        // Rules from a run that was killed rather than stopped outlive the process that made them.
        Firewall::clear_stale();
        Ok(Self {
            platform,
            binary,
            version,
            child: None,
            firewall: Firewall::new(),
            active: None,
            supervisor: Supervisor::new(),
            last_error: None,
        })
    }

    pub fn status(&self) -> EngineStatus {
        EngineStatus {
            running: self.child.is_some(),
            active_config: self.active.as_ref().map(|a| a.config.name.clone()),
            ephemeral: self.active.as_ref().is_some_and(|a| a.ephemeral),
            pid: self.child.as_ref().map(Child::id),
            started_at: self.active.as_ref().and_then(|a| a.started_at),
            engine_version: self.version.clone(),
            last_error: self.last_error.clone(),
            revert_in_seconds: self
                .active
                .as_ref()
                .and_then(|a| a.revert_at)
                .map(|at| at.saturating_duration_since(Instant::now()).as_secs()),
        }
    }

    /// Brings back whatever was running before the machine went down. Failure is logged, never
    /// fatal: a daemon that refuses to start because a saved config went stale is worse than one
    /// that comes up idle and says why.
    pub fn restore(&mut self) {
        kill_orphan();
        let Some(config) = load_saved() else {
            return;
        };
        tracing::info!(config = %config.name, "restoring the config that was active at shutdown");
        if let Err(e) = self.start(&config, false) {
            tracing::error!(error = %format!("{e:#}"), "could not restore the saved config");
            self.last_error = Some(format!("could not restore {}: {e:#}", config.name));
        }
    }

    /// Drives supervision. Called on a timer rather than from a `waitpid` thread, because the
    /// engine is a single child that restarts at most a few times a minute.
    ///
    /// ponytail: polling. Swap for SIGCHLD if the daemon ever supervises more than one process.
    pub fn tick(&mut self) {
        if self
            .active
            .as_ref()
            .and_then(|a| a.revert_at)
            .is_some_and(|at| Instant::now() >= at)
        {
            tracing::warn!("nobody confirmed the trial run; putting the machine back");
            let outcome = self.stop();
            // Set after the stop, which clears it: this is the reason the engine is down and the
            // user needs to see it, having quite possibly been unable to reach anything.
            self.last_error = Some(match outcome {
                Ok(()) => "the trial run was not confirmed, so it was reverted".into(),
                Err(e) => format!("reverting the unconfirmed trial run failed: {e:#}"),
            });
            return;
        }
        if let Some(at) = self.active.as_ref().and_then(|a| a.restart_at) {
            if Instant::now() >= at {
                self.restart();
            }
            return;
        }

        let Some(child) = self.child.as_mut() else {
            return;
        };
        let exit = match child.try_wait() {
            Ok(Some(exit)) => exit,
            Ok(None) => return,
            Err(e) => {
                tracing::warn!(error = %e, "could not check on the engine");
                return;
            }
        };

        tracing::error!(%exit, "engine exited on its own");
        self.child = None;
        self.last_error = Some(format!("engine exited: {exit}"));

        // Rules must not outlive the process reading the queue, not even for the seconds between
        // a crash and a restart.
        if let Err(e) = self.firewall.teardown() {
            tracing::error!(error = %e, "could not remove rules after the engine died");
        }

        // An ephemeral run is an experiment; if it dies, it has answered the question.
        if self.active.as_ref().is_some_and(|a| a.ephemeral) {
            tracing::info!("ephemeral run died; not restarting it");
            self.active = None;
            return;
        }

        match self.supervisor.on_exit(Instant::now()) {
            Decision::RestartAfter(delay) => {
                tracing::warn!(?delay, "restarting the engine");
                if let Some(active) = self.active.as_mut() {
                    active.restart_at = Some(Instant::now() + delay);
                }
            }
            Decision::GiveUp { restarts, window } => {
                tracing::error!(restarts, ?window, "engine keeps dying; leaving it down");
                self.last_error = Some(format!(
                    "engine exited {restarts} times in {}s and was left down; last exit: {exit}",
                    window.as_secs()
                ));
                self.active = None;
            }
        }
    }

    fn restart(&mut self) {
        let Some(active) = self.active.as_mut() else {
            return;
        };
        active.restart_at = None;
        let config = active.config.clone();
        let ephemeral = active.ephemeral;
        if let Err(e) = self.spawn(&config, ephemeral) {
            tracing::error!(error = %format!("{e:#}"), "restart failed");
            self.last_error = Some(format!("{e:#}"));
        }
    }

    pub fn start(&mut self, config: &Config, ephemeral: bool) -> Result<()> {
        config.validate()?;
        if self.child.is_some() {
            self.stop().context("restarting")?;
        }
        // A deliberate start is a fresh slate: the user may well be fixing what crashed.
        self.supervisor.reset();
        self.spawn(config, ephemeral)?;

        // Persisted only after a successful start, so a config that cannot run is never the one
        // the machine tries to bring up at boot. Ephemeral runs are never persisted: not
        // surviving a restart is the whole point of them.
        if !ephemeral {
            if let Err(e) = save_active(config) {
                tracing::warn!(error = %e, "could not save the active config; it will not return on boot");
            }
        }
        Ok(())
    }

    fn spawn(&mut self, config: &Config, ephemeral: bool) -> Result<()> {
        let runtime = paths::runtime_dir();
        std::fs::create_dir_all(&runtime)
            .with_context(|| format!("creating {}", runtime.display()))?;

        let params_path = runtime.join("nfqws2.conf").display().to_string();
        let mut options = EngineOptions {
            platform: self.platform,
            queue_num: 200,
            pidfile: runtime.join("engine.pid").display().to_string(),
            debug_log: Some(format!("{}/engine.log", paths::log_dir().display())),
            lua_init: lua_init()?,
            validate: false,
        };

        options.validate = true;
        let check_path = runtime.join("nfqws2-check.conf").display().to_string();
        std::fs::write(&check_path, render::parameter_file(config, &options))
            .with_context(|| format!("writing {check_path}"))?;
        self.validate(&check_path)?;

        options.validate = false;
        std::fs::write(&params_path, render::parameter_file(config, &options))
            .with_context(|| format!("writing {params_path}"))?;

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
        self.last_error = None;
        self.active = Some(Active {
            config: config.clone(),
            ephemeral,
            started_at: now(),
            restart_at: None,
            revert_at: ephemeral.then(|| Instant::now() + revert_after()),
        });
        tracing::info!(config = %config.name, pid, ephemeral, "engine started");
        Ok(())
    }

    /// Keeps a trial run: cancels the revert and persists the config, so it comes back at boot.
    pub fn confirm(&mut self) -> Result<()> {
        let Some(active) = self.active.as_mut() else {
            bail!("nothing is running, so there is nothing to keep");
        };
        active.ephemeral = false;
        active.revert_at = None;
        let config = active.config.clone();
        save_active(&config).context("saving the confirmed config")?;
        tracing::info!(config = %config.name, "trial run kept");
        Ok(())
    }

    /// Runs the config with `--intercept=0`: options are checked, the Lua is loaded and every
    /// action is resolved, then the engine exits without opening NFQUEUE. `--dry-run` is the
    /// weaker check — it returns 0 for an action no Lua defines — so this is used instead.
    fn validate(&self, params_path: &str) -> Result<()> {
        let out = Command::new(&self.binary)
            .args(render::run_args(params_path))
            .output()
            .with_context(|| format!("checking the config with {}", self.binary.display()))?;
        if !out.status.success() {
            bail!(
                "the engine rejected this config: {}",
                String::from_utf8_lossy(&out.stderr).trim()
            );
        }
        Ok(())
    }

    pub fn stop(&mut self) -> Result<()> {
        // Rules first: with them gone, traffic flows normally even if killing the engine drags.
        let rules = self.firewall.teardown();

        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
            tracing::info!("engine stopped");
        }
        self.active = None;
        self.last_error = None;
        self.supervisor.reset();
        // Stopping is a decision to stay stopped, including across a reboot.
        if let Err(e) = clear_saved() {
            tracing::warn!(error = %e, "could not clear the saved config");
        }
        rules
    }

    /// Kills the engine and removes the rules on the way out of the process. Unlike [`stop`] it
    /// leaves the saved config alone — this is the machine going down, not a decision to stay
    /// stopped — and it never fails, because there is nobody left to report a failure to.
    ///
    /// Without this the engine outlives a killed daemon, keeps NFQUEUE bound, and every later
    /// start dies with `nfq_create_queue(): Operation not permitted`.
    pub fn shutdown(&mut self) {
        let _ = self.firewall.teardown();
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        self.active = None;
    }
}

/// Kills an engine left behind by a daemon that died without tearing it down. It still holds
/// NFQUEUE, so without this every start fails with `nfq_create_queue(): Operation not permitted`
/// until someone finds the process by hand.
///
/// Found by scanning `/proc` for our own parameter file rather than by reading the pidfile:
/// nfqws2 only writes that under `--daemon`, and the daemon supervises the engine as a child
/// instead, so the file is truncated to zero bytes and never filled in.
#[cfg(target_os = "linux")]
fn kill_orphan() {
    let params = format!("@{}", paths::runtime_dir().join("nfqws2.conf").display());
    let Ok(entries) = std::fs::read_dir("/proc") else {
        return;
    };
    for entry in entries.flatten() {
        let Ok(pid) = entry.file_name().to_string_lossy().parse::<u32>() else {
            continue;
        };
        let Ok(cmdline) = std::fs::read(entry.path().join("cmdline")) else {
            continue;
        };
        let args: Vec<_> = cmdline.split(|b| *b == 0).collect();
        if args.contains(&params.as_bytes()) {
            tracing::warn!(pid, "killing an engine left behind by a previous daemon");
            let _ = Command::new("kill")
                .arg("-TERM")
                .arg(pid.to_string())
                .status();
        }
    }
}

#[cfg(not(target_os = "linux"))]
fn kill_orphan() {}

fn saved_path() -> PathBuf {
    paths::state_dir().join("active.json")
}

fn save_active(config: &Config) -> Result<()> {
    let dir = paths::state_dir();
    std::fs::create_dir_all(&dir).with_context(|| format!("creating {}", dir.display()))?;
    std::fs::write(saved_path(), config.to_json()?).context("writing the active config")
}

fn clear_saved() -> Result<()> {
    match std::fs::remove_file(saved_path()) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e.into()),
    }
}

/// `None` when there is nothing to restore, or when what is there is unusable. Re-validated on the
/// way in because this file decides what a root process runs.
fn load_saved() -> Option<Config> {
    let path = saved_path();
    let text = std::fs::read_to_string(&path).ok()?;
    match parse_saved(&text) {
        Ok(config) => Some(config),
        Err(e) => {
            tracing::error!(path = %path.display(), error = %e, "ignoring an unusable saved config");
            None
        }
    }
}

fn parse_saved(text: &str) -> Result<Config, String> {
    let config = Config::from_json(text).map_err(|e| e.to_string())?;
    config.validate().map_err(|e| e.to_string())?;
    Ok(config)
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

fn lua_init() -> Result<Vec<String>> {
    let dir = detect::locate_lua_dir().context(
        "zapret2's Lua scripts were not found. They ship with the engine — look for \
         zapret-lib.lua under /opt/zapret2/lua — and without them no desync action exists",
    )?;
    Ok(detect::lua_init_scripts(&dir))
}

fn locate(binary: &str) -> Result<PathBuf> {
    detect::locate_engine(binary).with_context(|| {
        format!(
            "{binary} not found. Install zapret2 (https://github.com/bol-van/zapret2) or place \
             the binary in /opt/zapret2"
        )
    })
}

fn read_version(binary: &Path) -> Option<String> {
    let out = Command::new(binary).arg("--version").output().ok()?;
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .next()
        .map(|l| l.trim().to_owned())
        .filter(|l| !l.is_empty())
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

    #[test]
    fn a_saved_config_is_revalidated_on_the_way_back_in() {
        let good = render::starter_config("saved");
        assert_eq!(parse_saved(&good.to_json().unwrap()).unwrap().name, "saved");

        // Anything that would be refused from the socket is refused from disk too — the file is
        // root-owned, but a bug that writes a bad one must not become a bug that runs it.
        assert!(parse_saved(r#"{"name":"../escape"}"#).is_err());
        assert!(parse_saved("not json").is_err());
        assert!(parse_saved(
            r#"{"name":"x","strategies":[{"name":"s","filter":{"tcp":"443\n--qnum=9"}}]}"#
        )
        .is_err());
    }
}
