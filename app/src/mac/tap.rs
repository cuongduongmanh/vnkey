//! Chặn phím toàn hệ thống bằng CGEventTap và thay chữ bằng Backspace + Unicode.

use super::ffi::*;
use std::cell::RefCell;
use std::ffi::c_void;
use std::ptr::{null, null_mut};
use std::sync::atomic::{AtomicPtr, Ordering};
use vnkey_engine::{Action, Engine};

/// Đánh dấu các sự kiện do chính app gửi ra để không xử lý lại.
const MAGIC: i64 = 0x56_4E_4B; // "VNK"

static TAP: AtomicPtr<c_void> = AtomicPtr::new(null_mut());

struct State {
    engine: Engine,
    enabled: bool,
    /// Đang giữ Ctrl+Shift mà chưa gõ phím nào khác.
    hotkey_pending: bool,
    source: CGEventSourceRef,
}

thread_local! {
    // Callback của event tap và menu đều chạy trên main thread.
    static STATE: RefCell<State> = RefCell::new(State {
        engine: Engine::new(),
        enabled: true,
        hotkey_pending: false,
        source: null_mut(),
    });
}

pub fn start() -> bool {
    unsafe {
        let source = CGEventSourceCreate(kCGEventSourceStatePrivate);
        STATE.with(|s| s.borrow_mut().source = source);

        let mask: u64 = (1 << kCGEventKeyDown)
            | (1 << kCGEventFlagsChanged)
            | (1 << kCGEventLeftMouseDown)
            | (1 << kCGEventRightMouseDown)
            | (1 << kCGEventOtherMouseDown);
        let tap = CGEventTapCreate(
            kCGSessionEventTap,
            kCGHeadInsertEventTap,
            kCGEventTapOptionDefault,
            mask,
            callback,
            null_mut(),
        );
        if tap.is_null() {
            return false;
        }
        TAP.store(tap, Ordering::SeqCst);
        let run_loop_source = CFMachPortCreateRunLoopSource(null(), tap, 0);
        CFRunLoopAddSource(CFRunLoopGetMain(), run_loop_source, kCFRunLoopCommonModes);
        CGEventTapEnable(tap, true);
        true
    }
}

/// Đảo trạng thái bật/tắt, trả về trạng thái mới.
pub fn toggle() -> bool {
    STATE.with(|s| {
        let mut s = s.borrow_mut();
        s.enabled = !s.enabled;
        s.engine.reset();
        s.enabled
    })
}

extern "C" fn callback(proxy: CGEventTapProxy, etype: u32, event: CGEventRef, _: *mut c_void) -> CGEventRef {
    unsafe {
        match etype {
            kCGEventTapDisabledByTimeout | kCGEventTapDisabledByUserInput => {
                let tap = TAP.load(Ordering::SeqCst);
                if !tap.is_null() {
                    CGEventTapEnable(tap, true);
                }
                event
            }
            _ if CGEventGetIntegerValueField(event, kCGEventSourceUserData) == MAGIC => event,
            kCGEventKeyDown => on_key_down(proxy, event),
            kCGEventFlagsChanged => {
                if on_flags_changed(CGEventGetFlags(event)) {
                    super::toggle();
                }
                event
            }
            _ => {
                // Click chuột: con trỏ có thể đã nhảy chỗ khác.
                STATE.with(|s| s.borrow_mut().engine.reset());
                event
            }
        }
    }
}

unsafe fn on_key_down(proxy: CGEventTapProxy, event: CGEventRef) -> CGEventRef {
    STATE.with(|s| {
        let mut s = s.borrow_mut();
        s.hotkey_pending = false;
        if !s.enabled {
            return event;
        }
        let flags = CGEventGetFlags(event);
        if flags & (kCGEventFlagMaskCommand | kCGEventFlagMaskControl | kCGEventFlagMaskAlternate) != 0 {
            s.engine.reset();
            return event;
        }
        let keycode = CGEventGetIntegerValueField(event, kCGKeyboardEventKeycode) as u16;
        if keycode == KEY_DELETE {
            s.engine.backspace();
            return event;
        }
        let Some(ch) = event_char(event) else {
            s.engine.reset();
            return event;
        };
        match s.engine.process(ch) {
            Action::PassThrough => event,
            Action::Replace { backspaces, text } => {
                send(proxy, s.source, keycode, backspaces, &text);
                null_mut() // nuốt phím gốc
            }
        }
    })
}

