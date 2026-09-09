//! CPU-time sampling, used to tell real work apart from a CPU-starved host.

use std::time::Duration;

/// Whole-process CPU time (user + system, summed over every thread) consumed
/// since the process started.
///
/// Sample it before and after a section; the delta is how much CPU that section
/// actually burned. Compared with wall-clock it exposes contention: if a parse
/// takes 12 s of wall time but only 0.6 s of CPU, the box was starved — the work
/// is cheap, the wait is not.
///
/// Because it is process-wide it also counts other requests parsing at the same
/// time, so under heavy concurrency treat it as an upper bound.
pub fn process_cpu_time() -> Duration {
    #[cfg(unix)]
    {
        // SAFETY: `getrusage` only writes into the provided `rusage`, and a
        // zeroed `rusage` is a valid value to pass.
        unsafe {
            let mut usage: libc::rusage = std::mem::zeroed();
            if libc::getrusage(libc::RUSAGE_SELF, &mut usage) == 0 {
                return timeval_to_duration(usage.ru_utime) + timeval_to_duration(usage.ru_stime);
            }
        }
    }
    Duration::ZERO
}

#[cfg(unix)]
fn timeval_to_duration(tv: libc::timeval) -> Duration {
    Duration::new(tv.tv_sec.max(0) as u64, 0) + Duration::from_micros(tv.tv_usec.max(0) as u64)
}
