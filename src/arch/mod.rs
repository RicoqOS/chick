#[cfg(target_arch = "x86_64")]
#[path = "x86_64/mod.rs"]
mod api;

pub use api::*;

#[cfg(not(any(target_arch = "x86_64")))]
panic!("unsupported target architecture");
