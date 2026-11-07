use std::mem;
use std::time::SystemTime;

use crate::error::MemoraError;
use crate::models::{MemoryPressure, MemorySnapshot};
use crate::system::memory::MemoryCollector;

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
    fn host_page_size(host: libc::mach_port_t, out: *mut libc::vm_size_t) -> libc::kern_return_t;
    fn host_statistics64(
        host_priv: libc::mach_port_t,
        flavor: libc::c_int,
        host_info_out: *mut libc::c_void,
        host_info_out_cnt: *mut libc::mach_msg_type_number_t,
    ) -> libc::kern_return_t;
}

fn sysctl_read<T: Default>(key: &std::ffi::CStr, label: &str) -> Result<T, MemoraError> {
    let (mut val, mut len) = (T::default(), mem::size_of::<T>());
    let ptr = &mut val as *mut T as *mut libc::c_void;
    let ret = unsafe { libc::sysctlbyname(key.as_ptr(), ptr, &mut len, std::ptr::null_mut(), 0) };
    if ret != 0 {
        return Err(MemoraError::SysctlError {
            key: label.to_string(),
            message: std::io::Error::last_os_error().to_string(),
        });
    }
    Ok(val)
}

#[derive(Default)]
pub struct MacosMemoryCollector;

impl MacosMemoryCollector {
    pub fn new() -> Self {
        Self
    }

    fn get_swap_usage(&self) -> Result<(u64, u64), MemoraError> {
        let swap: XswUsage = sysctl_read(c"vm.swapusage", "vm.swapusage")?;
        Ok((swap.xsu_total, swap.xsu_used))
    }

    fn get_memory_pressure(&self) -> MemoryPressure {
        let key = c"kern.memorystatus_vm_pressure_level";
        match sysctl_read::<libc::c_int>(key, "kern.memorystatus_vm_pressure_level") {
            Ok(1) => MemoryPressure::Normal,
            Ok(2) => MemoryPressure::Warning,
            Ok(4) => MemoryPressure::Critical,
            _ => MemoryPressure::Unknown,
        }
    }

    fn get_vm_statistics(&self) -> Result<(VmStatistics64Data, u64), MemoraError> {
        let host = unsafe { mach_host_self() };
        let mut page_size: libc::vm_size_t = 0;
        let kr = unsafe { host_page_size(host, &mut page_size) };
        if kr != libc::KERN_SUCCESS {
            return Err(MemoraError::MachError(kr));
        }

        let (mut vm_stat, mut count) = (VmStatistics64Data::default(), HOST_VM_INFO64_COUNT);
        let ptr = &mut vm_stat as *mut VmStatistics64Data as *mut libc::c_void;
        let kr = unsafe { host_statistics64(host, HOST_VM_INFO64, ptr, &mut count) };
        if kr != libc::KERN_SUCCESS {
            return Err(MemoraError::MachError(kr));
        }
        Ok((vm_stat, page_size as u64))
    }
}

impl MemoryCollector for MacosMemoryCollector {
    fn collect_memory(&self) -> Result<MemorySnapshot, MemoraError> {
        let total_bytes = sysctl_read::<u64>(c"hw.memsize", "hw.memsize")?;
        let (swap_total_bytes, swap_used_bytes) = self.get_swap_usage().unwrap_or((0, 0));
        let pressure = self.get_memory_pressure();
        let (vm, ps) = self.get_vm_statistics()?;

        let (internal, purgeable) = (vm.internal_page_count as u64, vm.purgeable_count as u64);
        let app_memory_bytes = internal.saturating_sub(purgeable) * ps;
        let wired_bytes = (vm.wire_count as u64) * ps;
        let compressed_bytes = (vm.compressor_page_count as u64) * ps;
        let cached_files_bytes = (purgeable + vm.external_page_count as u64) * ps;
        let purgeable_bytes = purgeable * ps;
        let free_bytes = (vm.free_count as u64) * ps;
        let used_bytes = (app_memory_bytes + wired_bytes + compressed_bytes).min(total_bytes);

        Ok(MemorySnapshot {
            total_bytes,
            used_bytes,
            available_bytes: total_bytes.saturating_sub(used_bytes),
            app_memory_bytes,
            wired_bytes,
            compressed_bytes,
            cached_files_bytes,
            purgeable_bytes,
            free_bytes,
            swap_total_bytes,
            swap_used_bytes,
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
        let snap = MacosMemoryCollector::new().collect_memory().unwrap();
        assert!(snap.total_bytes > 0 && snap.used_bytes <= snap.total_bytes);
        assert_ne!(snap.pressure, MemoryPressure::Unknown);
    }
}
