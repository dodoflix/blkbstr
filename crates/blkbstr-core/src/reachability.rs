//! Whether a site is reachable, and if not, what stopped it. The check step of onboarding and the
//! oracle auto-configuration measures against. Done over plain TCP with no TLS library: any TLS
//! record coming back once a hostname is named means the far end answered, and a TLS stack that
//! retried or reused a session could turn a blocked site into a working one.

use serde::Serialize;
use std::io::{ErrorKind, Read, Write};
use std::net::{IpAddr, SocketAddr, TcpStream, ToSocketAddrs};
use std::time::{Duration, Instant};

/// Sites commonly interfered with, plus enough variety that one company's outage is not mistaken
/// for censorship.
pub const DEFAULT_HOSTS: &[&str] = &[
    "www.youtube.com",
    "discord.com",
    "x.com",
    "www.instagram.com",
    "www.facebook.com",
    "rutracker.org",
];

/// Reserved by IANA for documentation, serves TLS, and no censor cares about it. If it fails, the
/// network is broken.
pub const CONTROL_HOST: &str = "example.com";

/// The only port the check speaks on; a candidate filtering another port cannot change the result.
pub const PORT: u16 = 443;
/// Long enough for a slow mobile link, short enough that a silent drop does not hold the check.
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(5);

/// Used while walking candidates. A host a strategy has unblocked answers quickly; the rest of the
/// wait only proves a silent drop is still a silent drop.
pub const TRIAL_TIMEOUT: Duration = Duration::from_secs(2);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Verdict {
    /// The server answered after being told which host we wanted.
    Fine,
    /// The name did not resolve at all.
    DnsFailed,
    /// It resolved, but to an address nothing can be hosted on.
    DnsPoisoned,
    /// The address resolved and the connection could not be opened.
    TcpBlocked,
    /// The connection opened and then died once the hostname was on the wire.
    TlsReset,
    /// The connection opened and nothing ever came back.
    TlsSilent,
    /// The hostname itself is unusable, so nothing was sent.
    BadHost,
}

