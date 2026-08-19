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

/// Ordered most to least likely to be enough. TLS first: SNI inspection is the common case, and a
/// blocked site that loads once the ClientHello is broken up needs nothing else.
pub fn candidates() -> Vec<Config> {
    vec![
        tls(
            "multisplit",
            "Split the TLS hello so the hostname spans two packets",
            vec![Action::new("multisplit").with("strategy", "1")],
        ),
        tls(
            "multidisorder",
            "Split the hello and send the pieces out of order",
            vec![Action::new("multidisorder").with("strategy", "1")],
        ),
        tls(
            "fake-multidisorder",
            "Send a decoy hello with a bad checksum, then split and reorder the real one",
            vec![
                Action::new("fake")
                    .with("blob", "fake_default_tls")
                    .with("badsum", "")
                    .with("strategy", "1"),
                Action::new("multidisorder").with("strategy", "2"),
            ],
        ),
        tls(
            "fakedsplit",
            "Split the hello with a decoy in the gap",
            vec![Action::new("fakedsplit").with("strategy", "1")],
        ),
        tls(
            "fakeddisorder",
            "Split the hello with a decoy, out of order",
            vec![Action::new("fakeddisorder").with("strategy", "1")],
        ),
        tls(
            "wssize",
            "Shrink the advertised window so the hello arrives in pieces",
            vec![Action::new("wssize").with("strategy", "1")],
        ),
        tls(
            "oob",
            "Send an out-of-band byte the inspector reads and the server ignores",
            vec![Action::new("oob").with("strategy", "1")],
        ),
        http(
            "http-hostcase",
            "Change the case of the Host header, which some inspectors match literally",
            vec![
                Action::new("http_hostcase").with("strategy", "1"),
                Action::new("multisplit").with("strategy", "2"),
            ],
        ),
    ]
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
    fn show_what_each_candidate_renders_to() {
        for config in candidates() {
            println!("# {}\n{}", config.name, parameter_file(&config, &options()));
        }
    }

    #[test]
    fn each_candidate_renders_one_profile_with_at_least_one_action() {
        for config in candidates() {
            let rendered = parameter_file(&config, &options());
            assert_eq!(
                rendered.matches("--new=").count(),
                1,
                "{}: {rendered}",
                config.name
            );
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
