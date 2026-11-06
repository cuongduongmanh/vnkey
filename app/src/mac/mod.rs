mod ffi;
mod login;
mod menu;
mod tap;

pub fn run() {
    // Hiện hộp thoại xin quyền Accessibility nếu chưa có.
    let trusted = unsafe { ffi::ax_trusted_prompt() };
    let tap_ok = tap::start();
    if !trusted || !tap_ok {
        eprintln!(
            "vnkey: chưa có quyền. Vào System Settings > Privacy & Security > Accessibility, \
             bật VnKey rồi mở lại app."
        );
    }
    menu::run(tap_ok);
}

/// Bật/tắt tiếng Việt (từ menu hoặc phím tắt Ctrl+Shift).
pub fn toggle() {
    let enabled = tap::toggle();
    menu::set_title(if enabled { "V" } else { "E" });
}
