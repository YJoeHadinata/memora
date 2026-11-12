pub mod memory;
pub mod menu_bar;
pub mod notification;
pub mod process;
pub mod storage;
pub mod thermal;

pub use memory::MacosMemoryCollector;
pub use menu_bar::MacosMenuBar;
pub use notification::send_notification;
pub use process::MacosProcessCollector;
pub use storage::MacosStorageCollector;
pub use thermal::MacosThermalCollector;

/// Requests the macOS system allocator to return unused heap memory pages back to the kernel.
#[cfg(target_os = "macos")]
pub fn purge_heap_memory() {
    extern "C" {
        fn malloc_zone_pressure_relief(zone: *mut libc::c_void, goal: libc::size_t)
            -> libc::size_t;
    }
    unsafe {
        malloc_zone_pressure_relief(std::ptr::null_mut(), 0);
    }
}
