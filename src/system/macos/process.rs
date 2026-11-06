use std::collections::{HashMap, HashSet};
use std::ffi::CStr;
use std::mem;
use std::sync::Mutex;
use std::time::Instant;

use crate::error::MemoraError;
use crate::models::{ProcessArchitecture, ProcessInfo};
use crate::system::process::{ProcessCollector, ProcessManager};

const PROC_ALL_PIDS: u32 = 1;
const RUSAGE_INFO_V4: libc::c_int = 4;
const PROC_PIDTASKINFO: libc::c_int = 4;
const PROC_PIDPATHINFO_MAXSIZE: usize = 4096;

#[repr(C)]
#[derive(Debug, Default, Clone, Copy)]
struct RusageInfoV4 {
    ri_uuid: [u8; 16],
    ri_user_time: u64,
    ri_system_time: u64,
    ri_pkg_idle_wkups: u64,
    ri_interrupt_wkups: u64,
    ri_pageins: u64,
    ri_wired_size: u64,
    ri_resident_size: u64,
    ri_phys_footprint: u64,
    ri_proc_start_abstime: u64,
    ri_proc_exit_abstime: u64,
    ri_child_user_time: u64,
    ri_child_system_time: u64,
    ri_child_pkg_idle_wkups: u64,
    ri_child_interrupt_wkups: u64,
    ri_child_pageins: u64,
    ri_child_elapsed_abstime: u64,
    ri_diskio_bytesread: u64,
    ri_diskio_byteswritten: u64,
    ri_cpu_time_qos_default: u64,
    ri_cpu_time_qos_maintenance: u64,
    ri_cpu_time_qos_background: u64,
    ri_cpu_time_qos_utility: u64,
    ri_cpu_time_qos_legacy: u64,
    ri_cpu_time_qos_user_initiated: u64,
    ri_cpu_time_qos_user_interactive: u64,
    ri_billed_system_time: u64,
    ri_serviced_system_time: u64,
    ri_logical_writes: u64,
    ri_lifetime_max_phys_footprint: u64,
    ri_instructions: u64,
    ri_cycles: u64,
    ri_billed_energy: u64,
    ri_serviced_energy: u64,
    ri_interval_max_phys_footprint: u64,
    ri_runnable_time: u64,
}

#[repr(C)]
#[derive(Debug, Default, Clone, Copy)]
struct ProcTaskInfo {
    pti_virtual_size: u64,
    pti_resident_size: u64,
    pti_total_user: u64,
    pti_total_system: u64,
    pti_threads_user: u64,
    pti_threads_system: u64,
    pti_policy: i32,
    pti_faults: i32,
    pti_pageins: i32,
    pti_cow_faults: i32,
    pti_messages_sent: i32,
    pti_messages_received: i32,
    pti_syscalls_mach: i32,
    pti_syscalls_unix: i32,
    pti_csw: i32,
    pti_threadnum: i32,
    pti_numrunning: i32,
    pti_priority: i32,
}

extern "C" {
    fn proc_listpids(
        type_: u32,
        typeinfo: u32,
        buffer: *mut libc::c_void,
        buffersize: libc::c_int,
    ) -> libc::c_int;
    fn proc_name(pid: libc::c_int, buffer: *mut libc::c_void, buffersize: u32) -> libc::c_int;
    fn proc_pidpath(pid: libc::c_int, buffer: *mut libc::c_void, buffersize: u32) -> libc::c_int;
    fn proc_pid_rusage(
        pid: libc::c_int,
        flavor: libc::c_int,
        buffer: *mut libc::c_void,
    ) -> libc::c_int;
    fn proc_pidinfo(
        pid: libc::c_int,
        flavor: libc::c_int,
        arg: u64,
        buffer: *mut libc::c_void,
        buffersize: libc::c_int,
    ) -> libc::c_int;
}

struct ProcessMetrics {
    memory_bytes: u64,
    resident_bytes: u64,
    peak_memory_bytes: u64,
    pageins: u64,
    disk_read_bytes: u64,
    disk_write_bytes: u64,
    cpu_usage: Option<f32>,
}

pub struct MacosProcessCollector {
    protected_names: HashSet<&'static str>,
    previous_cpu: Mutex<HashMap<i32, (Instant, u64)>>,
}

