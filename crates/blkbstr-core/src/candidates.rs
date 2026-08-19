//! Strategies to try, in the order worth trying them.
//!
//! The order is a heuristic and nothing more: it decides how *fast* auto-configuration finds an
//! answer, never *whether* the answer is right. What a candidate is judged on is the reachability
//! check run while it is applied, so a badly ranked list costs time and a wrong one costs nothing —
//! the engine's own `--dry-run` refuses a candidate it cannot load before it reaches the network.
//!
//! Every function named here is in [`crate::registry::FUNCTIONS`], which the tests assert, so a
//! typo is a build failure rather than a candidate that silently never works.

use crate::config::{Action, Config, Filter, Strategy};

/// Split positions, in the order zapret2's own `blockcheck2` sweeps them. A strategy is only as
/// good as where it cuts, and `multisplit` with no `pos` splits after one byte, which almost
/// nothing falls for.
const TLS_POSITIONS: &[&str] = &[
    "1,midsld",
    "midsld",
    "sniext+1",
    "sniext+4",
    "host+1",
    "2",
    "1",
    "1,midsld,1220",
    "1,sniext+1,host+1,midsld-2,midsld,midsld+2,endhost-1",
];

/// The positions worth combining with something else. Sweeping every fooling against all nine
/// would be several hundred candidates, and blockcheck2 only reaches that many because it prunes
/// as it goes — which this list cannot do, since the user chooses from the finished results.
const TLS_KEY_POSITIONS: &[&str] = &["1,midsld", "midsld", "sniext+1"];

const HTTP_POSITIONS: &[&str] = &["method+2,midsld", "midsld", "method+2"];

/// `FOOLINGS46_TCP` from blockcheck2's `def.inc`: ways to build a packet the DPI accepts and the
/// server throws away. Each is `(name, args)`, where args are appended to the action.
const FOOLINGS: &[(&str, &[(&str, &str)])] = &[
    ("md5", &[("tcp_md5", "")]),
    ("badsum", &[("badsum", "")]),
    ("seqback", &[("tcp_seq", "-3000")]),
    ("seqfwd", &[("tcp_seq", "1000000")]),
    ("ackback", &[("tcp_ack", "-66000"), ("tcp_ts_up", "")]),
    ("tsback", &[("tcp_ts", "-1000")]),
    ("noack", &[("tcp_flags_unset", "ACK")]),
    ("syn", &[("tcp_flags_set", "SYN")]),
];

