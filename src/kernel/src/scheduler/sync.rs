use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicBool, Ordering};

/// A synchronization primitive which can nominally be written to only once.
pub struct OnceLock<T> {
    once: AtomicBool,
    value: UnsafeCell<Option<T>>,
}

unsafe impl<T: Sync> Sync for OnceLock<T> {}
unsafe impl<T: Send> Send for OnceLock<T> {}

impl<T> OnceLock<T> {
    /// Creates a new uninitialized cell.
    #[inline]
    #[must_use]
    pub const fn new() -> Self {
        Self {
            once: AtomicBool::new(false),
            value: UnsafeCell::new(None),
        }
    }

    /// Initializes the contents of the cell to `value`.
    #[inline]
    pub fn set(&self, value: T) -> Result<(), ()> {
        if self.once.swap(true, Ordering::AcqRel) {
            return Err(());
        }

        unsafe {
            *self.value.get() = Some(value);
        }

        Ok(())
    }

    /// Gets the reference to the underlying value.
    #[inline]
    pub fn get(&self) -> Option<&T> {
        unsafe { (*self.value.get()).as_ref() }
    }
}
