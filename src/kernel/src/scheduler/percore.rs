use alloc::boxed::Box;
use alloc::vec::Vec;
use core::cell::UnsafeCell;

use crate::arch::cpuid;

#[derive(Debug, Default)]
pub struct PerCore<T> {
    cores: Box<[UnsafeCell<T>]>,
}

unsafe impl<T> Sync for PerCore<T> {}

impl<T> PerCore<T> {
    /// Create [`PerCore`] with `n` cores.
    pub fn new_with<F>(n: usize, mut init: F) -> Self
    where
        F: FnMut() -> T,
    {
        let mut v = Vec::with_capacity(n);
        for _ in 0..n {
            v.push(UnsafeCell::new(init()));
        }
        let cores = v.into_boxed_slice();
        Self { cores }
    }

    /// Create default `T` with `n` cores.
    pub fn new(n: usize) -> Self
    where
        T: Default,
    {
        Self::new_with(n, T::default)
    }

    /// Unsafe access to a core scheduler.
    pub unsafe fn get_unsafe(&self, i: usize) -> &T {
        unsafe { &*self.cores[i].get() }
    }

    /// Access to current core scheduler.
    pub fn get(&self) -> &T {
        let i = cpuid().try_into().unwrap_or(0);
        unsafe { &*self.cores[i].get() }
    }

    /// Mutable access to current core scheduler.
    #[allow(clippy::mut_from_ref)]
    pub fn get_mut(&self) -> &mut T {
        let i = cpuid().try_into().unwrap_or(0);
        unsafe { &mut *self.cores[i].get() }
    }

    /// Number of cores.
    pub fn len(&self) -> usize {
        self.cores.len()
    }
}
