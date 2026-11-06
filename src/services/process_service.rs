use std::sync::Arc;

use crate::error::MemoraError;
use crate::models::{ProcessFilterCategory, ProcessInfo, ProcessSortColumn, SortDirection};
use crate::system::{ProcessCollector, ProcessManager};

pub struct ProcessService {
    collector: Arc<dyn ProcessCollector>,
    manager: Arc<dyn ProcessManager>,
}

impl ProcessService {
    pub fn new(collector: Arc<dyn ProcessCollector>, manager: Arc<dyn ProcessManager>) -> Self {
        Self { collector, manager }
    }

    pub fn list_processes(&self) -> Result<Vec<ProcessInfo>, MemoraError> {
        self.collector.collect_processes()
    }

    pub fn filter_and_sort(
        processes: &[ProcessInfo],
        search_query: &str,
        category: ProcessFilterCategory,
        sort_column: ProcessSortColumn,
        direction: SortDirection,
    ) -> Vec<ProcessInfo> {
        let query = search_query.trim().to_lowercase();

        let mut filtered: Vec<ProcessInfo> = processes
            .iter()
            .filter(|p| {
                let category_match = match category {
                    ProcessFilterCategory::All => true,
                    ProcessFilterCategory::Applications => p.is_application,
                    ProcessFilterCategory::Background => !p.is_application && !p.is_system_process,
                    ProcessFilterCategory::System => p.is_system_process,
                    ProcessFilterCategory::HighMemory => p.memory_bytes >= 500 * 1024 * 1024,
                    ProcessFilterCategory::HighEnergy => p.cpu_usage.unwrap_or(0.0) >= 10.0,
                };

                if !category_match {
                    return false;
                }
                if query.is_empty() {
                    return true;
                }
                p.name.to_lowercase().contains(&query)
                    || p.pid.to_string().contains(&query)
                    || p.path
                        .as_ref()
                        .is_some_and(|path| path.to_lowercase().contains(&query))
            })
            .cloned()
            .collect();

        filtered.sort_by(|a, b| {
            let ordering = match sort_column {
                ProcessSortColumn::Memory => a.memory_bytes.cmp(&b.memory_bytes),
                ProcessSortColumn::Cpu => {
                    let a_cpu = a.cpu_usage.unwrap_or(0.0);
                    let b_cpu = b.cpu_usage.unwrap_or(0.0);
                    a_cpu
                        .partial_cmp(&b_cpu)
                        .unwrap_or(std::cmp::Ordering::Equal)
                }
                ProcessSortColumn::Name => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
                ProcessSortColumn::Pid => a.pid.cmp(&b.pid),
            };

            match direction {
                SortDirection::Ascending => ordering,
                SortDirection::Descending => ordering.reverse(),
            }
        });

        filtered
    }

    pub fn terminate(&self, pid: i32) -> Result<(), MemoraError> {
        self.manager.terminate_process(pid)
    }

    pub fn force_kill(&self, pid: i32) -> Result<(), MemoraError> {
        self.manager.force_kill_process(pid)
    }

    pub fn is_protected(&self, pid: i32, name: &str) -> bool {
        self.manager.is_protected_process(pid, name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filter_and_sort() {
        let procs = vec![
            ProcessInfo {
                pid: 100,
                name: "Safari".to_string(),
                path: Some("/Applications/Safari.app".to_string()),
                memory_bytes: 500 * 1024 * 1024,
                resident_bytes: 400 * 1024 * 1024,
                peak_memory_bytes: 520 * 1024 * 1024,
                pageins: 100,
                disk_read_bytes: 1024,
                disk_write_bytes: 512,
                cpu_usage: Some(15.0),
                is_application: true,
                is_system_process: false,
                is_protected: false,
                user_id: None,
                bundle_id: None,
                architecture: crate::models::ProcessArchitecture::AppleSilicon,
            },
            ProcessInfo {
                pid: 200,
                name: "syspolicyd".to_string(),
                path: Some("/usr/libexec/syspolicyd".to_string()),
                memory_bytes: 100 * 1024 * 1024,
                resident_bytes: 80 * 1024 * 1024,
                peak_memory_bytes: 100 * 1024 * 1024,
                pageins: 20,
                disk_read_bytes: 0,
                disk_write_bytes: 0,
                cpu_usage: Some(0.1),
                is_application: false,
                is_system_process: true,
                is_protected: true,
                user_id: None,
                bundle_id: None,
                architecture: crate::models::ProcessArchitecture::AppleSilicon,
            },
            ProcessInfo {
                pid: 300,
                name: "Terminal".to_string(),
                path: Some("/Applications/Utilities/Terminal.app".to_string()),
                memory_bytes: 250 * 1024 * 1024,
                resident_bytes: 200 * 1024 * 1024,
                peak_memory_bytes: 260 * 1024 * 1024,
                pageins: 50,
                disk_read_bytes: 2048,
                disk_write_bytes: 1024,
                cpu_usage: Some(0.5),
                is_application: true,
                is_system_process: false,
                is_protected: false,
                user_id: None,
                bundle_id: None,
                architecture: crate::models::ProcessArchitecture::AppleSilicon,
            },
        ];

        // Filter applications only
        let apps = ProcessService::filter_and_sort(
            &procs,
            "",
            ProcessFilterCategory::Applications,
            ProcessSortColumn::Memory,
            SortDirection::Descending,
        );
        assert_eq!(apps.len(), 2);
        assert_eq!(apps[0].name, "Safari");
        assert_eq!(apps[1].name, "Terminal");

        // High memory filter (>500MB)
        let high_mem = ProcessService::filter_and_sort(
            &procs,
            "",
            ProcessFilterCategory::HighMemory,
            ProcessSortColumn::Memory,
            SortDirection::Descending,
        );
        assert_eq!(high_mem.len(), 1);
        assert_eq!(high_mem[0].name, "Safari");

        // Search query
        let search = ProcessService::filter_and_sort(
            &procs,
            "term",
            ProcessFilterCategory::All,
            ProcessSortColumn::Memory,
            SortDirection::Descending,
        );
        assert_eq!(search.len(), 1);
        assert_eq!(search[0].name, "Terminal");

        // High energy filter (>10% CPU)
        let high_energy = ProcessService::filter_and_sort(
            &procs,
            "",
            ProcessFilterCategory::HighEnergy,
            ProcessSortColumn::Cpu,
            SortDirection::Descending,
        );
        assert_eq!(high_energy.len(), 1);
        assert_eq!(high_energy[0].name, "Safari");
    }
}
