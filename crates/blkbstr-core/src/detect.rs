//! What is already on this machine, before Blockbuster installs anything.
//!
//! Runs unprivileged from the GUI, because onboarding has to say what is missing while there is
//! still no daemon to ask. Every probe is best-effort: "not found" is the normal first-run answer,
//! not an error, so nothing here returns a `Result`.

use crate::registry::Platform;
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, Serialize)]
pub struct Tool {
    pub path: PathBuf,
    /// First line of the tool's own version output, verbatim — parsing it further is the caller's
    /// problem, and showing it unedited is what makes a bug report useful.
    pub version: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum LuaFlavour {
    LuaJit,
    Puc,
}

#[derive(Debug, Clone, Serialize)]
pub struct LuaRuntime {
    #[serde(flatten)]
    pub tool: Tool,
    pub flavour: LuaFlavour,
    pub major: u32,
    pub minor: u32,
    /// zapret2's attacks are Lua, not C. Below LuaJIT 2.1 or PUC 5.3 the engine starts and then
    /// fails to load them, which reads as "nothing happened" — so this is checked before any of
    /// the rest is offered.
    pub supported: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum PackageManager {
    Pacman,
    Apt,
    Dnf,
    Zypper,
    Apk,
    Xbps,
    Portage,
}

#[derive(Debug, Clone, Serialize)]
pub struct Distro {
    pub id: String,
    pub pretty_name: Option<String>,
    pub package_manager: Option<PackageManager>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct Environment {
    pub platform: Option<Platform>,
    pub engine: Option<Tool>,
    pub lua: Option<LuaRuntime>,
    pub nftables: Option<Tool>,
    pub distro: Option<Distro>,
    /// A zapret2 tree already on the machine — something to import from rather than write over.
    pub existing_install: Option<PathBuf>,
}

/// Everything onboarding needs to decide what to do next.
pub fn environment() -> Environment {
    let platform = Platform::current();
    Environment {
        platform,
        engine: platform
            .and_then(|p| locate_engine(p.engine_binary()))
            .map(|path| Tool {
                version: version_of(&path, "--version"),
                path,
            }),
        lua: lua_runtime(),
        nftables: which("nft").map(|path| Tool {
            version: version_of(&path, "--version"),
            path,
        }),
        distro: distro(),
        existing_install: existing_install(),
    }
}

/// Looks for the engine next to the running binary, in the usual install locations, then on PATH.
pub fn locate_engine(binary: &str) -> Option<PathBuf> {
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            candidates.push(dir.join(binary));
        }
    }
    candidates.extend(
        [
            "/opt/zapret2",
            "/usr/local/bin",
            "/usr/bin",
            "/usr/local/libexec",
        ]
        .iter()
        .map(|d| Path::new(d).join(binary)),
    );
    // PATH read directly rather than shelling out to `command -v`: a root daemon that never execs
    // a shell is one less thing for an LSM profile to allow and for a reader to worry about.
    if let Some(path) = std::env::var_os("PATH") {
        candidates.extend(std::env::split_paths(&path).map(|d| d.join(binary)));
    }
    candidates.into_iter().find(|p| p.is_file())
}

/// zapret2's own Lua library, in the order its init script loads it. `zapret-lib.lua` defines the
/// primitives the others build on, so the order is not decorative.
pub const LUA_SCRIPTS: &[&str] = &["zapret-lib.lua", "zapret-antidpi.lua", "zapret-auto.lua"];

/// Looks for the directory holding [`LUA_SCRIPTS`]. Distribution packages split the binary from the
/// scripts — the AUR `zapret2-bin` puts `nfqws2` on PATH and the Lua under `/opt/zapret2/lua` — so
/// this is searched independently of the engine.
pub fn locate_lua_dir() -> Option<PathBuf> {
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            candidates.push(dir.join("lua"));
            candidates.push(dir.join("../lua"));
        }
    }
    candidates.extend(
        [
            "/opt/zapret2/lua",
            "/usr/share/zapret2/lua",
            "/usr/local/share/zapret2/lua",
            "/usr/lib/zapret2/lua",
        ]
        .iter()
        .map(PathBuf::from),
    );
    candidates
        .into_iter()
        .find(|d| d.join(LUA_SCRIPTS[0]).is_file())
}