impl MacosProcessCollector {
    pub fn new() -> Self {
        let mut protected_names = HashSet::new();
        protected_names.insert("kernel_task");
        protected_names.insert("launchd");
        protected_names.insert("WindowServer");
        protected_names.insert("loginwindow");
        protected_names.insert("syspolicyd");
        protected_names.insert("diskarbitrationd");
        protected_names.insert("opendirectoryd");
        protected_names.insert("securityd");
        protected_names.insert("powerd");
        protected_names.insert("coreauthd");
        protected_names.insert("mds");
        protected_names.insert("mds_stores");
        protected_names.insert("fseventsd");
        protected_names.insert("notifyd");
        protected_names.insert("configd");
        protected_names.insert("distnoted");
        protected_names.insert("syslogd");
        protected_names.insert("logd");
        protected_names.insert("kernel");

        Self {
            protected_names,
            previous_cpu: Mutex::new(HashMap::new()),
        }
    }

    fn list_pids(&self) -> Result<Vec<i32>, MemoraError> {
        let num_pids = unsafe { proc_listpids(PROC_ALL_PIDS, 0, std::ptr::null_mut(), 0) };
        if num_pids <= 0 {
            return Err(MemoraError::SystemError(
                "Failed to count pids from proc_listpids".to_string(),
            ));
        }

        let capacity = (num_pids as usize / mem::size_of::<i32>()) + 128;
        let mut pids = vec![0i32; capacity];
        let bytes = (capacity * mem::size_of::<i32>()) as libc::c_int;

        let res = unsafe {
            proc_listpids(
                PROC_ALL_PIDS,
                0,
                pids.as_mut_ptr() as *mut libc::c_void,
                bytes,
            )
        };

        if res <= 0 {
            return Err(MemoraError::SystemError(
                "Failed to fetch pids from proc_listpids".to_string(),
            ));
        }

        let actual_count = (res as usize) / mem::size_of::<i32>();
        pids.truncate(actual_count);
        Ok(pids)
    }

    fn get_process_name_and_path(&self, pid: i32) -> (String, Option<String>) {
        let mut path_buf = [0u8; PROC_PIDPATHINFO_MAXSIZE];
        let path_len = unsafe {
            proc_pidpath(
                pid,
                path_buf.as_mut_ptr() as *mut libc::c_void,
                path_buf.len() as u32,
            )
        };

        let path = if path_len > 0 {
            CStr::from_bytes_until_nul(&path_buf)
                .ok()
                .and_then(|c| c.to_str().ok())
                .map(|s| s.to_string())
        } else {
            None
        };

        let mut name_buf = [0u8; 256];
        let name_len = unsafe {
            proc_name(
                pid,
                name_buf.as_mut_ptr() as *mut libc::c_void,
                name_buf.len() as u32,
            )
        };

        let raw_proc_name = if name_len > 0 {
            CStr::from_bytes_until_nul(&name_buf)
                .ok()
                .and_then(|c| c.to_str().ok())
                .unwrap_or("")
                .to_string()
        } else {
            String::new()
        };

        // Prefer untruncated executable name from path when available, since
        // Darwin kernel proc_name() truncates process names to 31 bytes (MAXCOMLEN).
        let name = if let Some(ref p) = path {
            let file_name = std::path::Path::new(p)
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("");
            if !file_name.is_empty() {
                file_name.to_string()
            } else if !raw_proc_name.is_empty() {
                raw_proc_name
            } else {
                String::new()
            }
        } else if !raw_proc_name.is_empty() {
            raw_proc_name
        } else {
            String::new()
        };

        (name, path)
    }

    fn get_process_architecture(&self, pid: i32) -> ProcessArchitecture {
        if std::env::consts::ARCH != "aarch64" {
            return ProcessArchitecture::Intel;
        }

        // On Apple Silicon (arm64), check if process is translated via Rosetta 2:
        // sysctl([CTL_KERN, KERN_PROC, KERN_PROC_PID, pid], ...)
        // p_flag & P_TRANSLATED (0x00020000)
        let mut mib = [libc::CTL_KERN, libc::KERN_PROC, libc::KERN_PROC_PID, pid];
        let mut kinfo = [0u8; 648];
        let mut size = kinfo.len();

        let res = unsafe {
            libc::sysctl(
                mib.as_mut_ptr(),
                4,
                kinfo.as_mut_ptr() as *mut libc::c_void,
                &mut size,
                std::ptr::null_mut(),
                0,
            )
        };

        if res == 0 && size >= 36 {
            let p_flag = u32::from_ne_bytes([kinfo[32], kinfo[33], kinfo[34], kinfo[35]]);
            const P_TRANSLATED: u32 = 0x00020000;
            if (p_flag & P_TRANSLATED) != 0 {
                ProcessArchitecture::Intel
            } else {
                ProcessArchitecture::AppleSilicon
            }
        } else {
            ProcessArchitecture::AppleSilicon
        }
    }

