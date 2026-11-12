use std::ffi::c_void;

use crate::config::MenuBarDisplayMode;
use crate::models::{MemoryPressure, MemorySnapshot, ThermalSnapshot};

use super::ffi::{
    create_menu_item, create_nsstring, create_ram_icon, ensure_target_class_registered, msg0,
    msg1_bool, msg1_ptr, msg1_usize, objc_getClass, objc_msgSend, sel_registerName, NSApp,
};

pub struct MacosMenuBar {
    ui: (*mut c_void, *mut c_void, *mut c_void),
    res: (*mut c_void, *mut c_void),
}

unsafe impl Send for MacosMenuBar {}
unsafe impl Sync for MacosMenuBar {}

impl MacosMenuBar {
    pub fn new() -> Option<Self> {
        unsafe {
            let cls_bar = objc_getClass(c"NSStatusBar".as_ptr());
            if NSApp.is_null() || cls_bar.is_null() {
                return None;
            }
            ensure_target_class_registered();
            let status_bar = msg0(cls_bar, c"systemStatusBar");
            if status_bar.is_null() {
                return None;
            }
            let sel_item = sel_registerName(c"statusItemWithLength:".as_ptr());
            let f_item: extern "C" fn(*mut c_void, *mut c_void, f64) -> *mut c_void =
                std::mem::transmute(objc_msgSend as *const ());
            let item = f_item(status_bar, sel_item, -1.0);
            if item.is_null() {
                return None;
            }
            msg0(item, c"retain");

            let btn = msg0(item, c"button");
            let icon = create_ram_icon();
            if !icon.is_null() {
                msg1_ptr(btn, c"setImage:", icon);
                msg1_usize(btn, c"setImagePosition:", 7);
            }
            msg1_ptr(btn, c"setTitle:", create_nsstring(" -- GB"));

            let cls_target = objc_getClass(c"MemoraMenuBarTarget".as_ptr());
            let tgt = msg0(msg0(cls_target, c"alloc"), c"init");
            msg0(tgt, c"retain");

            let menu = msg0(msg0(objc_getClass(c"NSMenu".as_ptr()), c"alloc"), c"init");
            let hdr = create_menu_item("Memora — Telemetry", "", "", std::ptr::null_mut());
            msg1_bool(hdr, c"setEnabled:", false);
            msg0(hdr, c"retain");
            msg1_ptr(menu, c"addItem:", hdr);
            let cls_mi = objc_getClass(c"NSMenuItem".as_ptr());
            msg1_ptr(menu, c"addItem:", msg0(cls_mi, c"separatorItem"));

            for (lbl, act, key) in [
                ("Open Memora Dashboard", "openDashboard:", "o"),
                ("Purge RAM Cache", "purgeRamCache:", "p"),
                ("Empty Trash", "emptyTrash:", "t"),
            ] {
                msg1_ptr(menu, c"addItem:", create_menu_item(lbl, act, key, tgt));
            }
            msg1_ptr(menu, c"addItem:", msg0(cls_mi, c"separatorItem"));
            msg1_ptr(
                menu,
                c"addItem:",
                create_menu_item("Quit Memora", "quitMemora:", "q", tgt),
            );
            msg1_ptr(item, c"setMenu:", menu);
            Some(Self {
                ui: (item, btn, hdr),
                res: (tgt, icon),
            })
        }
    }

    pub fn update(
        &mut self,
        snap: &MemorySnapshot,
        th: Option<&ThermalSnapshot>,
        mode: MenuBarDisplayMode,
    ) {
        unsafe {
            let ((_, btn, hdr), (_, icon)) = (self.ui, self.res);
            if btn.is_null() || hdr.is_null() {
                return;
            }
            let gb = 1024.0 * 1024.0 * 1024.0;
            let (used, total) = (snap.used_bytes as f64 / gb, snap.total_bytes as f64 / gb);
            let pct = (snap.used_bytes as f64 / snap.total_bytes as f64 * 100.0).round() as u64;
            let temp_str = match th.and_then(|t| t.primary_temp()) {
                Some(temp) => format!(" • {:.0}°C", temp),
                None => String::new(),
            };
            let (img, pos, txt) = match mode {
                MenuBarDisplayMode::IconAndUsage => {
                    (icon, 7, format!(" {:.1} GB{}", used, temp_str))
                }
                MenuBarDisplayMode::IconOnly => (icon, 1, String::new()),
                MenuBarDisplayMode::UsageOnly => (
                    std::ptr::null_mut(),
                    0,
                    format!("{:.1} GB{}", used, temp_str),
                ),
            };
            msg1_ptr(btn, c"setImage:", img);
            msg1_usize(btn, c"setImagePosition:", pos);
            msg1_ptr(btn, c"setTitle:", create_nsstring(&txt));

            let p_str = match snap.pressure {
                MemoryPressure::Normal => "Normal",
                MemoryPressure::Warning => "Warning",
                MemoryPressure::Critical => "Critical",
                MemoryPressure::Unknown => "Unknown",
            };
            let h_lbl = match th.and_then(|t| t.primary_temp()) {
                Some(temp) => format!(
                    "Memora — {:.1}/{:.0} GB ({}%) • {:.1}°C",
                    used, total, pct, temp
                ),
                None => format!(
                    "Memora — {:.1}/{:.0} GB ({}% • {})",
                    used, total, pct, p_str
                ),
            };
            msg1_ptr(hdr, c"setTitle:", create_nsstring(&h_lbl));
        }
    }
}

impl Drop for MacosMenuBar {
    fn drop(&mut self) {
        unsafe {
            let ((item, _, hdr), (tgt, icon)) = (self.ui, self.res);
            if !item.is_null() {
                let status_bar = msg0(objc_getClass(c"NSStatusBar".as_ptr()), c"systemStatusBar");
                if !status_bar.is_null() {
                    msg1_ptr(status_bar, c"removeStatusItem:", item);
                }
                msg0(item, c"release");
            }
            for ptr in [hdr, tgt, icon] {
                if !ptr.is_null() {
                    msg0(ptr, c"release");
                }
            }
        }
    }
}