/// The Lua files to pass as `--lua-init`, in load order. Missing ones are skipped rather than
/// failing: only `zapret-lib.lua` is load-bearing for every action.
pub fn lua_init_scripts(dir: &Path) -> Vec<String> {
    LUA_SCRIPTS
        .iter()
        .map(|name| dir.join(name))
        .filter(|p| p.is_file())
        .map(|p| p.display().to_string())
        .collect()
}

/// The best Lua on the machine. Prefers a supported runtime over a newer-looking one: a box with
/// both `lua` 5.1 and `lua5.4` installed has a working setup, and reporting the 5.1 would send the
/// user off installing something they already have.
fn lua_runtime() -> Option<LuaRuntime> {
    let found: Vec<LuaRuntime> = ["luajit", "lua", "lua5.4", "lua5.3"]
        .iter()
        .filter_map(|name| {
            let path = which(name)?;
            let line = version_of(&path, "-v")?;
            let (flavour, major, minor) = parse_lua_version(&line)?;
            Some(LuaRuntime {
                tool: Tool {
                    path,
                    version: Some(line),
                },
                flavour,
                major,
                minor,
                supported: lua_supported(flavour, major, minor),
            })
        })
        .collect();
    found
        .iter()
        .find(|l| l.supported)
        .or_else(|| found.first())
        .cloned()
}

/// `LuaJIT 2.1.1785763465 -- Copyright ...` or `Lua 5.4.8  Copyright ...`
fn parse_lua_version(line: &str) -> Option<(LuaFlavour, u32, u32)> {
    let (flavour, rest) = line
        .strip_prefix("LuaJIT ")
        .map(|rest| (LuaFlavour::LuaJit, rest))
        .or_else(|| {
            line.strip_prefix("Lua ")
                .map(|rest| (LuaFlavour::Puc, rest))
        })?;
    let mut parts = rest.split_whitespace().next()?.split('.');
    Some((
        flavour,
        parts.next()?.parse().ok()?,
        parts.next()?.parse().ok()?,
    ))
}

fn lua_supported(flavour: LuaFlavour, major: u32, minor: u32) -> bool {
    match flavour {
        LuaFlavour::LuaJit => (major, minor) >= (2, 1),
        LuaFlavour::Puc => (major, minor) >= (5, 3),
    }
}

fn distro() -> Option<Distro> {
    let text = std::fs::read_to_string("/etc/os-release").ok()?;
    let (id, id_like, pretty_name) = parse_os_release(&text);
    let id = id?;
    Some(Distro {
        package_manager: package_manager_for(&id, id_like.as_deref()),
        id,
        pretty_name,
    })
}

fn parse_os_release(text: &str) -> (Option<String>, Option<String>, Option<String>) {
    let value = |key: &str| {
        text.lines()
            .find_map(|l| l.strip_prefix(key)?.strip_prefix('='))
            .map(|v| v.trim_matches('"').to_owned())
            .filter(|v| !v.is_empty())
    };
    (value("ID"), value("ID_LIKE"), value("PRETTY_NAME"))
}

/// From the distro id, never from which binaries happen to exist: a machine can carry a stray
/// `apt-get` from a container tool or an old experiment, and an Arch box that answers "apt" gets
/// told to install packages that are not there.
fn package_manager_for(id: &str, id_like: Option<&str>) -> Option<PackageManager> {
    // ID_LIKE names the parent, which is how derivatives are covered without listing them all.
    std::iter::once(id)
        .chain(id_like.unwrap_or_default().split_whitespace())
        .find_map(|name| match name {
            "arch" | "archlinux" => Some(PackageManager::Pacman),
            "debian" | "ubuntu" => Some(PackageManager::Apt),
            "fedora" | "rhel" | "centos" => Some(PackageManager::Dnf),
            "opensuse" | "suse" | "sles" => Some(PackageManager::Zypper),
            "alpine" => Some(PackageManager::Apk),
            "void" => Some(PackageManager::Xbps),
            "gentoo" => Some(PackageManager::Portage),
            _ => None,
        })
}

