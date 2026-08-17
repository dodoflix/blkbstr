//! Linux packet interception: an nftables table that feeds traffic to the engine's NFQUEUE.
//!
//! Everything lives in one dedicated table, so teardown is `nft delete table inet blkbstr` — one
//! atomic operation that cannot touch another program's rules. This is the whole reason for the
//! separate table, and the reason iptables is not supported: there, rules from every program share
//! the same chains, and removing ours means matching and deleting individual rules. Upstream's own
//! guidance is to avoid iptables on any modern distribution.
//!
//! ponytail: nftables only. Add an iptables path if someone turns up on a kernel older than 5.15.

use anyhow::{bail, Context, Result};
use std::io::Write;
use std::process::{Command, Stdio};

/// Table name; also the handle used to tear everything down.
const TABLE: &str = "blkbstr";

/// Bit set on packets the engine emits, so the rules can skip them. Without it, generated packets
/// re-enter the queue and the machine locks up. Matches nfqws2's `--fwmark` default.
const FWMARK: &str = "0x40000000";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InterceptSpec {
    /// Interface traffic leaves by. Interception is scoped to it so loopback and LAN traffic are
    /// not queued into userspace for nothing.
    pub wan_iface: String,
    /// Comma-separated, already validated, e.g. `80,443`.
    pub tcp_ports: String,
    pub udp_ports: String,
    pub queue_num: u16,
    /// How many packets from the start of a flow to intercept. Beyond this the engine has already
    /// made its decision and queuing only costs CPU.
    pub max_pkt_out: u16,
    pub max_pkt_in: u16,
}

impl InterceptSpec {
    pub fn new(wan_iface: impl Into<String>, queue_num: u16) -> Self {
        Self {
            wan_iface: wan_iface.into(),
            tcp_ports: "80,443".into(),
            udp_ports: "443".into(),
            queue_num,
            max_pkt_out: 15,
            max_pkt_in: 15,
        }
    }
}

/// The nftables script, as fed to `nft -f -`. Pure, so the exact ruleset can be tested and shown
/// to the user before anything is applied.
///
/// Follows the POSTNAT scheme from the zapret2 manual: outgoing traffic is intercepted after NAT
/// so packets already carry their final source address, and `notrack` on the engine's own packets
/// keeps NAT from dropping techniques that deliberately violate its expectations.
pub fn ruleset(spec: &InterceptSpec) -> String {
    let InterceptSpec {
        wan_iface: wan,
        tcp_ports: tcp,
        udp_ports: udp,
        queue_num: q,
        max_pkt_out: out,
        max_pkt_in: inp,
    } = spec;

    let mut s = String::new();
    // Idempotent: `add` creates the table if absent, `delete` then guarantees a clean slate even
    // if a previous run died without tearing down.
    s.push_str(&format!("add table inet {TABLE}\n"));
    s.push_str(&format!("delete table inet {TABLE}\n"));
    s.push_str(&format!("add table inet {TABLE}\n"));

    s.push_str(&format!(
        "add chain inet {TABLE} postnat {{ type filter hook postrouting priority srcnat + 1; }}\n"
    ));
    if !udp.is_empty() {
        s.push_str(&format!(
            "add rule inet {TABLE} postnat oifname \"{wan}\" meta mark and {FWMARK} == 0 \
             udp dport {{ {udp} }} ct original packets 1-{out} queue num {q} bypass\n"
        ));
    }
    if !tcp.is_empty() {
        s.push_str(&format!(
            "add rule inet {TABLE} postnat oifname \"{wan}\" meta mark and {FWMARK} == 0 \
             tcp dport {{ {tcp} }} ct original packets 1-{out} queue num {q} bypass\n"
        ));
        // FIN/RST are cheap and let the engine's conntrack retire flows promptly.
        s.push_str(&format!(
            "add rule inet {TABLE} postnat oifname \"{wan}\" meta mark and {FWMARK} == 0 \
             tcp dport {{ {tcp} }} tcp flags fin,rst queue num {q} bypass\n"
        ));
    }

    s.push_str(&format!(
        "add chain inet {TABLE} pre {{ type filter hook prerouting priority filter; }}\n"
    ));
    if !udp.is_empty() {
        s.push_str(&format!(
            "add rule inet {TABLE} pre iifname \"{wan}\" udp sport {{ {udp} }} \
             ct reply packets 1-{inp} queue num {q} bypass\n"
        ));
    }
    if !tcp.is_empty() {
        s.push_str(&format!(
            "add rule inet {TABLE} pre iifname \"{wan}\" tcp sport {{ {tcp} }} \
             ct reply packets 1-{inp} queue num {q} bypass\n"
        ));
        // Unquoted: the quotes in upstream's example are shell quoting. `nft -f -` reads this
        // directly, and a quoted expression is parsed as a literal string, failing on the next
        // token with "syntax error, unexpected queue".
        s.push_str(&format!(
            "add rule inet {TABLE} pre iifname \"{wan}\" tcp sport {{ {tcp} }} \
             tcp flags & (syn | ack) == (syn | ack) queue num {q} bypass\n"
        ));
        s.push_str(&format!(
            "add rule inet {TABLE} pre iifname \"{wan}\" tcp sport {{ {tcp} }} \
             tcp flags fin,rst queue num {q} bypass\n"
        ));
    }

    // Packets the engine injects must skip conntrack entirely, or NAT rejects the ones whose
    // sequence numbers deliberately do not add up.
    s.push_str(&format!(
        "add chain inet {TABLE} predefrag {{ type filter hook output priority -401; }}\n"
    ));
    s.push_str(&format!(
        "add rule inet {TABLE} predefrag mark and {FWMARK} != 0x00000000 notrack\n"
    ));
    s
}

