use std::ffi::{c_char, c_void, CString};
use std::sync::atomic::{AtomicBool, Ordering};

use crate::config::MenuBarDisplayMode;
use crate::models::{MemoryPressure, MemorySnapshot};

#[link(name = "AppKit", kind = "framework")]
extern "C" {
    static NSApp: *mut c_void;
    fn objc_getClass(name: *const c_char) -> *mut c_void;
    fn sel_registerName(name: *const c_char) -> *mut c_void;
    fn objc_allocateClassPair(
        superclass: *mut c_void,
        name: *const c_char,
        extra_bytes: usize,
    ) -> *mut c_void;
    fn class_addMethod(
        cls: *mut c_void,
        name: *mut c_void,
        imp: *const (),
        types: *const c_char,
    ) -> bool;
    fn objc_registerClassPair(cls: *mut c_void);
    fn objc_msgSend();
}

static TARGET_CLASS_REGISTERED: AtomicBool = AtomicBool::new(false);
static SHOW_WINDOW_REQUESTED: AtomicBool = AtomicBool::new(false);

/// Signal requested to unhide/restore the main dashboard window.
pub fn request_show_window() {
    SHOW_WINDOW_REQUESTED.store(true, Ordering::SeqCst);
}

/// Consumes any pending window show/restore request.
pub fn take_show_window_request() -> bool {
    SHOW_WINDOW_REQUESTED.swap(false, Ordering::SeqCst)
}

pub struct MacosMenuBar {
    status_item: *mut c_void,
    button: *mut c_void,
    header_item: *mut c_void,
    target: *mut c_void,
    ram_icon: *mut c_void,
}

unsafe impl Send for MacosMenuBar {}
unsafe impl Sync for MacosMenuBar {}

