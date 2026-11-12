use std::ffi::{c_char, c_void};

pub(super) const K_CF_NUMBER_INT_TYPE: isize = 9;
pub(super) const K_CF_STRING_ENCODING_UTF8: u32 = 0x08000100;
pub(super) const K_IO_HID_EVENT_TYPE_TEMPERATURE: i64 = 15;
pub(super) const IOHID_EVENT_FIELD_TEMPERATURE: i32 = 15 << 16;

#[link(name = "IOKit", kind = "framework")]
extern "C" {
    pub(super) fn IOHIDEventSystemClientCreate(alloc: *const c_void) -> *mut c_void;
    pub(super) fn IOHIDEventSystemClientSetMatching(client: *mut c_void, match_dict: *const c_void);
    pub(super) fn IOHIDEventSystemClientCopyServices(client: *mut c_void) -> *mut c_void;
    pub(super) fn IOHIDServiceClientCopyProperty(
        svc: *mut c_void,
        prop: *const c_void,
    ) -> *mut c_void;
    pub(super) fn IOHIDServiceClientCopyEvent(
        s: *mut c_void,
        t: i64,
        opt: u32,
        fl: i64,
    ) -> *mut c_void;
    pub(super) fn IOHIDEventGetFloatValue(event: *mut c_void, field: i32) -> f64;
}

#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    pub(super) fn CFNumberCreate(alloc: *const c_void, t: isize, val: *const c_void)
        -> *mut c_void;
    pub(super) fn CFStringCreateWithCString(
        a: *const c_void,
        s: *const c_char,
        enc: u32,
    ) -> *mut c_void;
    pub(super) fn CFStringGetCString(s: *const c_void, b: *mut c_char, sz: isize, enc: u32)
        -> bool;
    pub(super) fn CFDictionaryCreate(
        a: *const c_void,
        k: *const *const c_void,
        v: *const *const c_void,
        n: isize,
        kc: *const c_void,
        vc: *const c_void,
    ) -> *mut c_void;
    pub(super) fn CFArrayGetCount(arr: *const c_void) -> isize;
    pub(super) fn CFArrayGetValueAtIndex(arr: *const c_void, idx: isize) -> *const c_void;
    pub(super) fn CFRelease(cf: *const c_void);

    pub(super) fn objc_getClass(name: *const c_char) -> *mut c_void;
    pub(super) fn sel_registerName(name: *const c_char) -> *mut c_void;
    pub(super) fn objc_msgSend();
}

pub(super) unsafe fn create_thermal_matching_dict() -> (*mut c_void, [*mut c_void; 5]) {
    let (mut page, mut usage) = (0xff00i32, 5i32);
    let page_num = CFNumberCreate(
        std::ptr::null(),
        K_CF_NUMBER_INT_TYPE,
        &mut page as *mut _ as _,
    );
    let usage_num = CFNumberCreate(
        std::ptr::null(),
        K_CF_NUMBER_INT_TYPE,
        &mut usage as *mut _ as _,
    );
    let k_page = CFStringCreateWithCString(
        std::ptr::null(),
        c"PrimaryUsagePage".as_ptr(),
        K_CF_STRING_ENCODING_UTF8,
    );
    let k_usage = CFStringCreateWithCString(
        std::ptr::null(),
        c"PrimaryUsage".as_ptr(),
        K_CF_STRING_ENCODING_UTF8,
    );
    let k_prod = CFStringCreateWithCString(
        std::ptr::null(),
        c"Product".as_ptr(),
        K_CF_STRING_ENCODING_UTF8,
    );

    let keys: [*const c_void; 2] = [k_page, k_usage];
    let values: [*const c_void; 2] = [page_num, usage_num];
    let dict = CFDictionaryCreate(
        std::ptr::null(),
        keys.as_ptr(),
        values.as_ptr(),
        2,
        std::ptr::null(),
        std::ptr::null(),
    );
    (dict, [page_num, usage_num, k_page, k_usage, k_prod])
}
