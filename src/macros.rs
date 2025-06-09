/// Print to console.
#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ({
        use core::fmt::Write;
        crate::console::WRITER.lock().get_or_insert_default().write_fmt((format_args!($($arg)*))).unwrap()
    });
}
