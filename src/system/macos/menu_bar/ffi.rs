use std::ffi::{c_char, c_void, CStr, CString};
use std::sync::atomic::{AtomicBool, Ordering};

use crate::system::macos::MacosMemoryCollector;
use crate::system::memory::MemoryCollector;

use super::{dispatch_menu_bar_snapshot, request_show_window};

#[link(name = "AppKit", kind = "framework")]
extern "C" {
    pub(super) static NSApp: *mut c_void;
    pub(super) static _dispatch_main_q: c_void;
    pub(super) fn dispatch_async_f(
        queue: *const c_void,
        context: *mut c_void,
        work: extern "C" fn(*mut c_void),
    );
    pub(super) fn objc_getClass(name: *const c_char) -> *mut c_void;
    fn object_getClass(obj: *mut c_void) -> *mut c_void;
    pub(super) fn sel_registerName(name: *const c_char) -> *mut c_void;
    fn objc_allocateClassPair(sup: *mut c_void, name: *const c_char, extra: usize) -> *mut c_void;
    fn class_addMethod(
        cls: *mut c_void,
        name: *mut c_void,
        imp: *const (),
        types: *const c_char,
    ) -> bool;
    fn objc_registerClassPair(cls: *mut c_void);
    pub(super) fn objc_msgSend();
}

static TARGET_CLASS_REGISTERED: AtomicBool = AtomicBool::new(false);

pub(super) unsafe fn msg0(obj: *mut c_void, sel_str: &CStr) -> *mut c_void {
    let f: extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void =
        std::mem::transmute(objc_msgSend as *const ());
    f(obj, sel_registerName(sel_str.as_ptr()))
}

pub(super) unsafe fn msg1_ptr(obj: *mut c_void, sel_str: &CStr, arg: *mut c_void) {
    let f: extern "C" fn(*mut c_void, *mut c_void, *mut c_void) =
        std::mem::transmute(objc_msgSend as *const ());
    f(obj, sel_registerName(sel_str.as_ptr()), arg);
}

pub(super) unsafe fn msg1_bool(obj: *mut c_void, sel_str: &CStr, arg: bool) {
    let f: extern "C" fn(*mut c_void, *mut c_void, bool) =
        std::mem::transmute(objc_msgSend as *const ());
    f(obj, sel_registerName(sel_str.as_ptr()), arg);
}

pub(super) unsafe fn msg1_usize(obj: *mut c_void, sel_str: &CStr, arg: usize) -> *mut c_void {
    let f: extern "C" fn(*mut c_void, *mut c_void, usize) -> *mut c_void =
        std::mem::transmute(objc_msgSend as *const ());
    f(obj, sel_registerName(sel_str.as_ptr()), arg)
}

pub(super) unsafe fn ensure_target_class_registered() {
    if TARGET_CLASS_REGISTERED.swap(true, Ordering::SeqCst) {
        return;
    }
    let superclass = objc_getClass(c"NSObject".as_ptr());
    let new_class = objc_allocateClassPair(superclass, c"MemoraMenuBarTarget".as_ptr(), 0);
    if !new_class.is_null() {
        let methods: [(&CStr, *const ()); 4] = [
            (c"openDashboard:", open_dashboard_callback as *const ()),
            (c"purgeRamCache:", purge_ram_callback as *const ()),
            (c"emptyTrash:", empty_trash_callback as *const ()),
            (c"quitMemora:", quit_callback as *const ()),
        ];
        for (sel_str, imp) in methods {
            let sel = sel_registerName(sel_str.as_ptr());
            class_addMethod(new_class, sel, imp, c"v@:@".as_ptr());
        }
        objc_registerClassPair(new_class);
    }

    let app = shared_app();
    if !app.is_null() {
        let delegate = msg0(app, c"delegate");
        if !delegate.is_null() {
            let sel =
                sel_registerName(c"applicationShouldHandleReopen:hasVisibleWindows:".as_ptr());
            let imp = dock_reopen_callback as *const ();
            class_addMethod(object_getClass(delegate), sel, imp, c"B@:@B".as_ptr());
        }
    }
}

unsafe fn shared_app() -> *mut c_void {
    msg0(
        objc_getClass(c"NSApplication".as_ptr()),
        c"sharedApplication",
    )
}

