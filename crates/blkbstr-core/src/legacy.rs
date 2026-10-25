//! Reads the `config` file of an existing zapret2 install into a [`Config`]. The part that matters
//! is `NFQWS2_OPT`: the same options Blockbuster renders, in one shell string.

use crate::config::{Action, Config, Strategy};

#[derive(Debug)]
pub struct Imported {
    pub config: Config,
    /// Options that did not map onto a [`Config`], verbatim, so the user can see what was left out.
    pub skipped: Vec<String>,
}

/// `name` is what the config is saved as. Fails only when nothing usable was found or the result
/// would not pass [`Config::validate`], since the file is untrusted input like any import.
pub fn parse(config_file: &str, name: &str) -> Result<Imported, String> {
    let opt = shell_var(config_file, "NFQWS2_OPT").ok_or("no NFQWS2_OPT in that config")?;
    let filtering = shell_var(config_file, "MODE_FILTER").is_some_and(|m| m != "none");

    let mut config = Config::new(name);
    // The profiles were written to apply to all traffic or to lists of the user's choosing.
    config.auto_hostlist = false;
    let mut skipped = Vec::new();
    for (n, profile) in split_profiles(&opt).into_iter().enumerate() {
        let mut strategy = Strategy::new(format!("profile-{}", n + 1));
        let mut payload: Vec<String> = Vec::new();
        for token in profile {
            let (flag, value) = token.split_once('=').unwrap_or((token, ""));
            let list = || value.split(',').map(str::to_owned).collect::<Vec<_>>();
            let filter = &mut strategy.filter;
            match flag {
                "--name" if !value.is_empty() => strategy.name = value.to_owned(),
                "--filter-l3" => filter.l3 = Some(value.to_owned()),
                "--filter-tcp" => filter.tcp = Some(value.to_owned()),
                "--filter-udp" => filter.udp = Some(value.to_owned()),
                "--filter-l7" => filter.l7 = list(),
                "--hostlist" => filter.hostlist = Some(value.to_owned()),
                "--hostlist-exclude" => filter.hostlist_exclude = Some(value.to_owned()),
                "--hostlist-auto" => filter.hostlist_auto = Some(value.to_owned()),
                "--ipset" => filter.ipset = Some(value.to_owned()),
                "--ipset-exclude" => filter.ipset_exclude = Some(value.to_owned()),
                // Sticky in nfqws2: it applies to every `--lua-desync` after it.
                "--payload" => payload = list(),
                "--lua-desync" => {
                    let mut parts = value.split(':');
                    let mut action = Action::new(parts.next().unwrap_or_default());
                    action.payload = payload.clone();
                    for arg in parts {
                        let (k, v) = arg.split_once('=').unwrap_or((arg, ""));
                        action.args.insert(k.to_owned(), v.to_owned());
                    }
                    strategy.actions.push(action);
                }
                // The init script swaps these for whichever lists `MODE_FILTER` selects, which
                // Blockbuster does not manage.
                _ if token.starts_with('<') => {
                    if filtering {
                        skipped.push(format!("{token} (list selection is not imported)"));
                    }
                }
                _ => skipped.push(token.to_owned()),
            }
        }
        if !strategy.actions.is_empty() {
            config.strategies.push(strategy);
        }
    }
    if config.strategies.is_empty() {
        return Err("NFQWS2_OPT holds no --lua-desync actions to import".into());
    }
    config.validate().map_err(|e| e.to_string())?;
    Ok(Imported { config, skipped })
}

/// `--new` separates profiles. Tokens are whitespace-split: zapret2 option values do not contain spaces.
fn split_profiles(opt: &str) -> Vec<Vec<&str>> {
    let mut profiles = vec![Vec::new()];
    for token in opt.split_whitespace() {
        if token == "--new" {
            profiles.push(Vec::new());
        } else if let Some(last) = profiles.last_mut() {
            last.push(token);
        }
    }
    profiles
}

/// The value of `NAME="..."` or `NAME=...`, which may span lines. Never evaluated: the file is shell
/// but is only read, so `$(...)` inside it stays text.
fn shell_var(text: &str, name: &str) -> Option<String> {
    let start = text
        .lines()
        .scan(0, |off, l| {
            let at = *off;
            *off += l.len() + 1;
            Some((at, l))
        })
        .find(|(_, l)| l.starts_with(name) && l[name.len()..].starts_with('='))
        .map(|(at, _)| at + name.len() + 1)?;
    let rest = &text[start..];
    match rest.chars().next()? {
        q @ ('"' | '\'') => rest[1..].find(q).map(|end| rest[1..1 + end].to_owned()),
        _ => Some(rest.lines().next()?.trim().to_owned()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"
# comment
NFQWS2_ENABLE=0
NFQWS2_PORTS_TCP=80,443
NFQWS2_OPT="
--filter-tcp=80 --filter-l7=http <HOSTLIST> --payload=http_req --lua-desync=fake:blob=fake_default_http:tcp_md5 --lua-desync=multisplit:pos=method+2 --new
--filter-tcp=443 --filter-l7=tls <HOSTLIST> --payload=tls_client_hello --lua-desync=fake:blob=fake_default_tls:tcp_md5:tcp_seq=-10000 --lua-desync=multidisorder:pos=1,midsld --new
--filter-udp=443 --filter-l7=quic <HOSTLIST_NOAUTO> --payload=quic_initial --lua-desync=fake:blob=fake_default_quic:repeats=6
"
MODE_FILTER=none
"#;

    #[test]
    fn reads_zapret2s_default_config() {
        let got = parse(SAMPLE, "Legacy").unwrap();
        let c = &got.config;
        assert_eq!(c.name, "Legacy");
        assert_eq!(c.strategies.len(), 3);

        let tls = &c.strategies[1];
        assert_eq!(tls.filter.tcp.as_deref(), Some("443"));
        assert_eq!(tls.filter.l7, ["tls"]);
        let fake = &tls.actions[0];
        assert_eq!(fake.function, "fake");
        assert_eq!(fake.payload, ["tls_client_hello"]);
        assert_eq!(fake.args["tcp_md5"], "", "a bare flag stays a bare flag");
        assert_eq!(fake.args["tcp_seq"], "-10000");
        assert_eq!(tls.actions[1].args["pos"], "1,midsld");
        // `--payload` is sticky, so it reaches the second action too.
        assert_eq!(tls.actions[1].payload, ["tls_client_hello"]);

        assert_eq!(c.strategies[2].filter.udp.as_deref(), Some("443"));
        assert!(got.skipped.is_empty(), "MODE_FILTER=none: nothing is lost");
    }

    #[test]
    fn says_so_when_list_selection_is_dropped() {
        let text = SAMPLE.replace("MODE_FILTER=none", "MODE_FILTER=hostlist");
        let got = parse(&text, "Legacy").unwrap();
        assert!(got.skipped.iter().any(|s| s.starts_with("<HOSTLIST>")));
    }

    #[test]
    fn keeps_what_it_cannot_map_and_refuses_what_is_unsafe() {
        let got = parse(
            "NFQWS2_OPT=\"--ctrack-timeouts=1 --lua-desync=fake\"\n",
            "x",
        )
        .unwrap();
        assert_eq!(got.skipped, ["--ctrack-timeouts=1"]);
        assert!(parse("NFQWS2_OPT=\"--lua-desync=fake --new\"\n", "../x").is_err());
        assert!(parse("NFQWS2_ENABLE=0\n", "x").is_err());
        assert!(parse("NFQWS2_OPT=\"--filter-tcp=443\"\n", "x").is_err());
    }
}