/// Identified by its `config` file, because that is the thing worth importing. A bare `nfqws2` on
/// PATH is a binary, not an installation with settings to migrate.
fn existing_install() -> Option<PathBuf> {
    ["/opt/zapret2", "/usr/local/zapret2"]
        .iter()
        .map(Path::new)
        .find(|dir| dir.join("config").is_file())
        .map(Path::to_path_buf)
}

fn which(binary: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|d| d.join(binary))
        .find(|p| p.is_file())
}

fn version_of(path: &Path, arg: &str) -> Option<String> {
    let out = Command::new(path).arg(arg).output().ok()?;
    // LuaJIT prints its banner on stderr; most everything else uses stdout.
    let first = |bytes: &[u8]| {
        String::from_utf8_lossy(bytes)
            .lines()
            .map(str::trim)
            .find(|l| !l.is_empty())
            .map(str::to_owned)
    };
    first(&out.stdout).or_else(|| first(&out.stderr))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Runs every probe against the machine the tests are on. Asserts almost nothing on purpose:
    /// what is installed varies, and the point is that probing a bare machine cannot panic or hang.
    #[test]
    fn probing_this_machine_answers_something() {
        let env = environment();
        assert_eq!(
            env.platform.is_some(),
            cfg!(any(target_os = "linux", target_os = "windows"))
        );
        if let Some(lua) = &env.lua {
            assert_eq!(
                lua.supported,
                lua_supported(lua.flavour, lua.major, lua.minor)
            );
        }
    }

    #[test]
    fn reads_both_lua_flavours() {
        assert_eq!(
            parse_lua_version("LuaJIT 2.1.1785763465 -- Copyright (C) 2005-2026 Mike Pall."),
            Some((LuaFlavour::LuaJit, 2, 1))
        );
        assert_eq!(
            parse_lua_version("Lua 5.5.1  Copyright (C) 1994-2026 Lua.org, PUC-Rio"),
            Some((LuaFlavour::Puc, 5, 5))
        );
        assert_eq!(parse_lua_version("bash: lua: command not found"), None);
    }

    #[test]
    fn only_the_versions_zapret2_can_use_count_as_supported() {
        assert!(lua_supported(LuaFlavour::LuaJit, 2, 1));
        assert!(!lua_supported(LuaFlavour::LuaJit, 2, 0));
        assert!(lua_supported(LuaFlavour::Puc, 5, 3));
        assert!(!lua_supported(LuaFlavour::Puc, 5, 2));
    }

    #[test]
    fn os_release_values_lose_their_quotes() {
        let text = "NAME=\"CachyOS Linux\"\nPRETTY_NAME=\"CachyOS\"\nID=cachyos\nID_LIKE=arch\n";
        let (id, id_like, pretty) = parse_os_release(text);
        assert_eq!(id.as_deref(), Some("cachyos"));
        assert_eq!(id_like.as_deref(), Some("arch"));
        assert_eq!(pretty.as_deref(), Some("CachyOS"));
        assert_eq!(parse_os_release("ID=\n").0, None);
    }

    #[test]
    fn derivatives_are_covered_by_their_parent() {
        // Neither id is in the table; both are resolved through ID_LIKE.
        assert_eq!(
            package_manager_for("cachyos", Some("arch")),
            Some(PackageManager::Pacman)
        );
        assert_eq!(
            package_manager_for("linuxmint", Some("ubuntu debian")),
            Some(PackageManager::Apt)
        );
        assert_eq!(
            package_manager_for("fedora", None),
            Some(PackageManager::Dnf)
        );
        assert_eq!(package_manager_for("plan9", None), None);
    }
}
