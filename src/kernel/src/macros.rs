#[macro_export]
macro_rules! bit {
    ($x:expr) => {
        1 << $x
    };
}

#[macro_export]
macro_rules! mask {
    ($x:expr) => {
        $crate::bit!($x) - 1
    };
}

#[macro_export]
macro_rules! symbol {
    ($x:expr) => {
        unsafe { ($x).as_ptr() as usize }
    };
}

#[macro_export]
macro_rules! alignup {
    ($addr:expr, $sz: expr) => {
        (($addr) + $crate::bit!($sz) - 1) & ($crate::mask!($sz))
    };
}