extern "C" fn dock_reopen_callback(
    _t: *mut c_void,
    _c: *mut c_void,
    _a: *mut c_void,
    _vis: bool,
) -> bool {
    let z = std::ptr::null_mut();
    open_dashboard_callback(z, z, z);
    true
}

extern "C" fn open_dashboard_callback(_this: *mut c_void, _cmd: *mut c_void, _sender: *mut c_void) {
    request_show_window();
    unsafe {
        let app = shared_app();
        if app.is_null() {
            return;
        }
        msg1_bool(app, c"activateIgnoringOtherApps:", true);
        msg1_ptr(app, c"unhide:", std::ptr::null_mut());

        let windows = msg0(app, c"windows");
        if !windows.is_null() {
            let f_cnt: extern "C" fn(*mut c_void, *mut c_void) -> usize =
                std::mem::transmute(objc_msgSend as *const ());
            let count = f_cnt(windows, sel_registerName(c"count".as_ptr()));
            for i in 0..count {
                let win = msg1_usize(windows, c"objectAtIndex:", i);
                msg1_ptr(win, c"makeKeyAndOrderFront:", std::ptr::null_mut());
            }
        }
    }
}

extern "C" fn purge_ram_callback(_this: *mut c_void, _cmd: *mut c_void, _sender: *mut c_void) {
    std::thread::spawn(|| {
        let _ = std::process::Command::new("/usr/sbin/purge").output();
        if let Ok(snap) = MacosMemoryCollector::new().collect_memory() {
            dispatch_menu_bar_snapshot(snap);
        }
    });
}

extern "C" fn empty_trash_callback(_this: *mut c_void, _cmd: *mut c_void, _sender: *mut c_void) {
    std::thread::spawn(|| {
        let _ = crate::services::StorageCleanService::empty_trash();
    });
}

extern "C" fn quit_callback(_this: *mut c_void, _cmd: *mut c_void, _sender: *mut c_void) {
    unsafe {
        let app = shared_app();
        if !app.is_null() {
            msg1_ptr(app, c"terminate:", std::ptr::null_mut());
        }
    }
}

pub(super) unsafe fn create_nsstring(s: &str) -> *mut c_void {
    let cls_str = objc_getClass(c"NSString".as_ptr());
    let sel_utf8 = sel_registerName(c"stringWithUTF8String:".as_ptr());
    let f: extern "C" fn(*mut c_void, *mut c_void, *const c_char) -> *mut c_void =
        std::mem::transmute(objc_msgSend as *const ());
    let c_str = CString::new(s).unwrap_or_default();
    f(cls_str, sel_utf8, c_str.as_ptr())
}

pub(super) unsafe fn create_ram_icon() -> *mut c_void {
    let cls_img = objc_getClass(c"NSImage".as_ptr());
    let sel_sym = sel_registerName(c"imageWithSystemSymbolName:accessibilityDescription:".as_ptr());
    type SymFn = extern "C" fn(*mut c_void, *mut c_void, *mut c_void, *mut c_void) -> *mut c_void;
    let f_sym: SymFn = std::mem::transmute(objc_msgSend as *const ());
    let sym = create_nsstring("memorychip");
    let img = f_sym(cls_img, sel_sym, sym, std::ptr::null_mut());
    if !img.is_null() {
        msg1_bool(img, c"setTemplate:", true);
        msg0(img, c"retain");
    }
    img
}

pub(super) unsafe fn create_menu_item(
    title: &str,
    sel_name: &str,
    key_equiv: &str,
    target: *mut c_void,
) -> *mut c_void {
    let raw_item = msg0(objc_getClass(c"NSMenuItem".as_ptr()), c"alloc");
    let action_sel = if sel_name.is_empty() {
        std::ptr::null_mut()
    } else {
        sel_registerName(CString::new(sel_name).unwrap_or_default().as_ptr())
    };
    let sel_init = sel_registerName(c"initWithTitle:action:keyEquivalent:".as_ptr());
    type P = *mut c_void;
    let f_init: extern "C" fn(P, P, P, P, P) -> P = std::mem::transmute(objc_msgSend as *const ());
    let (ns_title, ns_key) = (create_nsstring(title), create_nsstring(key_equiv));
    let item = f_init(raw_item, sel_init, ns_title, action_sel, ns_key);
    if !target.is_null() {
        msg1_ptr(item, c"setTarget:", target);
    }
    item
}
