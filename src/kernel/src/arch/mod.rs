#[cfg(target_arch = "x86_64")]
mod x86_64;
#[cfg(target_arch = "x86_64")]
pub use x86_64::*;

#[cfg(not(any(target_arch = "x86_64")))]
panic!("unsupported target architecture");