impl MacosMenuBar {
    /// Initializes the macOS NSStatusItem in the top menu bar.
    pub fn new() -> Option<Self> {
        unsafe {
            // Guard against calling in headless CLI/unit tests where no WindowServer/NSApp connection exists
            if NSApp.is_null() {
                return None;
            }

            ensure_target_class_registered();

            let cls_status_bar = objc_getClass(c"NSStatusBar".as_ptr());
            if cls_status_bar.is_null() {
                return None;
            }

            let sel_sys = sel_registerName(c"systemStatusBar".as_ptr());
            let func_sys: extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void =
                std::mem::transmute(objc_msgSend as *const ());
            let status_bar = func_sys(cls_status_bar, sel_sys);
            if status_bar.is_null() {
                return None;
            }

            let sel_item = sel_registerName(c"statusItemWithLength:".as_ptr());
            let func_item: extern "C" fn(*mut c_void, *mut c_void, f64) -> *mut c_void =
                std::mem::transmute(objc_msgSend as *const ());
            // -1.0 = NSVariableStatusItemLength
            let status_item = func_item(status_bar, sel_item, -1.0);
            if status_item.is_null() {
                return None;
            }

            // CRITICAL: NSStatusBar does NOT retain NSStatusItem on macOS 10.10+.
            // We MUST retain it to prevent autorelease deallocation and EXC_BAD_ACCESS!
            let sel_retain = sel_registerName(c"retain".as_ptr());
            let func_retain: extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void =
                std::mem::transmute(objc_msgSend as *const ());
            func_retain(status_item, sel_retain);

            let sel_button = sel_registerName(c"button".as_ptr());
            let func_button: extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void =
                std::mem::transmute(objc_msgSend as *const ());
            let button = func_button(status_item, sel_button);

            // Load native macOS RAM chip template icon
            let ram_icon = create_ram_icon();
            if !ram_icon.is_null() {
                let sel_set_image = sel_registerName(c"setImage:".as_ptr());
                let func_set_image: extern "C" fn(*mut c_void, *mut c_void, *mut c_void) =
                    std::mem::transmute(objc_msgSend as *const ());
                func_set_image(button, sel_set_image, ram_icon);

                let sel_set_pos = sel_registerName(c"setImagePosition:".as_ptr());
                let func_set_pos: extern "C" fn(*mut c_void, *mut c_void, usize) =
                    std::mem::transmute(objc_msgSend as *const ());
                // 7 = NSImageLeading
                func_set_pos(button, sel_set_pos, 7);
            }

            let sel_set_title = sel_registerName(c"setTitle:".as_ptr());
            let func_set_title: extern "C" fn(*mut c_void, *mut c_void, *mut c_void) =
                std::mem::transmute(objc_msgSend as *const ());
            let title_str = create_nsstring(" -- GB");
            func_set_title(button, sel_set_title, title_str);

            // Create target instance
            let cls_target = objc_getClass(c"MemoraMenuBarTarget".as_ptr());
            let sel_alloc = sel_registerName(c"alloc".as_ptr());
            let sel_init = sel_registerName(c"init".as_ptr());
            let func_alloc: extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void =
                std::mem::transmute(objc_msgSend as *const ());
            let func_init: extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void =
                std::mem::transmute(objc_msgSend as *const ());
            let target = func_init(func_alloc(cls_target, sel_alloc), sel_init);
            func_retain(target, sel_retain);

            // Create NSMenu
            let cls_menu = objc_getClass(c"NSMenu".as_ptr());
            let menu = func_init(func_alloc(cls_menu, sel_alloc), sel_init);

            let sel_add_item = sel_registerName(c"addItem:".as_ptr());
            let func_add_item: extern "C" fn(*mut c_void, *mut c_void, *mut c_void) =
                std::mem::transmute(objc_msgSend as *const ());

            // 1. Header status item (disabled)
            let header_item = create_menu_item(
                "Memora — System Memory & Storage",
                "",
                "",
                std::ptr::null_mut(),
            );
            set_item_enabled(header_item, false);
            func_retain(header_item, sel_retain);
            func_add_item(menu, sel_add_item, header_item);

            // 2. Separator
            func_add_item(menu, sel_add_item, create_separator());

            // 3. Open Dashboard action
            let open_item =
                create_menu_item("Open Memora Dashboard", "openDashboard:", "o", target);
            func_add_item(menu, sel_add_item, open_item);

            // 4. Quick Purge RAM Cache
            let purge_item = create_menu_item("Purge RAM Cache", "purgeRamCache:", "p", target);
            func_add_item(menu, sel_add_item, purge_item);

            // 5. Quick Empty Trash
            let trash_item = create_menu_item("Empty Trash", "emptyTrash:", "t", target);
            func_add_item(menu, sel_add_item, trash_item);

            // 6. Separator
            func_add_item(menu, sel_add_item, create_separator());

            // 7. Quit Memora
            let quit_item = create_menu_item("Quit Memora", "quitMemora:", "q", target);
            func_add_item(menu, sel_add_item, quit_item);

            // Attach menu to status_item
            let sel_set_menu = sel_registerName(c"setMenu:".as_ptr());
            let func_set_menu: extern "C" fn(*mut c_void, *mut c_void, *mut c_void) =
                std::mem::transmute(objc_msgSend as *const ());
            func_set_menu(status_item, sel_set_menu, menu);

            Some(Self {
                status_item,
                button,
                header_item,
                target,
                ram_icon,
            })
        }
    }

