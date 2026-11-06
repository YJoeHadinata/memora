use std::mem;
use std::time::SystemTime;

use crate::error::MemoraError;
use crate::models::{MemoryPressure, MemorySnapshot};
use crate::system::memory::MemoryCollector;

// Mach host and VM statistics definitions
const HOST_VM_INFO64: libc::c_int = 4;
const HOST_VM_INFO64_COUNT: libc::mach_msg_type_number_t = (mem::size_of::<VmStatistics64Data>()
    / mem::size_of::<libc::integer_t>())
    as libc::mach_msg_type_number_t;

#[repr(C)]
#[derive(Debug, Default, Clone, Copy)]
struct VmStatistics64Data {
    free_count: libc::natural_t,
    active_count: libc::natural_t,
    inactive_count: libc::natural_t,
    wire_count: libc::natural_t,
    zero_fill_count: u64,
    reactivations: u64,
    pageins: u64,
    pageouts: u64,
    faults: u64,
    cow_faults: u64,
    lookups: u64,
    hits: u64,
    purges: u64,
    purgeable_count: libc::natural_t,
    speculative_count: libc::natural_t,
    decompressions: u64,
    compressions: u64,
    swapins: u64,
    swapouts: u64,
    compressor_page_count: libc::natural_t,
    throttled_count: libc::natural_t,
    external_page_count: libc::natural_t,
    internal_page_count: libc::natural_t,
    total_uncompressed_pages_in_compressor: u64,
}

#[repr(C)]
#[derive(Debug, Default, Clone, Copy)]
struct XswUsage {
    xsu_total: u64,
    xsu_avail: u64,
    xsu_used: u64,
    xsu_pagesize: u32,
    xsu_encrypted: libc::boolean_t,
}

extern "C" {
    fn mach_host_self() -> libc::mach_port_t;
    fn host_page_size(
        host: libc::mach_port_t,
        page_size: *mut libc::vm_size_t,
    ) -> libc::kern_return_t;
    fn host_statistics64(
        host_priv: libc::mach_port_t,
        flavor: libc::c_int,
        host_info_out: *mut libc::c_void,
        host_info_outCnt: *mut libc::mach_msg_type_number_t,
    ) -> libc::kern_return_t;
}

pub struct MacosMemoryCollector;

impl MacosMemoryCollector {
    pub fn new() -> Self {
        Self
    }

    /// Fetches total physical memory in bytes via sysctl `hw.memsize`.
    fn get_physical_memory(&self) -> Result<u64, MemoraError> {
        let mut memsize: u64 = 0;
        let mut len = mem::size_of::<u64>();
        let name = c"hw.memsize";

        let ret = unsafe {
            libc::sysctlbyname(
                name.as_ptr(),
                &mut memsize as *mut u64 as *mut libc::c_void,
                &mut len,
                std::ptr::null_mut(),
                0,
            )
        };

        if ret != 0 {
            return Err(MemoraError::SysctlError {
                key: "hw.memsize".to_string(),
                message: std::io::Error::last_os_error().to_string(),
            });
        }

        Ok(memsize)
    }

    /// Fetches swap usage via sysctl `vm.swapusage`.
    fn get_swap_usage(&self) -> Result<(u64, u64), MemoraError> {
        let mut swap = XswUsage::default();
        let mut len = mem::size_of::<XswUsage>();
        let name = c"vm.swapusage";

        let ret = unsafe {
            libc::sysctlbyname(
                name.as_ptr(),
                &mut swap as *mut XswUsage as *mut libc::c_void,
                &mut len,
                std::ptr::null_mut(),
                0,
            )
        };

        if ret != 0 {
            return Err(MemoraError::SysctlError {
                key: "vm.swapusage".to_string(),
                message: std::io::Error::last_os_error().to_string(),
            });
        }

        Ok((swap.xsu_total, swap.xsu_used))
    }

