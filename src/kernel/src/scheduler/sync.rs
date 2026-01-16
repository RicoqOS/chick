use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicU8, Ordering};

const UNINITIALIZED: u8 = 0;
const INITIALIZING: u8 = 1;
const INITIALIZED: u8 = 2;

struct Guard<'a> {
    state: &'a AtomicU8,
    active: bool,
}

impl<'a> Drop for Guard<'a> {
    fn drop(&mut self) {
        if self.active {
            self.state.store(UNINITIALIZED, Ordering::Release);
        }
    }
}

/// A synchronization primitive which can nominally be written to only once.
pub struct OnceLock<T> {
    once: AtomicU8,
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
            once: AtomicU8::new(UNINITIALIZED),
            value: UnsafeCell::new(None),
        }
    }

    /// Gets the contents of the cell, initializing it to `f()` if the cell was
    /// uninitialized.
    ///
    /// Many threads may call get_or_init concurrently with different
    /// initializing functions, but it is guaranteed that only one function
    /// will be executed if the function doesn’t panic.
    ///
    /// # Panics
    /// If `f()` panics, the panic is propagated to the caller, and the cell
    /// remains uninitialized.
    ///
    /// It is an error to reentrantly initialize the cell from f. The exact
    /// outcome is unspecified. Current implementation deadlocks, but this may
    /// be changed to a panic in the future.
    #[inline]
    pub fn get_or_init<F>(&self, f: F) -> &T
    where
        F: FnOnce() -> T,
    {
        // Fast path.
        if self.once.load(Ordering::Acquire) == INITIALIZED {
            return unsafe { (*self.value.get()).as_ref().unwrap() };
        }

        if self
            .once
            .compare_exchange(
                UNINITIALIZED,
                INITIALIZING,
                Ordering::AcqRel,
                Ordering::Acquire,
            )
            .is_ok()
        {
            let mut guard = Guard {
                state: &self.once,
                active: true,
            };

            let value = f();

            unsafe {
                *self.value.get() = Some(value);
            }

            guard.active = false;
            self.once.store(INITIALIZED, Ordering::Release);

            return unsafe { (*self.value.get()).as_ref().unwrap() };
        }

        // Cooperative wait.
        while self.once.load(Ordering::Acquire) == INITIALIZING {
            core::hint::spin_loop();
        }

        if self.once.load(Ordering::Acquire) == INITIALIZED {
            unsafe { (*self.value.get()).as_ref().unwrap() }
        } else {
            self.get_or_init(f)
        }
    }

    /// Initializes the contents of the cell to `value`.
    pub fn set(&self, value: T) -> Result<(), ()> {
        if self
            .once
            .compare_exchange(
                UNINITIALIZED,
                INITIALIZING,
                Ordering::AcqRel,
                Ordering::Acquire,
            )
            .is_err()
        {
            return Err(());
        }

        let mut guard = Guard {
            state: &self.once,
            active: true,
        };

        unsafe {
            *self.value.get() = Some(value);
        }

        guard.active = false;
        self.once.store(INITIALIZED, Ordering::Release);
        Ok(())
    }

    /// Gets the reference to the underlying value.
    ///
    /// Returns `None` if the cell is uninitialized, or being initialized.
    /// This method never blocks.
    #[inline]
    pub fn get(&self) -> Option<&T> {
        if self.once.load(Ordering::Acquire) == INITIALIZED {
            unsafe { (*self.value.get()).as_ref() }
        } else {
            None
        }
    }
}
