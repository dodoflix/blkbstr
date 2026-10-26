//! Runs one program as root through `pkexec`. The GUI never runs as root; this is the only way it
//! asks for more, and each call is one password prompt the user sees and can refuse.

use std::path::Path;
use std::process::Command;

/// Runs `program args...` as root and waits. `what` names the step for the error text.
pub fn run(program: &Path, args: &[&str], what: &str) -> Result<(), String> {
    tracing::info!(program = %program.display(), ?args, "asking for root permission");
    let out = Command::new("pkexec")
        .arg(program)
        .args(args)
        .output()
        .map_err(|e| match e.kind() {
            std::io::ErrorKind::NotFound => {
                "pkexec is not installed (it comes with polkit)".to_owned()
            }
            _ => format!("running pkexec: {e}"),
        })?;
    if out.status.success() {
        return Ok(());
    }
    Err(failure(
        out.status.code(),
        &String::from_utf8_lossy(&out.stderr),
        what,
    ))
}

/// pkexec exits 126 when the prompt is dismissed and 127 when it cannot be shown, both silent.
fn failure(code: Option<i32>, stderr: &str, what: &str) -> String {
    let said = stderr.trim();
    match code {
        Some(126) => "the authorisation prompt was dismissed".into(),
        _ if said.is_empty() => format!(
            "{what} failed ({})",
            code.map_or("no exit code".into(), |c| format!("exit code {c}"))
        ),
        _ => said.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_dismissed_prompt_is_named_and_a_silent_failure_says_what_failed() {
        assert_eq!(
            failure(Some(126), "", "installing the helper"),
            "the authorisation prompt was dismissed"
        );
        assert_eq!(
            failure(Some(1), "  ", "installing the helper"),
            "installing the helper failed (exit code 1)"
        );
        assert_eq!(failure(Some(1), "no systemd\n", "x"), "no systemd");
    }
}