impl Verdict {
    pub const fn blocked(self) -> bool {
        !matches!(self, Verdict::Fine)
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct SiteResult {
    pub host: String,
    pub verdict: Verdict,
    /// The address tried, or why the verdict is what it is; shown verbatim.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    pub elapsed_ms: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct Report {
    /// False when even the control host failed: the network is down, not filtered, and every other
    /// verdict is meaningless.
    pub network_ok: bool,
    pub control: SiteResult,
    pub sites: Vec<SiteResult>,
}

impl Report {
    pub fn blocked_count(&self) -> usize {
        self.sites.iter().filter(|s| s.verdict.blocked()).count()
    }
}

/// Checks the control host and every site concurrently, so the timeout is not multiplied by the list.
pub fn check(hosts: &[String], timeout: Duration) -> Report {
    std::thread::scope(|scope| {
        let control = scope.spawn(move || check_host(CONTROL_HOST, timeout));
        let running: Vec<_> = hosts
            .iter()
            .map(|host| scope.spawn(move || check_host(host, timeout)))
            .collect();

        let control = control.join().unwrap_or_else(|_| failed(CONTROL_HOST));
        Report {
            network_ok: control.verdict == Verdict::Fine,
            control,
            sites: running
                .into_iter()
                .zip(hosts)
                .map(|(handle, host)| handle.join().unwrap_or_else(|_| failed(host)))
                .collect(),
        }
    })
}

fn failed(host: &str) -> SiteResult {
    SiteResult {
        host: host.to_owned(),
        verdict: Verdict::BadHost,
        detail: Some("the check itself panicked".into()),
        elapsed_ms: 0,
    }
}

pub fn check_host(host: &str, timeout: Duration) -> SiteResult {
    let started = Instant::now();
    let (verdict, detail) = probe(host, timeout);
    SiteResult {
        host: host.to_owned(),
        verdict,
        detail,
        elapsed_ms: started.elapsed().as_millis() as u64,
    }
}

fn probe(host: &str, timeout: Duration) -> (Verdict, Option<String>) {
    if let Err(why) = check_hostname(host) {
        return (Verdict::BadHost, Some(why.into()));
    }

    let addresses: Vec<SocketAddr> = match (host, PORT).to_socket_addrs() {
        Ok(found) => found.collect(),
        Err(e) => return (Verdict::DnsFailed, Some(e.to_string())),
    };
    let Some(address) = addresses.iter().find(|a| !is_sinkhole(a.ip())) else {
        return match addresses.first() {
            Some(a) => (Verdict::DnsPoisoned, Some(a.ip().to_string())),
            None => (Verdict::DnsFailed, Some("no addresses returned".into())),
        };
    };

    probe_address(*address, host, timeout)
}

/// Split out from [`probe`] so verdicts that depend on server behaviour can be tested on a local socket.
fn probe_address(address: SocketAddr, host: &str, timeout: Duration) -> (Verdict, Option<String>) {
    let mut stream = match TcpStream::connect_timeout(&address, timeout) {
        Ok(stream) => stream,
        Err(e) => return (Verdict::TcpBlocked, Some(format!("{}: {e}", address.ip()))),
    };
    let _ = stream.set_read_timeout(Some(timeout));
    if let Err(e) = stream.write_all(&client_hello(host)) {
        return (Verdict::TlsReset, Some(format!("sending hello: {e}")));
    }

    let mut head = [0u8; 8];
    match stream.read(&mut head) {
        Ok(0) => (
            Verdict::TlsReset,
            Some(format!("{}: closed after the hello", address.ip())),
        ),
        // 0x16 is a handshake record, 0x15 an alert; either means the far end parsed the hello.
        Ok(_) if head[0] == 0x16 => (Verdict::Fine, Some(address.ip().to_string())),
        Ok(_) if head[0] == 0x15 => (
            Verdict::Fine,
            Some(format!("{}: answered with a TLS alert", address.ip())),
        ),
        Ok(n) => (
            Verdict::TlsReset,
            Some(format!("{}: {n} bytes, not a TLS record", address.ip())),
        ),
        Err(e) if e.kind() == ErrorKind::WouldBlock || e.kind() == ErrorKind::TimedOut => (
            Verdict::TlsSilent,
            Some(format!("{}: no answer in {timeout:?}", address.ip())),
        ),
        Err(e) => (Verdict::TlsReset, Some(format!("{}: {e}", address.ip()))),
    }
}

/// ASCII and within the DNS length limit: the name reaches a `u16` SNI length and the resolver.
fn check_hostname(host: &str) -> Result<(), &'static str> {
    if host.is_empty() || host.len() > 253 {
        return Err("a hostname is 1 to 253 characters");
    }
    if !host
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || b == b'.' || b == b'-')
    {
        return Err("only letters, digits, dots and hyphens; punycode an international name first");
    }
    Ok(())
}

/// Nothing real is served from these, so a name resolving to one was answered by something else.
fn is_sinkhole(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            v4.is_loopback()
                || v4.is_unspecified()
                || v4.is_private()
                || v4.is_link_local()
                || v4.is_broadcast()
        }
        IpAddr::V6(v6) => v6.is_loopback() || v6.is_unspecified(),
    }
}

/// A TLS 1.2 ClientHello naming `host` in the SNI extension. Hand-built so exactly what DPI
/// inspects is on the wire, with no retry, fallback or session reuse.
fn client_hello(host: &str) -> Vec<u8> {
    let mut extensions = Vec::new();
    // server_name: the extension the whole check exists to send.
    let name = host.as_bytes();
    extend_ext(&mut extensions, 0x0000, |e| {
        e.extend_from_slice(&((name.len() + 3) as u16).to_be_bytes());
        e.push(0); // host_name
        e.extend_from_slice(&(name.len() as u16).to_be_bytes());
        e.extend_from_slice(name);
    });
    // supported_groups: x25519, secp256r1.
    extend_ext(&mut extensions, 0x000a, |e| {
        e.extend_from_slice(&4u16.to_be_bytes());
        e.extend_from_slice(&[0x00, 0x1d, 0x00, 0x17]);
    });
    // ec_point_formats: uncompressed.
    extend_ext(&mut extensions, 0x000b, |e| e.extend_from_slice(&[1, 0]));
    // signature_algorithms: enough that a server has something to pick.
    extend_ext(&mut extensions, 0x000d, |e| {
        e.extend_from_slice(&6u16.to_be_bytes());
        e.extend_from_slice(&[0x04, 0x01, 0x04, 0x03, 0x08, 0x04]);
    });

    let mut body = Vec::new();
    body.extend_from_slice(&[0x03, 0x03]); // legacy_version: TLS 1.2
    body.extend_from_slice(&random_32()); // random
    body.push(0); // legacy_session_id: empty
    body.extend_from_slice(&8u16.to_be_bytes()); // cipher_suites
    body.extend_from_slice(&[0x13, 0x01, 0x13, 0x02, 0xc0, 0x2b, 0xc0, 0x2f]);
    body.extend_from_slice(&[1, 0]); // compression_methods: null only
    body.extend_from_slice(&(extensions.len() as u16).to_be_bytes());
    body.extend_from_slice(&extensions);

    let mut handshake = vec![0x01]; // ClientHello
    let len = body.len() as u32;
    handshake.extend_from_slice(&len.to_be_bytes()[1..]); // uint24
    handshake.extend_from_slice(&body);

    let mut record = vec![0x16, 0x03, 0x01]; // handshake, legacy record version
    record.extend_from_slice(&(handshake.len() as u16).to_be_bytes());
    record.extend_from_slice(&handshake);
    record
}

