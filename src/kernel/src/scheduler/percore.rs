use core::mem::MaybeUninit;
use core::sync::atomic::{AtomicUsize, Ordering};

pub const MAX_CPUS: usize = 16;

use crate::arch::cpuid;

#[derive(Debug)]
pub struct PerCore<T> {
    cores: [MaybeUninit<T>; MAX_CPUS],
    initialized_count: AtomicUsize,
}

unsafe impl<T> Sync for PerCore<T> {}

impl<T> PerCore<T> {
    /// Init one core on [`PerCore`].
    fn init_core<F>(&self, cpu_id: usize, mut init: F)
    where
        F: FnMut() -> T,
    {
        if cpu_id + 1 > MAX_CPUS {
            log::warn!("core {cpu_id} outbound max cores ({MAX_CPUS})");
            return;
        }

        unsafe {
            let ptr = self.cores[cpu_id].as_ptr() as *mut T;
            ptr.write(init());
        }
        self.initialized_count.fetch_add(1, Ordering::Release);
    }

    /// Create default `T` with `n` cores.
    pub fn new(n: usize) -> Self
    where
        T: Default,
    {
        let percore = Self {
            cores: [const { MaybeUninit::uninit() }; MAX_CPUS],
            initialized_count: AtomicUsize::new(0),
        };
        for i in 0..n {
            percore.init_core(i, T::default);
        }
        percore
    }

    /// Unsafe access to a core scheduler.
    pub unsafe fn get_unsafe(&self, i: usize) -> &T {
        unsafe { self.cores[i].assume_init_ref() }
    }

    /// Access to current core scheduler.
    pub fn get(&self) -> &T {
        let i = cpuid().try_into().unwrap_or(0);
        unsafe { self.cores[i].assume_init_ref() }
    }

    /// Mutable access to current core scheduler.
    #[allow(clippy::mut_from_ref)]
    pub fn get_mut(&self) -> &mut T {
        let i = cpuid().try_into().unwrap_or(0);
        unsafe { &mut *(self.cores[i].as_ptr() as *mut T) }
    }
}
