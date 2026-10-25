//! The app's own log, `blkbstr.log` in the per-user log directory. Everything the GUI does outside
//! the daemon (downloads, UAC prompts, service and update calls) happens here and nowhere else, so
//! without it a failed step leaves nothing to read.

use blkbstr_core::paths;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

/// Keep the guard alive for the life of the app; dropping it stops the writer thread.
pub fn init() -> Option<WorkerGuard> {
    let dir = paths::app_log_dir()?;
    std::fs::create_dir_all(&dir).ok()?;
    let (writer, guard) =
        tracing_appender::non_blocking(tracing_appender::rolling::daily(&dir, "blkbstr.log"));
    let filter = EnvFilter::try_from_env("BLKBSTR_LOG").unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::registry()
        .with(filter)
        .with(fmt::layer().with_ansi(false).with_writer(writer))
        .init();

    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        tracing::error!(panic = %info, "the app panicked");
        previous(info);
    }));
    tracing::info!(
        version = env!("CARGO_PKG_VERSION"),
        os = std::env::consts::OS,
        "blkbstr started"
    );
    Some(guard)
}

/// Logs a failed command on its way back to the UI, so the toast the user sees has a record.
pub fn logged<T>(what: &str, result: Result<T, String>) -> Result<T, String> {
    if let Err(e) = &result {
        tracing::error!(error = %e, "{what} failed");
    }
    result
}
