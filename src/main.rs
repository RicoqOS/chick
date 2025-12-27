//! Modified version of [bootloader example](https://github.com/rust-osdev/bootloader/blob/main/docs/create-disk-image.md).

fn main() {
    // read env variables that were set in build script
    let uefi_path = env!("UEFI_PATH");

    let mut cmd = std::process::Command::new("qemu-system-x86_64");
    cmd.arg("-bios").arg(ovmf_prebuilt::ovmf_pure_efi());
    cmd.arg("-drive")
        .arg(format!("format=raw,file={uefi_path}"));

    // Enable serial output.
    cmd.arg("-serial").arg("stdio");

    cmd.arg("-m").arg("50M");
    cmd.arg("-smp").arg("cores=4");
    cmd.arg("-monitor")
        .arg("telnet:127.0.0.1:7000,server,nowait");

    let mut child = cmd.spawn().unwrap();
    child.wait().unwrap();
}
