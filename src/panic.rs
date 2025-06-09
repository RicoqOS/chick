use core::panic::PanicInfo;
use x86_64::structures::idt::InterruptStackFrame;

pub(crate) extern "x86-interrupt" fn breakpoint_handler(stack_frame: InterruptStackFrame) {
    print!("\n");
    print!("CPU INTERUPT: {stack_frame:?}");
}

/// Handle panics.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    print!("\n");
    print!("KERNEL PANIC: {info:?}");
    loop {}
}
