//! The two host lists the engine reads: the learned one behind `--hostlist-auto`, and the user's
//! own behind `--hostlist`. The daemon owns both files. The engine appends to the learned list
//! when it sees a host fail; nothing removes a host again unless we do, which is what the periodic
//! check of the learned list is for.

use anyhow::{Context, Result};
use blkbstr_core::paths;
use blkbstr_core::protocol::{HostList, HostsStatus};
use blkbstr_core::reachability::{self, check_hostname};
use std::path::Path;
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// nfqws2 drops root once it is running and appends to the list as this user, so the file must be
/// theirs. It is the engine's default (`--user`/`--uid` unset), not a value we choose.
#[cfg(unix)]
const ENGINE_UID: u32 = 0x7FFF_FFFF;

/// Hosts probed at once: each is a thread and a connection.
const ROUND: usize = 16;

/// A probe has to skip the engine without anything being switched off: Linux marks the socket and
/// the firewall skips the mark, Windows sets a TTL that the engine's filter skips. Elsewhere the
/// only way to see a host without the strategy is to take it off the list, which breaks the
/// user's own connections to it while the check runs, so no check runs.
pub const CAN_RECHECK: bool = cfg!(any(target_os = "linux", windows));

/// Long enough for a block to have been lifted, short enough that a host nobody needs any more
/// does not linger for days. `BLKBSTR_AUTOHOSTS_INTERVAL_SECONDS` overrides it.
fn interval() -> Duration {
    std::env::var("BLKBSTR_AUTOHOSTS_INTERVAL_SECONDS")
        .ok()
        .and_then(|s| s.parse().ok())
        .map_or(Duration::from_secs(6 * 3600), Duration::from_secs)
}

/// The first check waits a while, so starting the engine does not open with an outage.
const FIRST_CHECK_AFTER: Duration = Duration::from_secs(600);

struct State {
    checking: bool,
    last_check: Option<u64>,
    last_dropped: Vec<String>,
    next_check: Option<u64>,
}

static STATE: Mutex<State> = Mutex::new(State {
    checking: false,
    last_check: None,
    last_dropped: Vec::new(),
    next_check: None,
});

/// Held across each read-modify-write of the file, never across a probe.
static FILE: Mutex<()> = Mutex::new(());

fn state() -> std::sync::MutexGuard<'static, State> {
    STATE.lock().unwrap_or_else(|e| e.into_inner())
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

pub fn path_for(list: HostList) -> std::path::PathBuf {
    match list {
        HostList::Auto => paths::auto_hostlist(),
        HostList::Manual => paths::manual_hostlist(),
    }
}

pub fn status(list: HostList, enabled: bool) -> HostsStatus {
    let s = state();
    let learned = list == HostList::Auto;
    HostsStatus {
        list,
        enabled,
        can_recheck: CAN_RECHECK,
        hosts: read(&path_for(list)),
        checking: learned && s.checking,
        last_check: s.last_check.filter(|_| learned),
        last_dropped: if learned {
            s.last_dropped.clone()
        } else {
            Vec::new()
        },
        next_check: s.next_check.filter(|_| learned && enabled && CAN_RECHECK),
    }
}

/// Called every few seconds by the monitor. Runs the check when it is due and the list is in use.
pub fn tick(in_use: bool) {
    if !CAN_RECHECK {
        return;
    }
    let due = {
        let mut s = state();
        if !in_use {
            s.next_check = None;
            return;
        }
        let at = *s
            .next_check
            .get_or_insert(now() + FIRST_CHECK_AFTER.as_secs());
        now() >= at
    };
    if due {
        run();
    }
}

/// For a request from the GUI: returns at once, the check takes as long as the slowest probe.
pub fn check_in_background() {
    if CAN_RECHECK {
        std::thread::spawn(run);
    }
}

fn run() {
    {
        let mut s = state();
        if s.checking {
            return;
        }
        s.checking = true;
    }
    let dropped = can_mark()
        .and_then(|()| recheck(&paths::auto_hostlist(), probe))
        .unwrap_or_else(|e| {
            tracing::warn!(error = %format!("{e:#}"), "could not re-check the learned hosts");
            Vec::new()
        });
    if !dropped.is_empty() {
        tracing::info!(?dropped, "learned hosts that no longer need a strategy");
    }
    let mut s = state();
    s.checking = false;
    s.last_check = Some(now());
    s.last_dropped = dropped;
    s.next_check = Some(now() + interval().as_secs());
}

/// The hosts that are still blocked, or `None` when even the control host failed: with the
/// network down every host looks blocked, and dropping none is the safe reading. A flaky network
/// can only make a host look blocked, never reachable, so it costs a re-check and nothing else.
#[cfg(any(target_os = "linux", windows))]
fn probe(hosts: &[String]) -> Option<Vec<String>> {
    let report =
        reachability::check_with(hosts, reachability::DEFAULT_TIMEOUT, &connect_unprotected);
    report.network_ok.then(|| {
        report
            .sites
            .into_iter()
            .filter(|site| site.verdict.blocked())
            .map(|site| site.host)
            .collect()
    })
}