/// Owns the applied ruleset. Dropping it removes the table, so a panic or an early return cannot
/// leave the machine with rules pointing at a queue nothing is reading — which is the failure that
/// silently breaks a user's network.
#[derive(Debug, Default)]
pub struct Firewall {
    applied: bool,
}

impl Firewall {
    pub fn new() -> Self {
        Self::default()
    }

    /// Removes a table left behind by a previous run. Called at startup, because `Drop` cannot run
    /// after SIGKILL or a power loss.
    pub fn clear_stale() {
        if delete_table().is_ok() {
            tracing::info!("removed a stale nftables table from a previous run");
        }
    }

    pub fn apply(&mut self, spec: &InterceptSpec) -> Result<()> {
        let script = ruleset(spec);
        tracing::debug!(%script, "applying nftables ruleset");

        let mut child = Command::new("nft")
            .arg("-f")
            .arg("-")
            .stdin(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .context("running nft — is nftables installed?")?;
        child
            .stdin
            .take()
            .context("nft stdin")?
            .write_all(script.as_bytes())?;
        let out = child.wait_with_output()?;
        if !out.status.success() {
            bail!(
                "nft rejected the ruleset: {}",
                String::from_utf8_lossy(&out.stderr).trim()
            );
        }

        self.applied = true;
        tracing::info!(iface = %spec.wan_iface, queue = spec.queue_num, "interception active");
        Ok(())
    }

    pub fn teardown(&mut self) -> Result<()> {
        if !self.applied {
            return Ok(());
        }
        delete_table()?;
        self.applied = false;
        tracing::info!("interception removed");
        Ok(())
    }
}

impl Drop for Firewall {
    fn drop(&mut self) {
        if let Err(e) = self.teardown() {
            tracing::error!(error = %e, "could not remove the nftables table; run `nft delete table inet {TABLE}`");
        }
    }
}

fn delete_table() -> Result<()> {
    let out = Command::new("nft")
        .args(["delete", "table", "inet", TABLE])
        .output()
        .context("running nft")?;
    if out.status.success() {
        Ok(())
    } else {
        bail!("{}", String::from_utf8_lossy(&out.stderr).trim().to_owned())
    }
}

/// Interface of the default route — the way out of this machine.
pub fn default_route_iface() -> Result<String> {
    let out = Command::new("ip")
        .args(["route", "show", "default"])
        .output()
        .context("running ip route")?;
    if !out.status.success() {
        bail!(
            "ip route failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    parse_default_iface(&String::from_utf8_lossy(&out.stdout))
        .context("no default route; is this machine online?")
}

fn parse_default_iface(routes: &str) -> Option<String> {
    // "default via 192.168.1.1 dev wlan0 proto dhcp metric 600"
    routes
        .lines()
        .find(|l| l.starts_with("default"))?
        .split_whitespace()
        .skip_while(|w| *w != "dev")
        .nth(1)
        .map(str::to_owned)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ruleset_starts_from_a_clean_table() {
        let s = ruleset(&InterceptSpec::new("wlan0", 200));
        let lines: Vec<_> = s.lines().take(3).collect();
        assert_eq!(
            lines,
            [
                "add table inet blkbstr",
                "delete table inet blkbstr",
                "add table inet blkbstr"
            ]
        );
    }

    #[test]
    fn every_queueing_rule_skips_the_engines_own_packets() {
        let s = ruleset(&InterceptSpec::new("eth0", 200));
        for line in s
            .lines()
            .filter(|l| l.contains("postnat") && l.contains("queue num"))
        {
            assert!(
                line.contains(&format!("meta mark and {FWMARK} == 0")),
                "outgoing rule without a fwmark guard would loop: {line}"
            );
        }
        assert!(s.contains("notrack"), "engine packets must skip conntrack");
    }

    #[test]
    fn omitting_a_protocol_omits_its_rules() {
        let mut spec = InterceptSpec::new("eth0", 200);
        spec.udp_ports = String::new();
        let s = ruleset(&spec);
        assert!(!s.contains("udp dport"), "{s}");
        assert!(s.contains("tcp dport"), "{s}");
    }

    #[test]
    fn queue_number_reaches_every_rule() {
        let s = ruleset(&InterceptSpec::new("eth0", 137));
        // 3 outgoing (udp, tcp, tcp fin/rst) + 4 incoming (udp, tcp, synack, tcp fin/rst).
        let queueing = s.lines().filter(|l| l.contains("queue num")).count();
        assert_eq!(queueing, 7);
        assert_eq!(s.matches("queue num 137 bypass").count(), 7);
    }

    #[test]
    fn match_expressions_are_not_quoted() {
        // Only interface names are quoted. A quoted match expression is read by nft as a literal.
        let s = ruleset(&InterceptSpec::new("eth0", 200));
        for line in s.lines() {
            let quoted: Vec<_> = line.match_indices('"').collect();
            assert!(
                quoted.len() % 2 == 0 && quoted.len() <= 2,
                "only the interface name may be quoted: {line}"
            );
        }
        assert!(s.contains("tcp flags & (syn | ack) == (syn | ack) queue num 200 bypass"));
    }

    #[test]
    fn finds_the_default_route_interface() {
        let routes = "default via 192.168.1.1 dev wlan0 proto dhcp src 192.168.1.23 metric 600\n\
                      10.0.0.0/8 dev tun0 scope link";
        assert_eq!(parse_default_iface(routes).as_deref(), Some("wlan0"));
        assert_eq!(parse_default_iface("10.0.0.0/8 dev tun0"), None);
    }
}
