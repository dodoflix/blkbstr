//! Ties the engine's life to the daemon's. A job object with `KILL_ON_JOB_CLOSE` kills its
//! processes when the last handle to it closes, which includes the daemon being killed outright.

use std::os::windows::io::AsRawHandle;
use std::process::Child;
use std::sync::OnceLock;
use windows_sys::Win32::Foundation::HANDLE;
use windows_sys::Win32::System::JobObjects::{
    AssignProcessToJobObject, CreateJobObjectW, JobObjectExtendedLimitInformation,
    SetInformationJobObject, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
    JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
};

struct Job(HANDLE);

// SAFETY: a job handle is a kernel object reference with no thread affinity.
unsafe impl Send for Job {}
unsafe impl Sync for Job {}

static JOB: OnceLock<Option<Job>> = OnceLock::new();

fn create() -> Option<Job> {
    // SAFETY: plain Win32 calls; `info` is zero-initialised, which is a valid "no limits" value.
    unsafe {
        let job = CreateJobObjectW(std::ptr::null(), std::ptr::null());
        if job.is_null() {
            return None;
        }
        let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
        info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        let ok = SetInformationJobObject(
            job,
            JobObjectExtendedLimitInformation,
            std::ptr::addr_of!(info).cast(),
            std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
        );
        (ok != 0).then_some(Job(job))
    }
}

/// Puts the engine in the daemon's job. Failure is logged, not fatal: the engine still runs, it
/// just no longer dies with the daemon.
pub fn adopt(child: &Child) {
    let Some(job) = JOB.get_or_init(create) else {
        tracing::warn!("could not create a job object; the engine will outlive a killed daemon");
        return;
    };
    // SAFETY: both handles are valid for the duration of the call.
    let ok = unsafe { AssignProcessToJobObject(job.0, child.as_raw_handle() as HANDLE) };
    if ok == 0 {
        tracing::warn!(
            error = %std::io::Error::last_os_error(),
            "could not add the engine to the job object; it will outlive a killed daemon"
        );
    }
}