/// Ordered most to least likely to be enough, and cheapest first within each family: the walk is
/// stopped by the user, so what comes early is what a short run gets to try.
///
/// This is blockcheck2's sweep, narrowed. Its own search runs to several hundred combinations
/// because it prunes after each stage ("do not test fakedsplit if multisplit works"); this list
/// cannot prune, because every result is reported and the choice is the user's.
pub fn candidates() -> Vec<Config> {
    let mut out = Vec::new();

    // Splitting alone. Beats a DPI that reads one packet and does not reassemble.
    for function in ["multisplit", "multidisorder"] {
        for pos in TLS_POSITIONS {
            out.push(tls(
                &name(&[function, pos]),
                &format!("Split the TLS hello at {}", describe(pos)),
                vec![split(function, pos)],
            ));
        }
    }

    // Overlapping the first segment with data the DPI has already passed on.
    for pos in TLS_KEY_POSITIONS {
        out.push(tls(
            &name(&["multisplit", "seqovl", pos]),
            &format!(
                "Split at {}, overlapping the segment before it",
                describe(pos)
            ),
            vec![split("multisplit", pos).with("seqovl", "1")],
        ));
    }

    // A decoy in the gap, discarded somewhere between here and the server. This is the family that
    // beats a DPI which reassembles the stream, so it gets the full fooling sweep.
    for function in ["fakedsplit", "fakeddisorder"] {
        for (fooling, args) in FOOLINGS {
            for pos in TLS_KEY_POSITIONS {
                out.push(tls(
                    &name(&[function, fooling, pos]),
                    &format!(
                        "Split at {} with a decoy the server discards",
                        describe(pos)
                    ),
                    vec![with_args(split(function, pos), args)],
                ));
            }
        }
    }

    // A whole decoy hello before the real one, then a split.
    for (fooling, args) in FOOLINGS {
        out.push(tls(
            &name(&["fake", fooling]),
            "Send a whole decoy hello the server discards, then split the real one",
            vec![
                with_args(Action::new("fake").with("blob", "fake_default_tls"), args),
                split("multidisorder", TLS_KEY_POSITIONS[0]),
            ],
        ));
    }

    // The same decoy, expiring before it reaches the server. autottl finds the hop count itself;
    // the fixed values are for when it guesses wrong.
    for delta in 1..=5 {
        out.push(tls(
            &name(&["fake", "autottl", &delta.to_string()]),
            "Send a decoy that expires before the server sees it, then split the real hello",
            vec![
                Action::new("fake")
                    .with("blob", "fake_default_tls")
                    .with("ip4_autottl", format!("-{delta},3-20"))
                    .with("repeats", "2"),
                split("multidisorder", TLS_KEY_POSITIONS[0]),
            ],
        ));
    }
    for ttl in 1..=12 {
        out.push(tls(
            &name(&["fake", "ttl", &ttl.to_string()]),
            &format!("Send a decoy that expires {ttl} hops away, then split the real hello"),
            vec![
                Action::new("fake")
                    .with("blob", "fake_default_tls")
                    .with("ip4_ttl", ttl.to_string())
                    .with("repeats", "2"),
                split("multidisorder", TLS_KEY_POSITIONS[0]),
            ],
        ));
    }

    // Making the kernel do the splitting, for a DPI that only reads the first segment.
    for function in ["multisplit", "multidisorder"] {
        for pos in TLS_KEY_POSITIONS {
            out.push(tls(
                &name(["wssize", function, pos].as_slice()),
                &format!(
                    "Shrink the window so the hello arrives in pieces, split at {}",
                    describe(pos)
                ),
                vec![
                    Action::new("wssize").with("wsize", "1").with("scale", "6"),
                    split(function, pos),
                ],
            ));
        }
    }

    out.extend([
        tls(
            "syndata",
            "Put a decoy hello in the SYN packet, before the real one",
            vec![
                Action::new("syndata").with("blob", "fake_default_tls"),
                split("multisplit", TLS_KEY_POSITIONS[0]),
            ],
        ),
        tls(
            "syndata-http",
            "Put a decoy HTTP request in the SYN packet, before the real hello",
            vec![
                Action::new("syndata").with("blob", "fake_default_http"),
                split("multisplit", TLS_KEY_POSITIONS[0]),
            ],
        ),
        tls(
            "tcpseg-seqovl",
            "Re-segment the hello with the first piece overlapping the one before it",
            vec![Action::new("tcpseg")
                .with("pos", "0,-1")
                .with("seqovl", "1")],
        ),
        tls(
            "tcpseg-midsld",
            "Re-segment the hello at the domain name",
            vec![Action::new("tcpseg")
                .with("pos", "0,midsld")
                .with("ip_id", "rnd")],
        ),
        tls(
            "oob",
            "Send an out-of-band byte the inspector reads and the server ignores",
            vec![Action::new("oob")],
        ),
        tls(
            "oob-urp",
            "The same out-of-band byte, with the urgent pointer moved",
            vec![Action::new("oob").with("urp", "1")],
        ),
    ]);

    // HTTP. The reachability check speaks TLS only, so the walk cannot currently measure these —
    // they are here so a config editor and an eventual HTTP probe have something to start from.
    for function in ["multisplit", "multidisorder"] {
        for pos in HTTP_POSITIONS {
            out.push(http(
                &name(&["http", function, pos]),
                &format!("Split the request at {}", describe(pos)),
                vec![split(function, pos)],
            ));
        }
    }
    out.push(http(
        "http-hostcase",
        "Change the case of the Host header, which some inspectors match literally",
        vec![
            Action::new("http_hostcase"),
            split("multisplit", HTTP_POSITIONS[0]),
        ],
    ));

    out
}

