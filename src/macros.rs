/// Print to console.
#[cfg(target_os = "none")]
#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ({
        use core::fmt::Write;
        crate::arch::console::WRITER.lock().get_or_insert_default().write_fmt((format_args!($($arg)*))).unwrap()
    });
}
