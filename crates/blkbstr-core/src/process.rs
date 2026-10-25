//! Child processes started from a GUI or a service have no console of their own, so Windows opens
//! one per child unless told not to.

use std::ffi::OsStr;
use std::process::Command;

#[cfg(windows)]
pub fn command(program: impl AsRef<OsStr>) -> Command {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let mut command = Command::new(program);
    command.creation_flags(CREATE_NO_WINDOW);
    command
}

#[cfg(not(windows))]
pub fn command(program: impl AsRef<OsStr>) -> Command {
    Command::new(program)
}
