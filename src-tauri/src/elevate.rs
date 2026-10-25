//! Runs one program elevated through a UAC prompt. The GUI never runs as an administrator; this is
//! the only way it asks for more, and each call is one prompt the user sees and can refuse.

/// `ERROR_CANCELLED`: the user declined the prompt.
const DISMISSED: i32 = 1223;

/// A PowerShell single-quoted string holding a double-quoted argument. `Start-Process` joins its
/// argument list with spaces and no quoting of its own, so a path with a space would split.
fn quote(arg: &str) -> String {
    format!("'\"{}\"'", arg.replace('\'', "''"))
}

fn script(program: &str, args: &[&str]) -> String {
    let list = args.iter().map(|a| quote(a)).collect::<Vec<_>>().join(",");
    let arguments = if args.is_empty() {
        String::new()
    } else {
        format!(" -ArgumentList {list}")
    };
    format!(
        "$ErrorActionPreference='Stop'; try {{ \
         $p = Start-Process -FilePath '{}'{arguments} -Verb RunAs -WindowStyle Hidden -Wait -PassThru; \
         exit $p.ExitCode }} catch {{ [Console]::Error.WriteLine($_.Exception.Message); exit {DISMISSED} }}",
        program.replace('\'', "''")
    )
}

/// `blkbstrd.exe`, which the installer puts next to the app.
#[cfg(windows)]
pub fn daemon_path() -> Result<std::path::PathBuf, String> {
    let exe = std::env::current_exe().map_err(|e| format!("finding the app: {e}"))?;
    let daemon = exe.with_file_name("blkbstrd.exe");
    daemon.is_file().then_some(daemon).ok_or_else(|| {
        "blkbstrd.exe is not next to the app. Reinstall Blockbuster with its installer".into()
    })
}

/// Runs `program args...` as administrator and waits. `what` names the step for the error text.
#[cfg(windows)]
pub fn run(program: &std::path::Path, args: &[&str], what: &str) -> Result<(), String> {
    tracing::info!(program = %program.display(), ?args, "asking for administrator permission");
    let out = blkbstr_core::process::command("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-Command"])
        .arg(script(&program.display().to_string(), args))
        .output()
        .map_err(|e| format!("running powershell: {e}"))?;
    tracing::info!(
        code = ?out.status.code(),
        stderr = %String::from_utf8_lossy(&out.stderr).trim(),
        "the elevated step ended"
    );
    match out.status.code() {
        Some(0) => Ok(()),
        Some(DISMISSED) => Err("the authorisation prompt was dismissed".into()),
        code => Err(format!(
            "{what} failed ({}). The Logs tab has blkbstrd.log with the reason",
            code.map_or("no exit code".into(), |c| format!("exit code {c}"))
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_with_spaces_and_quotes_stay_one_argument() {
        assert_eq!(quote(r"C:\Users\A B\x.zip"), r#"'"C:\Users\A B\x.zip"'"#);
        assert_eq!(quote("it's"), r#"'"it''s"'"#);
    }

    #[test]
    fn the_script_elevates_and_reports_a_dismissed_prompt() {
        let s = script(
            r"C:\Program Files\Blockbuster\blkbstrd.exe",
            &["install-service", "--allow-sid", "S-1-5-21-1"],
        );
        assert!(
            s.contains("-Verb RunAs") && s.contains("-Wait -PassThru"),
            "{s}"
        );
        assert!(
            s.contains(r#"-ArgumentList '"install-service"','"--allow-sid"','"S-1-5-21-1"'"#),
            "{s}"
        );
        assert!(s.contains("exit 1223"), "{s}");
        assert!(!script("x.exe", &[]).contains("-ArgumentList"));
    }
}
