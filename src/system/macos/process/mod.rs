mod ffi;

use std::collections::{HashMap, HashSet};
use std::mem;
use std::sync::Mutex;
use std::time::Instant;

use crate::error::MemoraError;
use crate::models::{ProcessArchitecture, ProcessInfo};
use crate::system::process::{ProcessCollector, ProcessManager};

use self::ffi::{
    list_all_pids, proc_pid_rusage, proc_pidinfo, query_name_and_path, query_process_architecture,
    ProcTaskInfo, RusageInfoV4, PROC_PIDTASKINFO, RUSAGE_INFO_V4,
};

#[derive(Default)]
struct ProcessMetrics {
    memory_bytes: u64,
    resident_bytes: u64,
    peak_memory_bytes: u64,
    pageins: u64,
    disk_read_bytes: u64,
    disk_write_bytes: u64,
    cpu_usage: Option<f32>,
}

#[derive(Default)]
struct ProcessStateCache {
    previous_cpu: HashMap<i32, (Instant, u64)>,
    arch_cache: HashMap<i32, ProcessArchitecture>,
}

const PROTECTED_LIST: &str = "kernel_task,launchd,WindowServer,loginwindow,syspolicyd,\
diskarbitrationd,opendirectoryd,securityd,powerd,coreauthd,mds,mds_stores,fseventsd,\
notifyd,configd,distnoted,syslogd,logd,kernel";

pub struct MacosProcessCollector {
    protected_names: HashSet<&'static str>,
    state: Mutex<ProcessStateCache>,
}

impl Default for MacosProcessCollector {
    fn default() -> Self {
        Self::new()
    }
}

impl MacosProcessCollector {
    pub fn new() -> Self {
        Self {
            protected_names: PROTECTED_LIST.split(',').collect(),
            state: Mutex::new(ProcessStateCache::default()),
        }
    }

    fn get_cached_architecture(&self, pid: i32) -> ProcessArchitecture {
        let Ok(mut g) = self.state.lock() else {
            return query_process_architecture(pid);
        };
        *g.arch_cache
            .entry(pid)
            .or_insert_with(|| query_process_architecture(pid))
    }

    fn get_process_metrics(&self, pid: i32) -> ProcessMetrics {
        let now = Instant::now();
        let mut rusage = RusageInfoV4::default();
        let r_ptr = &mut rusage as *mut RusageInfoV4 as *mut libc::c_void;
        if unsafe { proc_pid_rusage(pid, RUSAGE_INFO_V4, r_ptr) } == 0 {
            let total_cpu_ns = rusage.ri_user_time.saturating_add(rusage.ri_system_time);
            let cpu_usage = self.state.lock().ok().and_then(|mut guard| {
                let computed = guard.previous_cpu.get(&pid).map(|&(prev_t, prev_ns)| {
                    let delta_cpu = total_cpu_ns.saturating_sub(prev_ns);
                    let delta_wall = now.duration_since(prev_t).as_nanos() as u64;
                    if delta_wall > 0 {
                        ((delta_cpu as f32 / delta_wall as f32) * 100.0).clamp(0.0, 999.0)
                    } else {
                        0.0
                    }
                });
                guard.previous_cpu.insert(pid, (now, total_cpu_ns));
                computed
            });

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

        let mut ti = ProcTaskInfo::default();
        let t_ptr = &mut ti as *mut ProcTaskInfo as *mut libc::c_void;
        let t_sz = mem::size_of::<ProcTaskInfo>() as libc::c_int;
        if unsafe { proc_pidinfo(pid, PROC_PIDTASKINFO, 0, t_ptr, t_sz) } > 0 {
            ProcessMetrics {
                memory_bytes: ti.pti_resident_size,
                resident_bytes: ti.pti_resident_size,
                peak_memory_bytes: ti.pti_resident_size,
                pageins: ti.pti_pageins as u64,
                ..ProcessMetrics::default()
            }
        } else {
            ProcessMetrics::default()
        }
    }

    fn send_signal(&self, pid: i32, sig: libc::c_int) -> Result<(), MemoraError> {
        let (name, _) = query_name_and_path(pid);
        if self.is_protected_process(pid, &name) {
            return Err(MemoraError::ProtectedProcess { pid, name });
        }
        if unsafe { libc::kill(pid, sig) } != 0 {
            let err = std::io::Error::last_os_error();
            let message = err.to_string();
            return Err(match err.raw_os_error() {
                Some(libc::ESRCH) => MemoraError::ProcessNotFound(pid),
                Some(libc::EPERM) => MemoraError::PermissionDenied(pid),
                _ => MemoraError::ProcessInspectionError { pid, message },
            });
        }
        Ok(())
    }
}

impl ProcessCollector for MacosProcessCollector {
    fn collect_processes(&self) -> Result<Vec<ProcessInfo>, MemoraError> {
        let pids = list_all_pids()?;
        let mut processes = Vec::with_capacity(pids.len());

        for pid in pids.into_iter().filter(|&p| p > 0) {
            let (name, path) = query_name_and_path(pid);
            if name.is_empty() && path.is_none() {
                continue;
            }
            let m = self.get_process_metrics(pid);
            if m.memory_bytes == 0 && m.resident_bytes == 0 {
                continue;
            }
            let display_name = if name.is_empty() {
                format!("PID {}", pid)
            } else {
                name
            };
            let is_protected = self.is_protected_process(pid, &display_name);
            let is_application = path
                .as_ref()
                .is_some_and(|p| p.contains("/Applications/") || p.contains(".app/"));
            let sys_pfx = ["/System/", "/usr/libexec/", "/usr/sbin/"];
            let is_sys_path = path
                .as_ref()
                .is_some_and(|p| sys_pfx.iter().any(|s| p.starts_with(s)));

            processes.push(ProcessInfo {
                pid,
                name: display_name,
                path,
                memory_bytes: m.memory_bytes,
                resident_bytes: m.resident_bytes,
                peak_memory_bytes: m.peak_memory_bytes,
                pageins: m.pageins,
                disk_read_bytes: m.disk_read_bytes,
                disk_write_bytes: m.disk_write_bytes,
                cpu_usage: m.cpu_usage,
                is_application,
                is_system_process: is_protected || pid < 100 || is_sys_path,
                is_protected,
                user_id: None,
                bundle_id: None,
                architecture: self.get_cached_architecture(pid),
            });
        }

        if let Ok(mut guard) = self.state.lock() {
            let active: HashSet<i32> = processes.iter().map(|p| p.pid).collect();
            guard.previous_cpu.retain(|pid, _| active.contains(pid));
            guard.arch_cache.retain(|pid, _| active.contains(pid));
        }

        processes.sort_by_key(|a| std::cmp::Reverse(a.memory_bytes));
        Ok(processes)
    }
}

impl ProcessManager for MacosProcessCollector {
    fn is_protected_process(&self, pid: i32, name: &str) -> bool {
        pid <= 1 || self.protected_names.contains(name)
    }
    fn terminate_process(&self, pid: i32) -> Result<(), MemoraError> {
        self.send_signal(pid, libc::SIGTERM)
    }
    fn force_kill_process(&self, pid: i32) -> Result<(), MemoraError> {
        self.send_signal(pid, libc::SIGKILL)
    }
}