#[cfg(not(any(target_os = "linux", windows)))]
fn probe(_hosts: &[String]) -> Option<Vec<String>> {
    None
}

/// A connection the engine does not see, so the host is seen as it would be with no strategy while
/// it stays on its list and the user's own traffic to it carries on untouched.
#[cfg(any(target_os = "linux", windows))]
fn connect_unprotected(
    address: &std::net::SocketAddr,
    timeout: Duration,
) -> std::io::Result<std::net::TcpStream> {
    use socket2::{Domain, Socket, Type};
    let socket = Socket::new(Domain::for_address(*address), Type::STREAM, None)?;
    mark_probe(&socket, address)?;
    socket.connect_timeout(&(*address).into(), timeout)?;
    Ok(socket.into())
}

/// Before the connect, or the SYN leaves unmarked.
#[cfg(target_os = "linux")]
fn mark_probe(socket: &socket2::Socket, _address: &std::net::SocketAddr) -> std::io::Result<()> {
    socket.set_mark(crate::firewall::PROBE_MARK)
}

#[cfg(windows)]
fn mark_probe(socket: &socket2::Socket, address: &std::net::SocketAddr) -> std::io::Result<()> {
    let ttl = u32::from(blkbstr_core::render::PROBE_TTL);
    if address.is_ipv4() {
        socket.set_ttl_v4(ttl)
    } else {
        socket.set_unicast_hops_v6(ttl)
    }
}

/// Marking needs CAP_NET_ADMIN. Without it every probe would fail and read as blocked, so the
/// check is refused instead.
#[cfg(target_os = "linux")]
fn can_mark() -> Result<()> {
    let socket = socket2::Socket::new(socket2::Domain::IPV4, socket2::Type::STREAM, None)?;
    socket
        .set_mark(crate::firewall::PROBE_MARK)
        .context("marking probe packets needs CAP_NET_ADMIN")
}

#[cfg(windows)]
fn can_mark() -> Result<()> {
    Ok(())
}

#[cfg(not(any(target_os = "linux", windows)))]
fn can_mark() -> Result<()> {
    anyhow::bail!("this platform cannot re-check without disturbing traffic")
}

/// Returns the hosts that were dropped: the ones that now answer. The check goes out marked, so it
/// needs no list change and no pause in the user's own traffic.
fn recheck(path: &Path, probe: impl Fn(&[String]) -> Option<Vec<String>>) -> Result<Vec<String>> {
    let mut dropped = Vec::new();
    for round in read(path).chunks(ROUND) {
        // Network down: nothing learned, nothing dropped.
        let Some(blocked) = probe(round) else { break };
        let reachable: Vec<String> = round
            .iter()
            .filter(|h| !blocked.contains(h))
            .cloned()
            .collect();
        remove(path, &reachable)?;
        dropped.extend(reachable);
    }
    Ok(dropped)
}

/// Plain hostnames only: the file is read by the engine as root, one host per line, and the hosts
/// arrive over the socket.
fn valid(hosts: &[String]) -> Vec<String> {
    hosts
        .iter()
        .map(|h| h.trim().to_ascii_lowercase())
        .filter(|h| check_hostname(h).is_ok())
        .collect()
}

/// Returns how many were new.
pub fn add_hosts(path: &Path, hosts: &[String]) -> Result<usize> {
    ensure(path)?;
    add(path, &valid(hosts))
}

pub fn remove_hosts(path: &Path, hosts: &[String]) -> Result<()> {
    ensure(path)?;
    remove(path, &valid(hosts))
}

fn add(path: &Path, hosts: &[String]) -> Result<usize> {
    let _guard = FILE.lock().unwrap_or_else(|e| e.into_inner());
    let mut all = read(path);
    let before = all.len();
    for host in hosts {
        if !all.contains(host) {
            all.push(host.clone());
        }
    }
    if all.len() != before {
        write(path, &all)?;
    }
    Ok(all.len() - before)
}

fn remove(path: &Path, hosts: &[String]) -> Result<()> {
    let _guard = FILE.lock().unwrap_or_else(|e| e.into_inner());
    let mut all = read(path);
    all.retain(|h| !hosts.contains(h));
    write(path, &all)
}

/// Hosts one per line; blank lines and `#` comments are skipped, as the engine skips them.
pub fn read(path: &Path) -> Vec<String> {
    let mut hosts: Vec<String> = Vec::new();
    for line in std::fs::read_to_string(path).unwrap_or_default().lines() {
        let line = line.trim();
        if !line.is_empty() && !line.starts_with('#') && !hosts.iter().any(|h| h == line) {
            hosts.push(line.to_owned());
        }
    }
    hosts
}

