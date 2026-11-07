mod ffi;

use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::config::MenuBarDisplayMode;
use crate::models::{MemoryPressure, MemorySnapshot};

use self::ffi::{
    create_menu_item, create_nsstring, create_ram_icon, create_separator,
    ensure_target_class_registered, msg0, msg1_ptr, msg1_usize, objc_getClass, objc_msgSend,
    sel_registerName, set_item_enabled, NSApp,
};

static SHOW_WINDOW_REQUESTED: AtomicBool = AtomicBool::new(false);

pub fn request_show_window() {
    SHOW_WINDOW_REQUESTED.store(true, Ordering::SeqCst);
}

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
    pub fn new() -> Option<Self> {
        unsafe {
            if NSApp.is_null() {
                return None;
            }
            ensure_target_class_registered();

            let cls_status_bar = objc_getClass(c"NSStatusBar".as_ptr());
            if cls_status_bar.is_null() {
                return None;
            }
            let status_bar = msg0(cls_status_bar, c"systemStatusBar");
            if status_bar.is_null() {
                return None;
            }

            let sel_item = sel_registerName(c"statusItemWithLength:".as_ptr());
            let f_item: extern "C" fn(*mut c_void, *mut c_void, f64) -> *mut c_void =
                std::mem::transmute(objc_msgSend as *const ());
            let status_item = f_item(status_bar, sel_item, -1.0);
            if status_item.is_null() {
                return None;
            }
            msg0(status_item, c"retain");

            let button = msg0(status_item, c"button");
            let ram_icon = create_ram_icon();
            if !ram_icon.is_null() {
                msg1_ptr(button, c"setImage:", ram_icon);
                msg1_usize(button, c"setImagePosition:", 7);
            }
            msg1_ptr(button, c"setTitle:", create_nsstring(" -- GB"));

            let cls_target = objc_getClass(c"MemoraMenuBarTarget".as_ptr());
            let target = msg0(msg0(cls_target, c"alloc"), c"init");
            msg0(target, c"retain");

            let menu = msg0(msg0(objc_getClass(c"NSMenu".as_ptr()), c"alloc"), c"init");
            let header_item = create_menu_item(
                "Memora — System Memory & Storage",
                "",
                "",
                std::ptr::null_mut(),
            );
            set_item_enabled(header_item, false);
            msg0(header_item, c"retain");
            msg1_ptr(menu, c"addItem:", header_item);
            msg1_ptr(menu, c"addItem:", create_separator());

            let actions = [
                ("Open Memora Dashboard", "openDashboard:", "o"),
                ("Purge RAM Cache", "purgeRamCache:", "p"),
                ("Empty Trash", "emptyTrash:", "t"),
            ];
            for (lbl, act, key) in actions {
                msg1_ptr(menu, c"addItem:", create_menu_item(lbl, act, key, target));
            }
            msg1_ptr(menu, c"addItem:", create_separator());
            let quit_item = create_menu_item("Quit Memora", "quitMemora:", "q", target);
            msg1_ptr(menu, c"addItem:", quit_item);
            msg1_ptr(status_item, c"setMenu:", menu);

            Some(Self {
                status_item,
                button,
                header_item,
                target,
                ram_icon,
            })
        }
    }

    pub fn update(&mut self, snapshot: &MemorySnapshot, mode: MenuBarDisplayMode) {
        unsafe {
            if self.button.is_null() || self.header_item.is_null() {
                return;
            }
            let used_gb = snapshot.used_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
            let total_gb = snapshot.total_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
            let used_pct =
                (snapshot.used_bytes as f64 / snapshot.total_bytes as f64 * 100.0).round() as u64;

            match mode {
                MenuBarDisplayMode::IconAndUsage => {
                    if !self.ram_icon.is_null() {
                        msg1_ptr(self.button, c"setImage:", self.ram_icon);
                        msg1_usize(self.button, c"setImagePosition:", 7);
                    }
                    msg1_ptr(
                        self.button,
                        c"setTitle:",
                        create_nsstring(&format!(" {:.1} GB", used_gb)),
                    );
                }
                MenuBarDisplayMode::IconOnly => {
                    if !self.ram_icon.is_null() {
                        msg1_ptr(self.button, c"setImage:", self.ram_icon);
                        msg1_usize(self.button, c"setImagePosition:", 1);
                    }
                    msg1_ptr(self.button, c"setTitle:", create_nsstring(""));
                }
                MenuBarDisplayMode::UsageOnly => {
                    msg1_ptr(self.button, c"setImage:", std::ptr::null_mut());
                    msg1_usize(self.button, c"setImagePosition:", 0);
                    msg1_ptr(
                        self.button,
                        c"setTitle:",
                        create_nsstring(&format!("{:.1} GB", used_gb)),
                    );
                }
            }

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
            msg1_ptr(
                self.header_item,
                c"setTitle:",
                create_nsstring(&header_label),
            );
        }
    }
}

impl Drop for MacosMenuBar {
    fn drop(&mut self) {
        unsafe {
            if !self.status_item.is_null() {
                let cls_bar = objc_getClass(c"NSStatusBar".as_ptr());
                let status_bar = msg0(cls_bar, c"systemStatusBar");
                if !status_bar.is_null() {
                    msg1_ptr(status_bar, c"removeStatusItem:", self.status_item);
                }
                msg0(self.status_item, c"release");
            }
            for ptr in [self.header_item, self.target, self.ram_icon] {
                if !ptr.is_null() {
                    msg0(ptr, c"release");
                }
            }
        }
    }
}
