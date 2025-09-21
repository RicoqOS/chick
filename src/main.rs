//! Modified version of [bootloader example](https://github.com/rust-osdev/bootloader/blob/main/docs/create-disk-image.md).

fn main() {
    // read env variables that were set in build script
    let uefi_path = env!("UEFI_PATH");
    let bios_path = env!("BIOS_PATH");

    // choose whether to start the UEFI or BIOS image
    let uefi = std::env::var("UEFI").is_ok();

    let mut cmd = std::process::Command::new("qemu-system-x86_64");
    if uefi {
        cmd.arg("-bios").arg(ovmf_prebuilt::ovmf_pure_efi());
        cmd.arg("-drive")
            .arg(format!("format=raw,file={uefi_path}"));
    } else {
        cmd.arg("-drive")
            .arg(format!("format=raw,file={bios_path}"));
    }

    // Enable serial output.
    cmd.arg("-serial").arg("stdio");

    cmd.arg("-smp").arg("cores=4");

    let mut child = cmd.spawn().unwrap();
    child.wait().unwrap();
}