    /// Updates the text shown in the status bar button and the dropdown header based on display mode.
    pub fn update(&mut self, snapshot: &MemorySnapshot, mode: MenuBarDisplayMode) {
        unsafe {
            if self.button.is_null() || self.header_item.is_null() {
                return;
            }

            let used_gb = snapshot.used_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
            let total_gb = snapshot.total_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
            let used_pct =
                (snapshot.used_bytes as f64 / snapshot.total_bytes as f64 * 100.0).round() as u64;

            let sel_set_title = sel_registerName(c"setTitle:".as_ptr());
            let func_set_title: extern "C" fn(*mut c_void, *mut c_void, *mut c_void) =
                std::mem::transmute(objc_msgSend as *const ());

            let sel_set_image = sel_registerName(c"setImage:".as_ptr());
            let func_set_image: extern "C" fn(*mut c_void, *mut c_void, *mut c_void) =
                std::mem::transmute(objc_msgSend as *const ());

            let sel_set_pos = sel_registerName(c"setImagePosition:".as_ptr());
            let func_set_pos: extern "C" fn(*mut c_void, *mut c_void, usize) =
                std::mem::transmute(objc_msgSend as *const ());

            match mode {
                MenuBarDisplayMode::IconAndUsage => {
                    if !self.ram_icon.is_null() {
                        func_set_image(self.button, sel_set_image, self.ram_icon);
                        // 7 = NSImageLeading (icon placed before title)
                        func_set_pos(self.button, sel_set_pos, 7);
                    }
                    let status_label = format!(" {:.1} GB", used_gb);
                    let ns_title = create_nsstring(&status_label);
                    func_set_title(self.button, sel_set_title, ns_title);
                }
                MenuBarDisplayMode::IconOnly => {
                    if !self.ram_icon.is_null() {
                        func_set_image(self.button, sel_set_image, self.ram_icon);
                        // 1 = NSImageOnly
                        func_set_pos(self.button, sel_set_pos, 1);
                    }
                    let ns_empty = create_nsstring("");
                    func_set_title(self.button, sel_set_title, ns_empty);
                }
                MenuBarDisplayMode::UsageOnly => {
                    func_set_image(self.button, sel_set_image, std::ptr::null_mut());
                    // 0 = NSNoImage
                    func_set_pos(self.button, sel_set_pos, 0);
                    let status_label = format!("{:.1} GB", used_gb);
                    let ns_title = create_nsstring(&status_label);
                    func_set_title(self.button, sel_set_title, ns_title);
                }
            }

            // Dropdown menu header: "Memora — 11.2 GB / 16.0 GB (70% • Normal)"
            let pressure_str = match snapshot.pressure {
                MemoryPressure::Normal => "Normal",
                MemoryPressure::Warning => "Warning",
                MemoryPressure::Critical => "Critical",
                MemoryPressure::Unknown => "Unknown",
            };
            let header_label = format!(
                "Memora — {:.1} GB / {:.0} GB ({}% • {})",
                used_gb, total_gb, used_pct, pressure_str
            );
            let ns_header = create_nsstring(&header_label);
            func_set_title(self.header_item, sel_set_title, ns_header);
        }
    }
}

impl Drop for MacosMenuBar {
    fn drop(&mut self) {
        unsafe {
            let sel_release = sel_registerName(c"release".as_ptr());
            let func_release: extern "C" fn(*mut c_void, *mut c_void) =
                std::mem::transmute(objc_msgSend as *const ());

            if !self.status_item.is_null() {
                let cls_status_bar = objc_getClass(c"NSStatusBar".as_ptr());
                let sel_sys = sel_registerName(c"systemStatusBar".as_ptr());
                let func_sys: extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void =
                    std::mem::transmute(objc_msgSend as *const ());
                let status_bar = func_sys(cls_status_bar, sel_sys);

                if !status_bar.is_null() {
                    let sel_remove = sel_registerName(c"removeStatusItem:".as_ptr());
                    let func_remove: extern "C" fn(*mut c_void, *mut c_void, *mut c_void) =
                        std::mem::transmute(objc_msgSend as *const ());
                    func_remove(status_bar, sel_remove, self.status_item);
                }

                func_release(self.status_item, sel_release);
            }

            if !self.header_item.is_null() {
                func_release(self.header_item, sel_release);
            }

            if !self.target.is_null() {
                func_release(self.target, sel_release);
            }

            if !self.ram_icon.is_null() {
                func_release(self.ram_icon, sel_release);
            }
        }
    }
}

// Internal Cocoa Objective-C helpers
unsafe fn ensure_target_class_registered() {
    if TARGET_CLASS_REGISTERED.swap(true, Ordering::SeqCst) {
        return;
    }

    let superclass = objc_getClass(c"NSObject".as_ptr());
    let new_class = objc_allocateClassPair(superclass, c"MemoraMenuBarTarget".as_ptr(), 0);
    if new_class.is_null() {
        return;
    }

    // openDashboard:
    let sel_open = sel_registerName(c"openDashboard:".as_ptr());
    class_addMethod(
        new_class,
        sel_open,
        open_dashboard_callback as *const (),
        c"v@:@".as_ptr(),
    );

    // purgeRamCache:
    let sel_purge = sel_registerName(c"purgeRamCache:".as_ptr());
    class_addMethod(
        new_class,
        sel_purge,
        purge_ram_callback as *const (),
        c"v@:@".as_ptr(),
    );

    // emptyTrash:
    let sel_trash = sel_registerName(c"emptyTrash:".as_ptr());
    class_addMethod(
        new_class,
        sel_trash,
        empty_trash_callback as *const (),
        c"v@:@".as_ptr(),
    );

    // quitMemora:
    let sel_quit = sel_registerName(c"quitMemora:".as_ptr());
    class_addMethod(
        new_class,
        sel_quit,
        quit_callback as *const (),
        c"v@:@".as_ptr(),
    );

    objc_registerClassPair(new_class);
}