    fn get_process_metrics(&self, pid: i32) -> ProcessMetrics {
        let now = Instant::now();

        // 1. Try proc_pid_rusage with RUSAGE_INFO_V4
        let mut rusage = RusageInfoV4::default();
        let res = unsafe {
            proc_pid_rusage(
                pid,
                RUSAGE_INFO_V4,
                &mut rusage as *mut RusageInfoV4 as *mut libc::c_void,
            )
        };

        if res == 0 {
            let total_cpu_ns = rusage.ri_user_time.saturating_add(rusage.ri_system_time);

            let cpu_usage = if let Ok(mut map) = self.previous_cpu.lock() {
                let computed = if let Some(&(prev_time, prev_cpu_ns)) = map.get(&pid) {
                    let delta_cpu = total_cpu_ns.saturating_sub(prev_cpu_ns);
                    let delta_wall = now.duration_since(prev_time).as_nanos() as u64;
                    if delta_wall > 0 {
                        let pct = (delta_cpu as f32 / delta_wall as f32) * 100.0;
                        Some(pct.clamp(0.0, 999.0))
                    } else {
                        Some(0.0)
                    }
                } else {
                    None
                };
                map.insert(pid, (now, total_cpu_ns));
                computed
            } else {
                None
            };

            return ProcessMetrics {
                memory_bytes: rusage.ri_phys_footprint,
                resident_bytes: rusage.ri_resident_size,
                peak_memory_bytes: rusage.ri_lifetime_max_phys_footprint,
                pageins: rusage.ri_pageins,
                disk_read_bytes: rusage.ri_diskio_bytesread,
                disk_write_bytes: rusage.ri_diskio_byteswritten,
                cpu_usage,
            };
        }

        // 2. Fallback to PROC_PIDTASKINFO if restricted
        let mut taskinfo = ProcTaskInfo::default();
        let task_res = unsafe {
            proc_pidinfo(
                pid,
                PROC_PIDTASKINFO,
                0,
                &mut taskinfo as *mut ProcTaskInfo as *mut libc::c_void,
                mem::size_of::<ProcTaskInfo>() as libc::c_int,
            )
        };

        if task_res > 0 {
            ProcessMetrics {
                memory_bytes: taskinfo.pti_resident_size,
                resident_bytes: taskinfo.pti_resident_size,
                peak_memory_bytes: taskinfo.pti_resident_size,
                pageins: taskinfo.pti_pageins as u64,
                disk_read_bytes: 0,
                disk_write_bytes: 0,
                cpu_usage: None,
            }
        } else {
            ProcessMetrics {
                memory_bytes: 0,
                resident_bytes: 0,
                peak_memory_bytes: 0,
                pageins: 0,
                disk_read_bytes: 0,
                disk_write_bytes: 0,
                cpu_usage: None,
            }
        }
    }
}

impl Default for MacosProcessCollector {
    fn default() -> Self {
        Self::new()
    }
}

