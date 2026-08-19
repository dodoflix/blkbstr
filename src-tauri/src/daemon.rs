//! Client side of the GUI ↔ daemon socket.
//!
//! One connection per request. The daemon is a service that may restart under the GUI at any
//! time, so a long-lived connection would only mean reconnect logic for no gain.

use blkbstr_core::paths;
use blkbstr_core::protocol::{read_message, write_message, Request, Response};
use interprocess::local_socket::{prelude::*, GenericFilePath, GenericNamespaced, Stream};
use std::io::BufReader;

/// Distinguished from a protocol-level error so the UI can offer "install the service" instead of
/// a raw I/O message.
#[derive(Debug)]
pub enum Error {
    Unreachable(String),
    Io(String),
    Daemon(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Unreachable(e) => write!(f, "blkbstrd is not running or not reachable: {e}"),
            Error::Io(e) => write!(f, "talking to blkbstrd failed: {e}"),
            Error::Daemon(e) => write!(f, "{e}"),
        }
    }
}

impl serde::Serialize for Error {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_string())
    }
}

fn connect() -> Result<Stream, Error> {
    let name = paths::socket_name();
    let socket = if paths::socket_is_namespaced() {
        name.clone()
            .to_ns_name::<GenericNamespaced>()
            .map_err(|e| Error::Io(e.to_string()))?
    } else {
        name.clone()
            .to_fs_name::<GenericFilePath>()
            .map_err(|e| Error::Io(e.to_string()))?
    };
    Stream::connect(socket).map_err(|e| {
        let mut message = e.to_string();
        if e.kind() == std::io::ErrorKind::PermissionDenied {
            message.push_str(&group_hint(&name).unwrap_or_default());
        }
        Error::Unreachable(message)
    })
}

/// Turns `Permission denied (os error 13)` into the one thing that fixes it. The socket's group is
/// the authorization model, and `usermod -aG` does not touch a session that is already running, so
/// a fresh install fails here until the user logs out and back in.
#[cfg(unix)]
fn group_hint(socket: &str) -> Option<String> {
    use std::os::unix::fs::MetadataExt;

    let gid = std::fs::metadata(socket).ok()?.gid();
    if process_groups().is_none_or(|groups| groups.contains(&gid)) {
        return None;
    }
    let (group, members) = group_entry(gid)?;
    let user = std::env::var("USER").ok()?;
    Some(if members.contains(&user) {
        format!(
            ". Your account is in the {group} group, but this session started before it was added \
             — log out and back in"
        )
    } else {
        format!(
            ". Your account is not in the {group} group — run \
             `sudo usermod -aG {group} {user}`, then log out and back in"
        )
    })
}

#[cfg(not(unix))]
fn group_hint(_socket: &str) -> Option<String> {
    None
}

/// Real and supplementary groups of this process, from `/proc`. `getgroups(2)` would need a libc
/// dependency for one call, and every platform this branch runs on has `/proc/self/status`.
#[cfg(unix)]
fn process_groups() -> Option<Vec<u32>> {
    let status = std::fs::read_to_string("/proc/self/status").ok()?;
    let field = |name: &str| {
        status
            .lines()
            .find_map(|l| l.strip_prefix(name))?
            .split_whitespace()
            .map(str::parse::<u32>)
            .collect::<Result<Vec<_>, _>>()
            .ok()
    };
    let mut groups = field("Groups:")?;
    groups.extend(field("Gid:")?);
    Some(groups)
}

#[cfg(unix)]
fn group_entry(gid: u32) -> Option<(String, Vec<String>)> {
    parse_group(&std::fs::read_to_string("/etc/group").ok()?, gid)
}

#[cfg(unix)]
fn parse_group(text: &str, gid: u32) -> Option<(String, Vec<String>)> {
    text.lines().find_map(|line| {
        // name:password:gid:member,member
        let mut fields = line.splitn(4, ':');
        let name = fields.next()?;
        fields.next()?;
        if fields.next()? != gid.to_string() {
            return None;
        }
        let members = fields
            .next()
            .unwrap_or_default()
            .split(',')
            .filter(|m| !m.is_empty())
            .map(str::to_owned)
            .collect();
        Some((name.to_owned(), members))
    })
}

pub fn request(req: Request) -> Result<Response, Error> {
    let conn = connect()?;
    let mut writer = &conn;
    write_message(&mut writer, &req).map_err(|e| Error::Io(e.to_string()))?;

    let mut reader = BufReader::new(&conn);
    match read_message::<Response>(&mut reader).map_err(|e| Error::Io(e.to_string()))? {
        Some(Response::Error { message, .. }) => Err(Error::Daemon(message)),
        Some(response) => Ok(response),
        None => Err(Error::Io(
            "daemon closed the connection without replying".into(),
        )),
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::parse_group;

    #[test]
    fn reads_a_group_line() {
        let text = "root:x:0:\nwheel:x:998:dodo\nblkbstr:x:1001:dodo,ada\nempty:x:1002:\n";
        assert_eq!(
            parse_group(text, 1001),
            Some(("blkbstr".into(), vec!["dodo".into(), "ada".into()]))
        );
        assert_eq!(parse_group(text, 1002), Some(("empty".into(), vec![])));
        assert_eq!(parse_group(text, 4242), None);
    }
}
