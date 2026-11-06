//! Tự khởi động cùng máy qua SMAppService (macOS 13+).

use objc::runtime::{Class, Object, BOOL, NO};
use objc::{msg_send, sel, sel_impl};
use std::ptr::null_mut;

type Id = *mut Object;

// ServiceManagement có từ macOS 10.6; lớp SMAppService chỉ có từ macOS 13.
#[link(name = "ServiceManagement", kind = "framework")]
extern "C" {}

// SMAppServiceStatus
const STATUS_ENABLED: isize = 1;
const STATUS_REQUIRES_APPROVAL: isize = 2;

fn main_service() -> Option<Id> {
    let cls = Class::get("SMAppService")?;
    let svc: Id = unsafe { msg_send![cls, mainAppService] };
    (!svc.is_null()).then_some(svc)
}

/// macOS có hỗ trợ không (13+).
pub fn supported() -> bool {
    main_service().is_some()
}

pub fn is_enabled() -> bool {
    main_service().map_or(false, |svc| {
        let status: isize = unsafe { msg_send![svc, status] };
        status == STATUS_ENABLED
    })
}

/// Bật/tắt, trả về trạng thái mới.
pub fn set_enabled(on: bool) -> bool {
    let Some(svc) = main_service() else { return false };
    let mut err: Id = null_mut();
    let ok: BOOL = unsafe {
        if on {
            msg_send![svc, registerAndReturnError: &mut err]
        } else {
            msg_send![svc, unregisterAndReturnError: &mut err]
        }
    };
    if ok == NO {
        eprintln!("vnkey: không đổi được trạng thái tự khởi động");
    }
    let status: isize = unsafe { msg_send![svc, status] };
    if status == STATUS_REQUIRES_APPROVAL {
        // Người dùng cần duyệt trong System Settings > General > Login Items
        let cls = Class::get("SMAppService").unwrap();
        unsafe {
            let _: () = msg_send![cls, openSystemSettingsLoginItems];
        }
    }
    status == STATUS_ENABLED
}