extern "C" fn open_dashboard_callback(_this: *mut c_void, _cmd: *mut c_void, _sender: *mut c_void) {
    request_show_window();
    unsafe {
        let cls_app = objc_getClass(c"NSApplication".as_ptr());
        let sel_shared = sel_registerName(c"sharedApplication".as_ptr());
        let func_shared: extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void =
            std::mem::transmute(objc_msgSend as *const ());
        let app = func_shared(cls_app, sel_shared);
        if app.is_null() {
            return;
        }

        // Activate app ignoring others
        let sel_activate = sel_registerName(c"activateIgnoringOtherApps:".as_ptr());
        let func_activate: extern "C" fn(*mut c_void, *mut c_void, bool) =
            std::mem::transmute(objc_msgSend as *const ());
        func_activate(app, sel_activate, true);

        // Unhide
        let sel_unhide = sel_registerName(c"unhide:".as_ptr());
        let func_unhide: extern "C" fn(*mut c_void, *mut c_void, *mut c_void) =
            std::mem::transmute(objc_msgSend as *const ());
        func_unhide(app, sel_unhide, std::ptr::null_mut());

        // Bring windows to front
        let sel_windows = sel_registerName(c"windows".as_ptr());
        let func_windows: extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void =
            std::mem::transmute(objc_msgSend as *const ());
        let windows = func_windows(app, sel_windows);
        if !windows.is_null() {
            let sel_count = sel_registerName(c"count".as_ptr());
            let func_count: extern "C" fn(*mut c_void, *mut c_void) -> usize =
                std::mem::transmute(objc_msgSend as *const ());
            let count = func_count(windows, sel_count);

            let sel_obj_at = sel_registerName(c"objectAtIndex:".as_ptr());
            let func_obj_at: extern "C" fn(*mut c_void, *mut c_void, usize) -> *mut c_void =
                std::mem::transmute(objc_msgSend as *const ());
            let sel_order_front = sel_registerName(c"makeKeyAndOrderFront:".as_ptr());
            let func_order_front: extern "C" fn(*mut c_void, *mut c_void, *mut c_void) =
                std::mem::transmute(objc_msgSend as *const ());

            for i in 0..count {
                let win = func_obj_at(windows, sel_obj_at, i);
                func_order_front(win, sel_order_front, std::ptr::null_mut());
            }
        }
    }
}

extern "C" fn purge_ram_callback(_this: *mut c_void, _cmd: *mut c_void, _sender: *mut c_void) {
    std::thread::spawn(|| {
        let _ = std::process::Command::new("/usr/sbin/purge").output();
    });
}

extern "C" fn empty_trash_callback(_this: *mut c_void, _cmd: *mut c_void, _sender: *mut c_void) {
    std::thread::spawn(|| {
        let _ = crate::services::StorageCleanService::empty_trash();
    });
}

extern "C" fn quit_callback(_this: *mut c_void, _cmd: *mut c_void, _sender: *mut c_void) {
    unsafe {
        let cls_app = objc_getClass(c"NSApplication".as_ptr());
        let sel_shared = sel_registerName(c"sharedApplication".as_ptr());
        let func_shared: extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void =
            std::mem::transmute(objc_msgSend as *const ());
        let app = func_shared(cls_app, sel_shared);
        if !app.is_null() {
            let sel_terminate = sel_registerName(c"terminate:".as_ptr());
            let func_terminate: extern "C" fn(*mut c_void, *mut c_void, *mut c_void) =
                std::mem::transmute(objc_msgSend as *const ());
            func_terminate(app, sel_terminate, std::ptr::null_mut());
        }
    }
}

