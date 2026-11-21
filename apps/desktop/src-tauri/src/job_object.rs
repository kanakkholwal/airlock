//! Keeps child processes from outliving the app.
//!
//! Decision (architecture): engines (chat, image, voice) run as their own processes. If Airlock
//! is killed, a normal child would keep running with its model in memory and its folder open.
//! On Windows every child is put in one job object that the system closes with the app, and
//! closing it ends them all.

#[cfg(windows)]
mod imp {
    use std::ffi::c_void;
    use std::sync::OnceLock;
    use windows::Win32::Foundation::HANDLE;
    use windows::Win32::System::JobObjects::{
        AssignProcessToJobObject, CreateJobObjectW, JobObjectExtendedLimitInformation,
        SetInformationJobObject, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
        JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
    };

    struct Job(HANDLE);
    // The handle is only used to assign processes and is never closed until the app exits.
    unsafe impl Send for Job {}
    unsafe impl Sync for Job {}

    fn job() -> Option<&'static Job> {
        static JOB: OnceLock<Option<Job>> = OnceLock::new();
        JOB.get_or_init(|| unsafe {
            let handle = CreateJobObjectW(None, None).ok()?;
            let mut info = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
            info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
            SetInformationJobObject(
                handle,
                JobObjectExtendedLimitInformation,
                &info as *const _ as *const c_void,
                std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            )
            .ok()?;
            Some(Job(handle))
        })
        .as_ref()
    }

    pub fn bind(child: &tokio::process::Child) {
        let (Some(job), Some(process)) = (job(), child.raw_handle()) else {
            return;
        };
        // Failing to bind only means the child can outlive the app, as before.
        if let Err(e) = unsafe { AssignProcessToJobObject(job.0, HANDLE(process as *mut c_void)) } {
            log::warn!("Could not tie a helper process to the app: {e}");
        }
    }
}

/// Ties `child` to this app, so it is ended with it. A no-op outside Windows.
pub fn bind(child: &tokio::process::Child) {
    #[cfg(windows)]
    imp::bind(child);
    #[cfg(not(windows))]
    let _ = child;
}