/// Phím tắt kiểu Unikey: nhấn rồi thả Ctrl+Shift (không gõ phím nào khác).
/// Trả về `true` nếu cần bật/tắt.
fn on_flags_changed(flags: u64) -> bool {
    let ctrl_shift = kCGEventFlagMaskControl | kCGEventFlagMaskShift;
    let mods = flags & (ctrl_shift | kCGEventFlagMaskAlternate | kCGEventFlagMaskCommand);
    STATE.with(|s| {
        let mut s = s.borrow_mut();
        if mods == ctrl_shift {
            s.hotkey_pending = true;
            false
        } else if mods == 0 {
            std::mem::take(&mut s.hotkey_pending)
        } else {
            if mods & !ctrl_shift != 0 {
                s.hotkey_pending = false;
            }
            false
        }
    })
}

unsafe fn event_char(event: CGEventRef) -> Option<char> {
    let mut buf = [0u16; 4];
    let mut len: usize = 0;
    CGEventKeyboardGetUnicodeString(event, buf.len(), &mut len, buf.as_mut_ptr());
    if len != 1 {
        return None;
    }
    char::from_u32(buf[0] as u32)
}

/// `keycode`: mã phím vật lý vừa bị nuốt (f, s, w...). Chữ thay thế được gửi kèm mã phím này
/// thay vì 0: mã 0 là phím `a`, nếu ngón tay còn đang giữ `a` (gõ nhanh: `laf`, `quar`, `lawms`)
/// thì app thấy thêm một lần nhấn/nhả `a`, lệch trạng thái phím và xoá thừa một chữ (là -> à).
unsafe fn send(proxy: CGEventTapProxy, source: CGEventSourceRef, keycode: u16, backspaces: usize, text: &str) {
    // Giống "Fix recommend browser" của OpenKey: gõ trước một ký tự trống (U+202F) rồi xoá
    // thêm 1 lần. Ký tự này "hứng" gợi ý autocomplete/predictive text mà app đang giữ,
    // để Backspace không xoá nhầm chữ phía trước khi gõ nhanh (lắm -> ắm).
    post_key(proxy, source, keycode, Some(&[0x202F]));
    for _ in 0..backspaces + 1 {
        post_key(proxy, source, KEY_DELETE, None);
    }
    // Gửi cả chuỗi trong một sự kiện (như OpenKey) thay vì từng ký tự: ít sự kiện hơn,
    // app Chromium/Electron không bị lệch thứ tự khi gõ nhanh (mất/thừa chữ).
    // macOS giới hạn khoảng 20 UTF-16 unit cho mỗi sự kiện.
    let units: Vec<u16> = text.encode_utf16().collect();
    for chunk in units.chunks(20) {
        post_key(proxy, source, keycode, Some(chunk));
    }
}

unsafe fn post_key(proxy: CGEventTapProxy, source: CGEventSourceRef, keycode: u16, text: Option<&[u16]>) {
    for down in [true, false] {
        let ev = CGEventCreateKeyboardEvent(source, keycode, down);
        if ev.is_null() {
            continue;
        }
        if let Some(units) = text {
            CGEventKeyboardSetUnicodeString(ev, units.len(), units.as_ptr());
        }
        CGEventSetFlags(ev, 0); // bỏ Shift đang giữ để Backspace không thành Shift+Delete
        CGEventSetIntegerValueField(ev, kCGEventSourceUserData, MAGIC);
        CGEventTapPostEvent(proxy, ev);
        CFRelease(ev);
    }
}