/// `pos` is the argument that decides where a split lands; every splitting function takes it and
/// none of them do anything useful on their default of `2`.
fn split(function: &str, pos: &str) -> Action {
    Action::new(function).with("pos", pos)
}

fn with_args(mut action: Action, args: &[(&str, &str)]) -> Action {
    for (key, value) in args {
        action = action.with(*key, *value);
    }
    action
}

/// A config name is a file stem and an nfqws2 profile name, so a position like
/// `1,sniext+1,midsld-2` cannot go in raw.
fn name(parts: &[&str]) -> String {
    let joined = parts.join("-");
    let mut out = String::with_capacity(joined.len());
    let mut last_dash = true;
    for c in joined.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c);
            last_dash = false;
        } else if !last_dash {
            out.push('-');
            last_dash = true;
        }
    }
    out.trim_end_matches('-').to_owned()
}

/// Turns a position marker into something a user can read. Anything unrecognised is shown as is,
/// which is better than pretending to explain it.
fn describe(pos: &str) -> String {
    match pos {
        "1,midsld" => "the domain name, and again at the start".into(),
        "midsld" => "the domain name".into(),
        "sniext+1" => "the start of the server name extension".into(),
        "sniext+4" => "just inside the server name extension".into(),
        "host+1" => "the first byte of the hostname".into(),
        "method+2,midsld" => "the method, and again at the domain name".into(),
        "method+2" => "the method".into(),
        "2" => "its second byte".into(),
        "1" => "its first byte".into(),
        other => other.to_string(),
    }
}

/// A candidate and what it costs to run. The cost only ever orders strategies that already work,
/// so it can be a rough heuristic without being able to hide a working one.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Candidate {
    pub config: Config,
    pub cost: u32,
}

/// The candidate list with each entry's cost.
pub fn ranked() -> Vec<Candidate> {
    candidates()
        .into_iter()
        .map(|config| Candidate {
            cost: cost(&config),
            config,
        })
        .collect()
}

/// How much a strategy disturbs traffic beyond the minimum. Lower is gentler.
///
/// The weights say what they are paid for: injected packets are extra traffic that a middlebox or
/// the server may take badly, a deliberately malformed one more so, and `wssize` is the only thing
/// here that keeps costing throughput for the whole life of every connection it touches rather than
/// only during the handshake.
pub fn cost(config: &Config) -> u32 {
    let mut cost = 0;
    for strategy in config.strategies.iter().filter(|s| s.enabled) {
        for action in &strategy.actions {
            cost += 1;
            let repeats = action
                .args
                .get("repeats")
                .and_then(|r| r.parse::<u32>().ok())
                .unwrap_or(1);
            cost += match action.function.as_str() {
                // Sends packets that were never part of the conversation.
                "fake" | "syndata" | "hostfakesplit" => 3 * repeats,
                "fakedsplit" | "fakeddisorder" => 2 * repeats,
                // Shrinks the receive window for every connection on the port, for its whole life.
                "wssize" => 6,
                "oob" => 2,
                _ => 0,
            };
            // Each extra cut is another packet on the wire.
            if let Some(pos) = action.args.get("pos") {
                cost += pos.split(',').count().saturating_sub(1) as u32;
            }
            // Packets built to be discarded somewhere between here and the server.
            cost += action
                .args
                .keys()
                .filter(|k| {
                    matches!(k.as_str(), "badsum" | "tcp_md5" | "seqovl")
                        || k.contains("ttl")
                        || k.contains("autottl")
                })
                .count() as u32;
        }
    }
    cost
}

fn tls(name: &str, notes: &str, actions: Vec<Action>) -> Config {
    strategy_config(
        name,
        notes,
        "443",
        vec!["tls".into()],
        "tls_client_hello",
        actions,
    )
}

fn http(name: &str, notes: &str, actions: Vec<Action>) -> Config {
    strategy_config(name, notes, "80", vec!["http".into()], "http_req", actions)
}