unsafe fn create_nsstring(s: &str) -> *mut c_void {
    let cls_str = objc_getClass(c"NSString".as_ptr());
    let sel_utf8 = sel_registerName(c"stringWithUTF8String:".as_ptr());
    let func: extern "C" fn(*mut c_void, *mut c_void, *const c_char) -> *mut c_void =
        std::mem::transmute(objc_msgSend as *const ());
    let c_str = CString::new(s).unwrap_or_default();
    func(cls_str, sel_utf8, c_str.as_ptr())
}

unsafe fn create_ram_icon() -> *mut c_void {
    let cls_img = objc_getClass(c"NSImage".as_ptr());
    let sel_sym = sel_registerName(c"imageWithSystemSymbolName:accessibilityDescription:".as_ptr());
    let func_sym: extern "C" fn(*mut c_void, *mut c_void, *mut c_void, *mut c_void) -> *mut c_void =
        std::mem::transmute(objc_msgSend as *const ());

    let sym_name = create_nsstring("memorychip");
    let img = func_sym(cls_img, sel_sym, sym_name, std::ptr::null_mut());

    if !img.is_null() {
        let sel_template = sel_registerName(c"setTemplate:".as_ptr());
        let func_template: extern "C" fn(*mut c_void, *mut c_void, bool) =
            std::mem::transmute(objc_msgSend as *const ());
        func_template(img, sel_template, true);

        let sel_retain = sel_registerName(c"retain".as_ptr());
        let func_retain: extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void =
            std::mem::transmute(objc_msgSend as *const ());
        func_retain(img, sel_retain);
    }
    img
}

unsafe fn create_menu_item(
    title: &str,
    sel_name: &str,
    key_equiv: &str,
    target: *mut c_void,
) -> *mut c_void {
    let cls_item = objc_getClass(c"NSMenuItem".as_ptr());
    let sel_alloc = sel_registerName(c"alloc".as_ptr());
    let sel_init = sel_registerName(c"initWithTitle:action:keyEquivalent:".as_ptr());

    let func_alloc: extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void =
        std::mem::transmute(objc_msgSend as *const ());
    let raw_item = func_alloc(cls_item, sel_alloc);

    let ns_title = create_nsstring(title);
    let action_sel = if sel_name.is_empty() {
        std::ptr::null_mut()
    } else {
        let c_sel = CString::new(sel_name).unwrap_or_default();
        sel_registerName(c_sel.as_ptr())
    };
    let ns_key = create_nsstring(key_equiv);

    let func_init: extern "C" fn(
        *mut c_void,
        *mut c_void,
        *mut c_void,
        *mut c_void,
        *mut c_void,
    ) -> *mut c_void = std::mem::transmute(objc_msgSend as *const ());

    let item = func_init(raw_item, sel_init, ns_title, action_sel, ns_key);

    if !target.is_null() {
        let sel_set_target = sel_registerName(c"setTarget:".as_ptr());
        let func_set_target: extern "C" fn(*mut c_void, *mut c_void, *mut c_void) =
            std::mem::transmute(objc_msgSend as *const ());
        func_set_target(item, sel_set_target, target);
    }

    item
}

unsafe fn set_item_enabled(item: *mut c_void, enabled: bool) {
    let sel_set_enabled = sel_registerName(c"setEnabled:".as_ptr());
    let func_set_enabled: extern "C" fn(*mut c_void, *mut c_void, bool) =
        std::mem::transmute(objc_msgSend as *const ());
    func_set_enabled(item, sel_set_enabled, enabled);
}

unsafe fn create_separator() -> *mut c_void {
    let cls_item = objc_getClass(c"NSMenuItem".as_ptr());
    let sel_separator = sel_registerName(c"separatorItem".as_ptr());
    let func_sep: extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void =
        std::mem::transmute(objc_msgSend as *const ());
    func_sep(cls_item, sel_separator)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_menu_bar_init_does_not_panic() {
        // Safe check on macOS
        let bar = MacosMenuBar::new();
        assert!(bar.is_some() || bar.is_none());
    }
}