impl ProcessCollector for MacosProcessCollector {
    fn collect_processes(&self) -> Result<Vec<ProcessInfo>, MemoraError> {
        let pids = self.list_pids()?;
        let mut processes = Vec::with_capacity(pids.len());

        for pid in pids {
            if pid <= 0 {
                continue;
            }

            let (name, path) = self.get_process_name_and_path(pid);
            if name.is_empty() && path.is_none() {
                continue;
            }

            let metrics = self.get_process_metrics(pid);
            if metrics.memory_bytes == 0 && metrics.resident_bytes == 0 {
                continue;
            }

            let display_name = if !name.is_empty() {
                name
            } else if let Some(ref p) = path {
                std::path::Path::new(p)
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("Unknown")
                    .to_string()
            } else {
                format!("PID {}", pid)
            };

            let is_protected = self.is_protected_process(pid, &display_name);

            let is_application = if let Some(ref p) = path {
                p.contains("/Applications/") || p.contains(".app/")
            } else {
                false
            };

            let is_system_process = is_protected
                || pid < 100
                || path.as_ref().is_some_and(|p| {
                    p.starts_with("/System/")
                        || p.starts_with("/usr/libexec/")
                        || p.starts_with("/usr/sbin/")
                });

            let architecture = self.get_process_architecture(pid);

            processes.push(ProcessInfo {
                pid,
                name: display_name,
                path,
                memory_bytes: metrics.memory_bytes,
                resident_bytes: metrics.resident_bytes,
                peak_memory_bytes: metrics.peak_memory_bytes,
                pageins: metrics.pageins,
                disk_read_bytes: metrics.disk_read_bytes,
                disk_write_bytes: metrics.disk_write_bytes,
                cpu_usage: metrics.cpu_usage,
                is_application,
                is_system_process,
                is_protected,
                user_id: None,
                bundle_id: None,
                architecture,
            });
        }

        // Clean up terminated PIDs from previous_cpu map
        if let Ok(mut map) = self.previous_cpu.lock() {
            let active_set: HashSet<i32> = processes.iter().map(|p| p.pid).collect();
            map.retain(|pid, _| active_set.contains(pid));
        }

        // Sort default by memory descending
        processes.sort_by_key(|a| std::cmp::Reverse(a.memory_bytes));

        Ok(processes)
    }
}

impl ProcessManager for MacosProcessCollector {
    fn is_protected_process(&self, pid: i32, name: &str) -> bool {
        if pid <= 1 {
            return true;
        }
        self.protected_names.contains(name)
    }

    fn terminate_process(&self, pid: i32) -> Result<(), MemoraError> {
        let (name, _) = self.get_process_name_and_path(pid);
        if self.is_protected_process(pid, &name) {
            return Err(MemoraError::ProtectedProcess { pid, name });
        }

        let ret = unsafe { libc::kill(pid, libc::SIGTERM) };
        if ret != 0 {
            let err = std::io::Error::last_os_error();
            if err.raw_os_error() == Some(libc::ESRCH) {
                return Err(MemoraError::ProcessNotFound(pid));
            } else if err.raw_os_error() == Some(libc::EPERM) {
                return Err(MemoraError::PermissionDenied(pid));
            } else {
                return Err(MemoraError::ProcessInspectionError {
                    pid,
                    message: err.to_string(),
                });
            }
        }

        Ok(())
    }

    fn force_kill_process(&self, pid: i32) -> Result<(), MemoraError> {
        let (name, _) = self.get_process_name_and_path(pid);
        if self.is_protected_process(pid, &name) {
            return Err(MemoraError::ProtectedProcess { pid, name });
        }

        let ret = unsafe { libc::kill(pid, libc::SIGKILL) };
        if ret != 0 {
            let err = std::io::Error::last_os_error();
            if err.raw_os_error() == Some(libc::ESRCH) {
                return Err(MemoraError::ProcessNotFound(pid));
            } else if err.raw_os_error() == Some(libc::EPERM) {
                return Err(MemoraError::PermissionDenied(pid));
            } else {
                return Err(MemoraError::ProcessInspectionError {
                    pid,
                    message: err.to_string(),
                });
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_process_collection() {
        let collector = MacosProcessCollector::new();
        let procs = collector
            .collect_processes()
            .expect("Should collect processes");

        assert!(!procs.is_empty(), "Process list should not be empty");
        let my_pid = std::process::id() as i32;
        let found = procs.iter().any(|p| p.pid == my_pid);
        assert!(found, "Current test process should be in process list");
    }

    #[test]
    fn test_protected_process_refusal() {
        let manager = MacosProcessCollector::new();
        assert!(manager.is_protected_process(1, "launchd"));
        let err = manager.terminate_process(1).unwrap_err();
        match err {
            MemoraError::ProtectedProcess { pid, .. } => assert_eq!(pid, 1),
            _ => panic!("Expected ProtectedProcess error, got {:?}", err),
        }
    }
}
