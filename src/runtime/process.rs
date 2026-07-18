use std::process::{Child, ExitStatus};

use anyhow::Result;

#[derive(Debug)]
pub struct ProcessOutcome {
    pub status: ExitStatus,
    pub remaining_process_ids: Vec<u32>,
}

#[cfg(windows)]
pub fn supervise(
    spawn: impl FnOnce() -> Result<Child>,
    on_started: impl FnOnce(u32) -> Result<()>,
) -> Result<(u32, ProcessOutcome)> {
    use anyhow::Context;
    use win32job::{ExtendedLimitInfo, Job};

    struct ArmedJob {
        job: Option<Job>,
        armed: bool,
    }

    impl ArmedJob {
        fn disarm(&mut self) -> Result<()> {
            if self.armed {
                let job = self.job.take().expect("armed Job Object must be present");
                let mut limits = match job.query_extended_limit_info() {
                    Ok(limits) => limits,
                    Err(error) => {
                        std::mem::forget(job);
                        self.armed = false;
                        return Err(error.into());
                    }
                };
                limits.clear_limits();
                if let Err(error) = job.set_extended_limit_info(&limits) {
                    // Closing an armed job would kill this runtime process. Leak
                    // the handle on this exceptional path; Windows closes it
                    // when the process exits and still cleans up descendants.
                    std::mem::forget(job);
                    self.armed = false;
                    return Err(error.into());
                }
                self.armed = false;
                self.job = Some(job);
            }
            Ok(())
        }

        fn job(&self) -> &Job {
            self.job.as_ref().expect("Job Object must be present")
        }

        fn preserve_armed_until_process_exit(&mut self) {
            if let Some(job) = self.job.take() {
                std::mem::forget(job);
            }
            self.armed = false;
        }
    }

    impl Drop for ArmedJob {
        fn drop(&mut self) {
            if std::thread::panicking() {
                // Keep KILL_ON_JOB_CLOSE armed through process teardown. The
                // leaked handle is then closed by Windows, terminating the tree.
                self.preserve_armed_until_process_exit();
            } else {
                let _ = self.disarm();
            }
        }
    }

    let mut limits = ExtendedLimitInfo::new();
    limits.limit_kill_on_job_close();
    let job = Job::create_with_limit_info(&limits)
        .context("could not create a Windows process Job Object")?;
    job.assign_current_process().context(
        "could not assign the runtime to its Job Object; the emulator was not launched unsupervised",
    )?;
    let mut job = ArmedJob {
        job: Some(job),
        armed: true,
    };

    let mut child = match spawn() {
        Ok(child) => child,
        Err(error) => {
            job.disarm()?;
            return Err(error);
        }
    };
    let pid = child.id();
    if let Err(error) = on_started(pid) {
        let _ = child.kill();
        let _ = child.wait();
        job.preserve_armed_until_process_exit();
        return Err(error);
    }
    let status = match child.wait() {
        Ok(status) => status,
        Err(error) => {
            let _ = child.kill();
            let _ = child.wait();
            job.preserve_armed_until_process_exit();
            return Err(error).context("could not wait for the emulator process");
        }
    };

    let current_pid = std::process::id() as usize;
    loop {
        let process_ids = match job.job().query_process_id_list() {
            Ok(process_ids) => process_ids,
            Err(error) => {
                job.preserve_armed_until_process_exit();
                return Err(error).context("could not query the supervised emulator process tree");
            }
        };
        let remaining = process_ids
            .into_iter()
            .filter(|process_id| *process_id != current_pid)
            .collect::<Vec<_>>();
        if remaining.is_empty() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    let remaining_process_ids = match job.job().query_process_id_list() {
        Ok(process_ids) => process_ids
            .into_iter()
            .filter(|process_id| *process_id != current_pid)
            .map(|process_id| process_id as u32)
            .collect(),
        Err(error) => {
            job.preserve_armed_until_process_exit();
            return Err(error).context("could not query the final supervised process set");
        }
    };
    job.disarm()?;
    Ok((
        pid,
        ProcessOutcome {
            status,
            remaining_process_ids,
        },
    ))
}

#[cfg(not(windows))]
pub fn supervise(
    _spawn: impl FnOnce() -> Result<Child>,
    _on_started: impl FnOnce(u32) -> Result<()>,
) -> Result<(u32, ProcessOutcome)> {
    anyhow::bail!("process-tree supervision is supported only on Windows")
}
