//! Types shared by the unprivileged GUI (`src-tauri`) and the privileged daemon (`blkbstrd`).

pub mod candidates;
pub mod config;
pub mod detect;
pub mod paths;
pub mod protocol;
pub mod reachability;
pub mod registry;
pub mod render;

pub use config::{Action, Config, Filter, Strategy};
pub use detect::Environment;
pub use protocol::{EngineStatus, Request, Response};
pub use registry::{Platform, Warning};