fn strategy_config(
    name: &str,
    notes: &str,
    port: &str,
    l7: Vec<String>,
    payload: &str,
    mut actions: Vec<Action>,
) -> Config {
    let mut config = Config::new(name);
    config.notes = Some(notes.to_owned());
    let mut strategy = Strategy::new(name);
    strategy.filter = Filter {
        tcp: Some(port.to_owned()),
        l7,
        ..Filter::default()
    };
    for action in &mut actions {
        action.payload = vec![payload.to_owned()];
    }
    strategy.actions = actions;
    config.strategies.push(strategy);
    config
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry;

    #[test]
    fn every_candidate_is_valid_and_names_functions_this_build_knows() {
        for config in candidates() {
            config
                .validate()
                .unwrap_or_else(|e| panic!("{}: {e}", config.name));
            // An unknown function is passed through to the engine rather than refused, so a typo
            // would otherwise become a candidate that quietly never works. Asked of the registry
            // directly rather than by reading lint's prose, which is free to change.
            for action in config.strategies.iter().flat_map(|s| &s.actions) {
                assert!(
                    registry::function(&action.function).is_some(),
                    "{}: `{}` is not a function this build knows",
                    config.name,
                    action.function
                );
            }
        }
    }

    #[test]
    fn every_candidate_intercepts_something() {
        for config in candidates() {
            let ports: Vec<_> = config
                .strategies
                .iter()
                .filter(|s| s.enabled)
                .filter_map(|s| s.filter.tcp.clone())
                .collect();
            assert!(!ports.is_empty(), "{} selects no TCP port", config.name);
        }
    }

    /// The ranking exists to prefer the gentler of two strategies that both work, so the ordering
    /// between these three has to hold: splitting alone < injected decoys < a shrunken window.
    #[test]
    fn cost_orders_gentle_below_invasive() {
        let by_name = |name: &str| {
            let config = candidates()
                .into_iter()
                .find(|c| c.name == name)
                .unwrap_or_else(|| panic!("no candidate named {name}"));
            cost(&config)
        };
        assert!(by_name("multisplit-midsld") < by_name("fake-md5"));
        assert!(by_name("fake-md5") < by_name("wssize-multisplit-midsld"));
        assert!(by_name("multisplit-2") < by_name("multisplit-1-midsld"));
        assert_eq!(ranked().len(), candidates().len());
    }

    #[test]
    fn candidate_names_are_unique_and_usable_as_filenames() {
        let mut names: Vec<_> = candidates().iter().map(|c| c.name.clone()).collect();
        let before = names.len();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), before, "duplicate candidate name");
    }
}

#[cfg(test)]
mod render_tests {
    use super::*;
    use crate::registry::Platform;
    use crate::render::{parameter_file, EngineOptions};

    fn options() -> EngineOptions {
        EngineOptions {
            platform: Platform::Linux,
            queue_num: 200,
            pidfile: "/run/blkbstr/engine.pid".into(),
            debug_log: None,
            lua_init: Vec::new(),
            validate: false,
        }
    }

    /// Prints what every candidate becomes on the wire. Not an assertion — it is the artifact to
    /// read when a candidate turns out not to work against a real engine.
    #[test]
    #[ignore]
    fn show_the_cost_of_each_candidate() {
        let mut all = ranked();
        all.sort_by_key(|c| c.cost);
        for c in all {
            println!("{:>3}  {}", c.cost, c.config.name);
        }
    }

    #[test]
    #[ignore]
    fn show_what_each_candidate_renders_to() {
        for config in candidates() {
            println!("# {}\n{}", config.name, parameter_file(&config, &options()));
        }
    }

    /// `--name` and not `--new`: profile 1 is already open, so `--new` would leave it empty, and an
    /// empty profile matches every packet and passes it through untouched.
    #[test]
    fn each_candidate_renders_one_profile_with_at_least_one_action() {
        for config in candidates() {
            let rendered = parameter_file(&config, &options());
            assert_eq!(
                rendered.matches("--name=").count(),
                1,
                "{}: {rendered}",
                config.name
            );
            assert_eq!(rendered.matches("--new=").count(), 0, "{}", config.name);
            assert!(
                rendered.contains("--lua-desync="),
                "{}: {rendered}",
                config.name
            );
            assert!(
                rendered.contains("--filter-tcp="),
                "{}: {rendered}",
                config.name
            );
        }
    }
}
