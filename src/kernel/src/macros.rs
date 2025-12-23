#[macro_export]
macro_rules! bit {
    ($x:expr) => {
        1 << $x
    };
}

#[macro_export]
macro_rules! mask {
    ($x:expr) => {
        !bit!($x) - 1
    };
}

#[macro_export]
macro_rules! symbol {
    ($x:expr) => {
        unsafe { ($x).as_ptr() as usize }
    };
}

#[macro_export]
macro_rules! ALIGNUP {
    ($addr:expr, $sz: expr) => {
        (($addr) + !bit!($sz) - 1) & (!mask!($sz))
    };
}