/// Replaced by rename, so the engine never reads a half-written list.
fn write(path: &Path, hosts: &[String]) -> Result<()> {
    let tmp = path.with_extension("tmp");
    let mut text = hosts.join("\n");
    if !text.is_empty() {
        text.push('\n');
    }
    std::fs::write(&tmp, text).with_context(|| format!("writing {}", tmp.display()))?;
    give_to_engine(&tmp);
    std::fs::rename(&tmp, path).with_context(|| format!("replacing {}", path.display()))
}

/// Makes sure the file exists and the engine can append to it. Called before every start, since
/// `--intercept=0` refuses a `--hostlist-auto` file it cannot find.
pub fn ensure(path: &Path) -> Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
    }
    let _guard = FILE.lock().unwrap_or_else(|e| e.into_inner());
    if !path.exists() {
        write(path, &[])?;
    } else {
        give_to_engine(path);
    }
    Ok(())
}

#[cfg(unix)]
fn give_to_engine(path: &Path) {
    // Fails without root, which is a development run: the engine is then not dropping privileges either.
    if let Err(e) = std::os::unix::fs::chown(path, Some(ENGINE_UID), Some(ENGINE_UID)) {
        tracing::debug!(path = %path.display(), error = %e, "could not hand the list to the engine's user");
    }
}

#[cfg(not(unix))]
fn give_to_engine(_path: &Path) {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn list(contents: &str) -> std::path::PathBuf {
        static N: AtomicUsize = AtomicUsize::new(0);
        let dir = std::env::temp_dir().join(format!(
            "blkbstr-autolist-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("autohostlist.txt");
        std::fs::write(&path, contents).unwrap();
        path
    }

    fn strings(hosts: &[&str]) -> Vec<String> {
        hosts.iter().map(|h| (*h).to_owned()).collect()
    }

    #[test]
    fn reads_what_the_engine_reads() {
        let path = list("# learned\nexample.org\n\n  discord.com  \nexample.org\n");
        assert_eq!(read(&path), ["example.org", "discord.com"]);
        assert!(read(&path.with_file_name("missing.txt")).is_empty());
    }

    #[test]
    fn hosts_that_could_inject_a_line_are_not_added() {
        let path = list("");
        let added = add_hosts(
            &path,
            &strings(&["Example.ORG", "a.com\nb.com", "bad host", "", "ok-2.net"]),
        )
        .unwrap();
        assert_eq!(added, 2);
        assert_eq!(read(&path), ["example.org", "ok-2.net"]);
        assert_eq!(add_hosts(&path, &strings(&["example.org"])).unwrap(), 0);
    }

    #[test]
    fn removing_ignores_case_and_hosts_that_are_not_there() {
        let path = list("keep.example\ndrop.example\n");
        remove_hosts(
            &path,
            &strings(&["  DROP.example ", "never-listed.example", "bad host"]),
        )
        .unwrap();
        assert_eq!(read(&path), ["keep.example"]);
    }

    #[test]
    fn a_recheck_drops_only_the_hosts_that_answer_and_never_touches_the_rest_first() {
        let path = list("blocked.example\nlifted.example\n");
        let dropped = recheck(&path, |round| {
            // The user's traffic keeps its strategy while the probe runs: nothing leaves the list.
            assert_eq!(read(&path), ["blocked.example", "lifted.example"]);
            Some(
                round
                    .iter()
                    .filter(|h| *h == "blocked.example")
                    .cloned()
                    .collect(),
            )
        })
        .unwrap();
        assert_eq!(dropped, ["lifted.example"]);
        assert_eq!(read(&path), ["blocked.example"]);
    }

    #[test]
    fn a_down_network_drops_nothing() {
        let path = list("a.example\nb.example\n");
        assert!(recheck(&path, |_| None).unwrap().is_empty());
        assert_eq!(read(&path), ["a.example", "b.example"]);
    }

    #[test]
    fn hosts_learned_during_a_round_survive_it() {
        let path = list("a.example\nb.example\n");
        recheck(&path, |_| {
            std::fs::write(&path, "a.example\nb.example\nnew.example\n").unwrap();
            Some(strings(&["a.example"]))
        })
        .unwrap();
        assert_eq!(read(&path), ["a.example", "new.example"]);
    }

    #[test]
    fn a_long_list_is_probed_in_rounds() {
        let hosts: Vec<String> = (0..ROUND * 2 + 1)
            .map(|n| format!("h{n}.example"))
            .collect();
        let path = list(&hosts.join("\n"));
        let sizes = std::cell::RefCell::new(Vec::new());
        recheck(&path, |round| {
            sizes.borrow_mut().push(round.len());
            Some(round.to_vec())
        })
        .unwrap();
        assert_eq!(*sizes.borrow(), [ROUND, ROUND, 1]);
        assert_eq!(read(&path).len(), hosts.len());
    }

    #[test]
    #[cfg(any(target_os = "linux", windows))]
    fn the_check_waits_for_the_list_to_be_in_use() {
        tick(false);
        assert_eq!(state().next_check, None);
        tick(true);
        assert!(state().next_check.is_some_and(|at| at > now()));
        tick(false);
        assert_eq!(state().next_check, None);
    }
}
