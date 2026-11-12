mod ffi;
mod item;

use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

use crate::config::MenuBarDisplayMode;
use crate::models::{MemorySnapshot, ThermalSnapshot};

pub use item::MacosMenuBar;

use self::ffi::{_dispatch_main_q, dispatch_async_f};

static SHOW_WINDOW_REQUESTED: AtomicBool = AtomicBool::new(false);

pub fn request_show_window() {
    SHOW_WINDOW_REQUESTED.store(true, Ordering::SeqCst);
}

pub fn take_show_window_request() -> bool {
    SHOW_WINDOW_REQUESTED.swap(false, Ordering::SeqCst)
}

struct SharedMenuBarState {
    bar: Option<MacosMenuBar>,
    snapshot: Option<MemorySnapshot>,
    thermal: Option<ThermalSnapshot>,
    show: bool,
    mode: MenuBarDisplayMode,
}

static MENU_BAR_STATE: Mutex<SharedMenuBarState> = Mutex::new(SharedMenuBarState {
    bar: None,
    snapshot: None,
    thermal: None,
    show: true,
    mode: MenuBarDisplayMode::IconAndUsage,
});

pub fn sync_menu_bar_config(show: bool, mode: MenuBarDisplayMode) {
    if let Ok(mut g) = MENU_BAR_STATE.lock() {
        g.show = show;
        g.mode = mode;
    }
    schedule_main_thread_flush();
}

pub fn dispatch_menu_bar_telemetry(snapshot: MemorySnapshot, thermal: Option<ThermalSnapshot>) {
    if let Ok(mut g) = MENU_BAR_STATE.lock() {
        g.snapshot = Some(snapshot);
        if let Some(th) = thermal {
            g.thermal = Some(th);
        }
    }
    schedule_main_thread_flush();
}

pub fn dispatch_menu_bar_snapshot(snapshot: MemorySnapshot) {
    dispatch_menu_bar_telemetry(snapshot, None);
}

fn schedule_main_thread_flush() {
    unsafe {
        let q = std::ptr::addr_of!(_dispatch_main_q);
        dispatch_async_f(q, std::ptr::null_mut(), flush_menu_bar_on_main);
    }
}

extern "C" fn flush_menu_bar_on_main(_ctx: *mut c_void) {
    let Ok(mut state) = MENU_BAR_STATE.lock() else {
        return;
    };
    if !state.show {
        state.bar = None;
        return;
    }
    if state.bar.is_none() {
        state.bar = MacosMenuBar::new();
    }
    let (mode, snap_opt, th_opt) = (state.mode, state.snapshot.clone(), state.thermal.clone());
    if let (Some(ref mut bar), Some(ref snap)) = (&mut state.bar, &snap_opt) {
        bar.update(snap, th_opt.as_ref(), mode);
    }
}
