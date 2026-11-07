use std::ffi::CStr;
use std::mem;

use crate::error::MemoraError;
use crate::models::ProcessArchitecture;

pub(super) const PROC_ALL_PIDS: u32 = 1;
pub(super) const RUSAGE_INFO_V4: libc::c_int = 4;
pub(super) const PROC_PIDTASKINFO: libc::c_int = 4;
const PROC_PIDPATHINFO_MAXSIZE: usize = 4096;

#[repr(C)]
#[derive(Debug, Default, Clone, Copy)]
pub(super) struct RusageInfoV4 {
    pub ri_uuid: [u8; 16],
    pub ri_user_time: u64,
    pub ri_system_time: u64,
    pub ri_pkg_idle_wkups: u64,
    pub ri_interrupt_wkups: u64,
    pub ri_pageins: u64,
    pub ri_wired_size: u64,
    pub ri_resident_size: u64,
    pub ri_phys_footprint: u64,
    pub ri_proc_start_abstime: u64,
    pub ri_proc_exit_abstime: u64,
    pub ri_child_user_time: u64,
    pub ri_child_system_time: u64,
    pub ri_child_pkg_idle_wkups: u64,
    pub ri_child_interrupt_wkups: u64,
    pub ri_child_pageins: u64,
    pub ri_child_elapsed_abstime: u64,
    pub ri_diskio_bytesread: u64,
    pub ri_diskio_byteswritten: u64,
    pub ri_cpu_time_qos_default: u64,
    pub ri_cpu_time_qos_maintenance: u64,
    pub ri_cpu_time_qos_background: u64,
    pub ri_cpu_time_qos_utility: u64,
    pub ri_cpu_time_qos_legacy: u64,
    pub ri_cpu_time_qos_user_initiated: u64,
    pub ri_cpu_time_qos_user_interactive: u64,
    pub ri_billed_system_time: u64,
    pub ri_serviced_system_time: u64,
    pub ri_logical_writes: u64,
    pub ri_lifetime_max_phys_footprint: u64,
    pub ri_instructions: u64,
    pub ri_cycles: u64,
    pub ri_billed_energy: u64,
    pub ri_serviced_energy: u64,
    pub ri_interval_max_phys_footprint: u64,
    pub ri_runnable_time: u64,
}

#[repr(C)]
#[derive(Debug, Default, Clone, Copy)]
pub(super) struct ProcTaskInfo {
    pub pti_virtual_size: u64,
    pub pti_resident_size: u64,
    pub pti_total_user: u64,
    pub pti_total_system: u64,
    pub pti_threads_user: u64,
    pub pti_threads_system: u64,
    pub pti_policy: i32,
    pub pti_faults: i32,
    pub pti_pageins: i32,
    pub pti_cow_faults: i32,
    pub pti_messages_sent: i32,
    pub pti_messages_received: i32,
    pub pti_syscalls_mach: i32,
    pub pti_syscalls_unix: i32,
    pub pti_csw: i32,
    pub pti_threadnum: i32,
    pub pti_numrunning: i32,
    pub pti_priority: i32,
}

extern "C" {
    pub(super) fn proc_listpids(
        t: u32,
        ti: u32,
        b: *mut libc::c_void,
        sz: libc::c_int,
    ) -> libc::c_int;
    pub(super) fn proc_name(pid: libc::c_int, b: *mut libc::c_void, sz: u32) -> libc::c_int;
    pub(super) fn proc_pidpath(pid: libc::c_int, b: *mut libc::c_void, sz: u32) -> libc::c_int;
    pub(super) fn proc_pid_rusage(
        pid: libc::c_int,
        fl: libc::c_int,
        b: *mut libc::c_void,
    ) -> libc::c_int;
    pub(super) fn proc_pidinfo(
        pid: libc::c_int,
        fl: libc::c_int,
        arg: u64,
        b: *mut libc::c_void,
        sz: libc::c_int,
    ) -> libc::c_int;
}

pub(super) fn list_all_pids() -> Result<Vec<i32>, MemoraError> {
    let num_pids = unsafe { proc_listpids(PROC_ALL_PIDS, 0, std::ptr::null_mut(), 0) };
    if num_pids <= 0 {
        return Err(MemoraError::SystemError(
            "proc_listpids count failed".into(),
        ));
    }

    let capacity = (num_pids as usize / mem::size_of::<i32>()) + 128;
    let mut pids = vec![0i32; capacity];
    let bytes = (capacity * mem::size_of::<i32>()) as libc::c_int;
    let ptr = pids.as_mut_ptr() as *mut libc::c_void;
    let res = unsafe { proc_listpids(PROC_ALL_PIDS, 0, ptr, bytes) };
    if res <= 0 {
        return Err(MemoraError::SystemError(
            "proc_listpids fetch failed".into(),
        ));
    }

    pids.truncate((res as usize) / mem::size_of::<i32>());
    Ok(pids)
}

fn cstr_to_string(buf: &[u8]) -> Option<String> {
    CStr::from_bytes_until_nul(buf)
        .ok()
        .and_then(|c| c.to_str().ok())
        .map(String::from)
}

pub(super) fn query_name_and_path(pid: i32) -> (String, Option<String>) {
    let mut path_buf = [0u8; PROC_PIDPATHINFO_MAXSIZE];
    let p_ptr = path_buf.as_mut_ptr() as *mut libc::c_void;
    let path_len = unsafe { proc_pidpath(pid, p_ptr, path_buf.len() as u32) };
    let path = (path_len > 0).then(|| cstr_to_string(&path_buf)).flatten();

    let mut name_buf = [0u8; 256];
    let n_ptr = name_buf.as_mut_ptr() as *mut libc::c_void;
    let name_len = unsafe { proc_name(pid, n_ptr, name_buf.len() as u32) };
    let raw_name = (name_len > 0)
        .then(|| cstr_to_string(&name_buf))
        .flatten()
        .unwrap_or_default();

    let name = path
        .as_deref()
        .and_then(|p| std::path::Path::new(p).file_name())
        .and_then(|s| s.to_str())
        .filter(|s| !s.is_empty())
        .map(String::from)
        .unwrap_or(raw_name);

    (name, path)
}

pub(super) fn query_process_architecture(pid: i32) -> ProcessArchitecture {
    if std::env::consts::ARCH != "aarch64" {
        return ProcessArchitecture::Intel;
    }
    let mut mib = [libc::CTL_KERN, libc::KERN_PROC, libc::KERN_PROC_PID, pid];
    let (mut kinfo, mut size) = ([0u8; 648], 648usize);
    let ptr = kinfo.as_mut_ptr() as *mut libc::c_void;
    let res = unsafe { libc::sysctl(mib.as_mut_ptr(), 4, ptr, &mut size, std::ptr::null_mut(), 0) };
    if res == 0 && size >= 36 {
        let p_flag = u32::from_ne_bytes([kinfo[32], kinfo[33], kinfo[34], kinfo[35]]);
        if (p_flag & 0x0002_0000) != 0 {
            return ProcessArchitecture::Intel;
        }
    }
    ProcessArchitecture::AppleSilicon
}
