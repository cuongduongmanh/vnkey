//! Icon V/E trên menu bar (AppKit qua Objective-C runtime).

use objc::declare::ClassDecl;
use objc::runtime::{Class, Object, Sel, BOOL};
use objc::{class, msg_send, sel, sel_impl};
use std::ffi::CString;
use std::ptr::null_mut;
use std::sync::atomic::{AtomicPtr, Ordering};

type Id = *mut Object;

static BUTTON: AtomicPtr<Object> = AtomicPtr::new(null_mut());

fn nsstring(s: &str) -> Id {
    let c = CString::new(s).unwrap_or_default();
    unsafe { msg_send![class!(NSString), stringWithUTF8String: c.as_ptr()] }
}

pub fn set_title(title: &str) {
    let button = BUTTON.load(Ordering::SeqCst);
    if !button.is_null() {
        unsafe {
            let _: () = msg_send![button, setTitle: nsstring(title)];
        }
    }
}

/// Lớp Objective-C nhận sự kiện click menu.
fn controller_class() -> &'static Class {
    extern "C" fn toggle(_this: &Object, _cmd: Sel, _sender: Id) {
        super::toggle();
    }
    let mut decl = ClassDecl::new("VnKeyController", class!(NSObject)).expect("class đã tồn tại");
    unsafe {
        decl.add_method(sel!(toggle:), toggle as extern "C" fn(&Object, Sel, Id));
    }
    decl.register()
}

unsafe fn add_item(menu: Id, title: &str, action: Sel, target: Id, key: &str) {
    let item: Id = msg_send![class!(NSMenuItem), alloc];
    let item: Id = msg_send![item, initWithTitle: nsstring(title) action: action keyEquivalent: nsstring(key)];
    let _: () = msg_send![item, setTarget: target];
    let _: () = msg_send![menu, addItem: item];
}

pub fn run(tap_ok: bool) -> ! {
    unsafe {
        let app: Id = msg_send![class!(NSApplication), sharedApplication];
        // NSApplicationActivationPolicyAccessory: không hiện icon Dock
        let _: BOOL = msg_send![app, setActivationPolicy: 1isize];

        let bar: Id = msg_send![class!(NSStatusBar), systemStatusBar];
        let status_item: Id = msg_send![bar, statusItemWithLength: -1.0f64];
        let _: Id = msg_send![status_item, retain];
        let button: Id = msg_send![status_item, button];
        BUTTON.store(button, Ordering::SeqCst);
        set_title(if tap_ok { "V" } else { "⚠" });

        let menu: Id = msg_send![class!(NSMenu), new];
        let controller: Id = msg_send![controller_class(), new];
        if !tap_ok {
            // Không có target/action hợp lệ -> menu tự làm mờ dòng này
            add_item(menu, "Cần quyền Accessibility — cấp quyền rồi mở lại", sel!(vnkeyNoop:), null_mut(), "");
        }
        add_item(menu, "Bật/tắt tiếng Việt  (Ctrl+Shift)", sel!(toggle:), controller, "");
        let sep: Id = msg_send![class!(NSMenuItem), separatorItem];
        let _: () = msg_send![menu, addItem: sep];
        add_item(menu, "Thoát VnKey", sel!(terminate:), app, "q");
        let _: () = msg_send![status_item, setMenu: menu];

        let _: () = msg_send![app, run];
    }
    std::process::exit(0)
}
