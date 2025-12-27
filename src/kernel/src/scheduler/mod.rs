//! Access for scheduler.

/// Scheduler.
pub mod executor;

/// Unsafe percore manipulation.
mod percore;

/// `OnceLock`-like.
mod sync;

/// Light Rust futures.
pub mod task;

use crate::scheduler::percore::PerCore;
use crate::scheduler::sync::OnceLock;

pub static SCHEDULER: OnceLock<PerCore<executor::Executor>> = OnceLock::new();

/// Inits per-core scheduler.
pub fn init_scheduler() {
    let cores = crate::arch::sysinfo().cores as usize;
    let _ = SCHEDULER.set(PerCore::new(cores));
    log::info!("{cores} schedulers initialized");
}