    /// Fetches macOS memory pressure level via sysctl `kern.memorystatus_vm_pressure_level`.
    fn get_memory_pressure(&self) -> MemoryPressure {
        let mut level: libc::c_int = 0;
        let mut len = mem::size_of::<libc::c_int>();
        let name = c"kern.memorystatus_vm_pressure_level";

        let ret = unsafe {
            libc::sysctlbyname(
                name.as_ptr(),
                &mut level as *mut libc::c_int as *mut libc::c_void,
                &mut len,
                std::ptr::null_mut(),
                0,
            )
        };

        if ret != 0 {
            return MemoryPressure::Unknown;
        }

        match level {
            1 => MemoryPressure::Normal,
            2 => MemoryPressure::Warning,
            4 => MemoryPressure::Critical,
            _ => MemoryPressure::Unknown,
        }
    }

    /// Fetches Mach VM statistics via `host_statistics64`.
    fn get_vm_statistics(&self) -> Result<(VmStatistics64Data, u64), MemoraError> {
        let host = unsafe { mach_host_self() };

        let mut page_size: libc::vm_size_t = 0;
        let kern_ret = unsafe { host_page_size(host, &mut page_size) };
        if kern_ret != libc::KERN_SUCCESS {
            return Err(MemoraError::MachError(kern_ret));
        }

        let mut vm_stat = VmStatistics64Data::default();
        let mut count = HOST_VM_INFO64_COUNT;

        let kern_ret = unsafe {
            host_statistics64(
                host,
                HOST_VM_INFO64,
                &mut vm_stat as *mut VmStatistics64Data as *mut libc::c_void,
                &mut count,
            )
        };

        if kern_ret != libc::KERN_SUCCESS {
            return Err(MemoraError::MachError(kern_ret));
        }

        Ok((vm_stat, page_size as u64))
    }
}

impl Default for MacosMemoryCollector {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryCollector for MacosMemoryCollector {
    fn collect_memory(&self) -> Result<MemorySnapshot, MemoraError> {
        let total_bytes = self.get_physical_memory()?;
        let (swap_total, swap_used) = self.get_swap_usage().unwrap_or((0, 0));
        let pressure = self.get_memory_pressure();
        let (vm_stat, page_size) = self.get_vm_statistics()?;

        // macOS Activity Monitor calculation:
        // App Memory = (internal_page_count - purgeable_count) * page_size
        // Wired Memory = wire_count * page_size
        // Compressed Memory = compressor_page_count * page_size
        // Used Memory = App Memory + Wired Memory + Compressed Memory
        // Cached Files = (purgeable_count + external_page_count) * page_size
        // Free Memory = free_count * page_size
        // Available Memory = Free Memory + (reclaimable cache, or Total - Used)
        let internal_pages = vm_stat.internal_page_count as u64;
        let purgeable_pages = vm_stat.purgeable_count as u64;
        let wire_pages = vm_stat.wire_count as u64;
        let compressor_pages = vm_stat.compressor_page_count as u64;
        let external_pages = vm_stat.external_page_count as u64;
        let free_pages = vm_stat.free_count as u64;

        let app_memory_bytes = internal_pages.saturating_sub(purgeable_pages) * page_size;
        let wired_bytes = wire_pages * page_size;
        let compressed_bytes = compressor_pages * page_size;
        let cached_files_bytes = (purgeable_pages + external_pages) * page_size;
        let purgeable_bytes = purgeable_pages * page_size;
        let free_bytes = free_pages * page_size;

        let used_bytes = (app_memory_bytes + wired_bytes + compressed_bytes).min(total_bytes);
        let available_bytes = total_bytes.saturating_sub(used_bytes);

        Ok(MemorySnapshot {
            total_bytes,
            used_bytes,
            available_bytes,
            app_memory_bytes,
            wired_bytes,
            compressed_bytes,
            cached_files_bytes,
            purgeable_bytes,
            free_bytes,
            swap_total_bytes: swap_total,
            swap_used_bytes: swap_used,
            pressure,
            timestamp: SystemTime::now(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_macos_memory_collection() {
        let collector = MacosMemoryCollector::new();
        let snapshot = collector
            .collect_memory()
            .expect("Should collect macOS memory");

        assert!(snapshot.total_bytes > 0, "Total RAM should be > 0");
        assert!(snapshot.used_bytes > 0, "Used RAM should be > 0");
        assert!(
            snapshot.used_bytes <= snapshot.total_bytes,
            "Used RAM cannot exceed total"
        );
        assert_ne!(
            snapshot.pressure,
            MemoryPressure::Unknown,
            "Pressure should be resolved"
        );
    }
}