fn extend_ext(out: &mut Vec<u8>, kind: u16, body: impl FnOnce(&mut Vec<u8>)) {
    let mut inner = Vec::new();
    body(&mut inner);
    out.extend_from_slice(&kind.to_be_bytes());
    out.extend_from_slice(&(inner.len() as u16).to_be_bytes());
    out.extend_from_slice(&inner);
}

/// Not a security value: the handshake never completes. It varies so repeated checks do not look
/// like one replayed packet.
fn random_32() -> [u8; 32] {
    use std::collections::hash_map::RandomState;
    use std::hash::{BuildHasher, Hasher};

    let mut out = [0u8; 32];
    for (i, chunk) in out.chunks_mut(8).enumerate() {
        let mut hasher = RandomState::new().build_hasher();
        hasher.write_u64(i as u64);
        chunk.copy_from_slice(&hasher.finish().to_le_bytes());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{Ipv4Addr, Ipv6Addr, TcpListener};

    /// Walks the bytes like a server would, so a wrong length field fails here.
    fn sni_from(hello: &[u8]) -> Option<String> {
        assert_eq!(hello[0], 0x16, "record type");
        let record_len = u16::from_be_bytes([hello[3], hello[4]]) as usize;
        assert_eq!(record_len, hello.len() - 5, "record length");

        let handshake = &hello[5..];
        assert_eq!(handshake[0], 0x01, "ClientHello");
        let body_len = u32::from_be_bytes([0, handshake[1], handshake[2], handshake[3]]) as usize;
        assert_eq!(body_len, handshake.len() - 4, "handshake length");

        // version(2) + random(32) + session_id_len(1)
        let mut at = 4 + 2 + 32;
        at += 1 + handshake[at] as usize;
        at += 2 + u16::from_be_bytes([handshake[at], handshake[at + 1]]) as usize;
        at += 1 + handshake[at] as usize;
        let extensions_len = u16::from_be_bytes([handshake[at], handshake[at + 1]]) as usize;
        at += 2;
        assert_eq!(extensions_len, handshake.len() - at, "extensions length");

        let mut rest = &handshake[at..];
        while rest.len() >= 4 {
            let kind = u16::from_be_bytes([rest[0], rest[1]]);
            let len = u16::from_be_bytes([rest[2], rest[3]]) as usize;
            let body = &rest[4..4 + len];
            if kind == 0x0000 {
                let name_len = u16::from_be_bytes([body[3], body[4]]) as usize;
                return Some(String::from_utf8(body[5..5 + name_len].to_vec()).unwrap());
            }
            rest = &rest[4 + len..];
        }
        None
    }

    #[test]
    fn the_hello_carries_the_hostname_a_server_would_read() {
        assert_eq!(
            sni_from(&client_hello("www.youtube.com")).as_deref(),
            Some("www.youtube.com")
        );
        // A long name moves every enclosing length field; that is where an off-by-one hides.
        let long = format!("{}.example.com", "a".repeat(200));
        assert_eq!(sni_from(&client_hello(&long)).as_deref(), Some(&*long));
    }

    #[test]
    fn two_hellos_do_not_look_like_one_replayed_packet() {
        let a = client_hello("example.com");
        let b = client_hello("example.com");
        assert_ne!(a, b);
        assert_eq!(a.len(), b.len());
    }

    #[test]
    fn addresses_nothing_can_be_hosted_on_read_as_poisoned() {
        for ip in [
            Ipv4Addr::new(127, 0, 0, 1),
            Ipv4Addr::UNSPECIFIED,
            Ipv4Addr::new(10, 1, 2, 3),
            Ipv4Addr::new(192, 168, 0, 1),
        ] {
            assert!(is_sinkhole(ip.into()), "{ip}");
        }
        assert!(!is_sinkhole(Ipv4Addr::new(93, 184, 215, 14).into()));
        assert!(is_sinkhole(Ipv6Addr::LOCALHOST.into()));
        assert!(!is_sinkhole(
            "2606:2800:21f:cb07:6820:80da:af6b:8b2c".parse().unwrap()
        ));
    }

    #[test]
    fn a_hostname_that_would_not_fit_the_sni_field_is_refused() {
        assert!(check_hostname("www.youtube.com").is_ok());
        assert!(check_hostname("").is_err());
        assert!(check_hostname(&"a".repeat(254)).is_err());
        assert!(check_hostname("пример.рф").is_err());
        assert!(check_hostname("host name").is_err());
    }

    /// Answers the first connection with `behaviour`, so server-dependent verdicts can be produced on demand.
    fn server(
        behaviour: impl FnOnce(TcpStream) + Send + 'static,
    ) -> (SocketAddr, std::thread::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let handle = std::thread::spawn(move || {
            if let Ok((stream, _)) = listener.accept() {
                behaviour(stream);
            }
        });
        (address, handle)
    }

    fn swallow_hello(stream: &mut TcpStream) {
        let mut buf = [0u8; 1024];
        let _ = stream.read(&mut buf);
    }

    const SHORT: Duration = Duration::from_millis(300);

    #[test]
    fn a_server_that_answers_is_reachable() {
        let (address, handle) = server(|mut stream| {
            swallow_hello(&mut stream);
            let _ = stream.write_all(&[0x16, 0x03, 0x03, 0x00, 0x2a]);
        });
        assert_eq!(
            probe_address(address, "example.com", SHORT).0,
            Verdict::Fine
        );
        handle.join().unwrap();
    }

    #[test]
    fn an_alert_still_counts_as_an_answer() {
        // Something parsed the hello and replied. Censors inject resets, not alerts.
        let (address, handle) = server(|mut stream| {
            swallow_hello(&mut stream);
            let _ = stream.write_all(&[0x15, 0x03, 0x03, 0x00, 0x02, 0x02, 0x28]);
        });
        let (verdict, detail) = probe_address(address, "example.com", SHORT);
        assert_eq!(verdict, Verdict::Fine);
        assert!(detail.unwrap().contains("alert"));
        handle.join().unwrap();
    }

    #[test]
    fn a_connection_that_dies_after_the_hello_is_a_reset() {
        let (address, handle) = server(|mut stream| {
            swallow_hello(&mut stream);
            drop(stream);
        });
        assert_eq!(
            probe_address(address, "example.com", SHORT).0,
            Verdict::TlsReset
        );
        handle.join().unwrap();
    }

    #[test]
    fn a_connection_that_never_answers_is_silent() {
        let (address, handle) = server(|mut stream| {
            swallow_hello(&mut stream);
            std::thread::sleep(SHORT * 4);
        });
        assert_eq!(
            probe_address(address, "example.com", SHORT).0,
            Verdict::TlsSilent
        );
        handle.join().unwrap();
    }

    #[test]
    fn something_that_is_not_tls_is_not_an_answer() {
        let (address, handle) = server(|mut stream| {
            swallow_hello(&mut stream);
            let _ = stream.write_all(b"HTTP/1.1 403 Forbidden\r\n");
        });
        assert_eq!(
            probe_address(address, "example.com", SHORT).0,
            Verdict::TlsReset
        );
        handle.join().unwrap();
    }

    #[test]
    fn a_port_with_nothing_on_it_is_a_tcp_block() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        drop(listener);
        assert_eq!(
            probe_address(address, "example.com", SHORT).0,
            Verdict::TcpBlocked
        );
    }

    /// Real network, so not run by default: `cargo test -p blkbstr-core -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn checks_this_machine_against_the_real_internet() {
        let hosts: Vec<String> = DEFAULT_HOSTS.iter().map(|h| (*h).to_owned()).collect();
        let report = check(&hosts, DEFAULT_TIMEOUT);
        println!(
            "network_ok={} control={:?}",
            report.network_ok, report.control
        );
        for site in &report.sites {
            println!("{:>22}  {:?}  {:?}", site.host, site.verdict, site.detail);
        }
    }
}
