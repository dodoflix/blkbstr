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
    Stream::connect(socket).map_err(|e| Error::Unreachable(e.to_string()))
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
