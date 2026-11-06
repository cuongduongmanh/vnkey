#[cfg(target_os = "macos")]
mod mac;

fn main() {
    #[cfg(target_os = "macos")]
    mac::run();

    #[cfg(not(target_os = "macos"))]
    eprintln!("vnkey chỉ chạy trên macOS. Test engine bằng: cargo test -p vnkey-engine");
}
