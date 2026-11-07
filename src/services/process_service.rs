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
                query.is_empty()
                    || p.name.to_lowercase().contains(&query)
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
                ProcessSortColumn::Cpu => a
                    .cpu_usage
                    .unwrap_or(0.0)
                    .partial_cmp(&b.cpu_usage.unwrap_or(0.0))
                    .unwrap_or(std::cmp::Ordering::Equal),
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

    fn sample_proc(
        pid: i32,
        name: &str,
        path: &str,
        mem_mb: u64,
        cpu: f32,
        is_app: bool,
        is_sys: bool,
    ) -> ProcessInfo {
        ProcessInfo {
            pid,
            name: name.to_string(),
            path: Some(path.to_string()),
            memory_bytes: mem_mb * 1024 * 1024,
            resident_bytes: mem_mb * 1024 * 1024,
            peak_memory_bytes: mem_mb * 1024 * 1024,
            pageins: 10,
            disk_read_bytes: 0,
            disk_write_bytes: 0,
            cpu_usage: Some(cpu),
            is_application: is_app,
            is_system_process: is_sys,
            is_protected: is_sys,
            user_id: None,
            bundle_id: None,
            architecture: crate::models::ProcessArchitecture::AppleSilicon,
        }
    }

    #[test]
    fn test_filter_and_sort() {
        let procs = vec![
            sample_proc(
                100,
                "Safari",
                "/Applications/Safari.app",
                500,
                15.0,
                true,
                false,
            ),
            sample_proc(
                200,
                "syspolicyd",
                "/usr/libexec/syspolicyd",
                100,
                0.1,
                false,
                true,
            ),
            sample_proc(
                300,
                "Terminal",
                "/Applications/Utilities/Terminal.app",
                250,
                0.5,
                true,
                false,
            ),
        ];

        let apps = ProcessService::filter_and_sort(
            &procs,
            "",
            ProcessFilterCategory::Applications,
            ProcessSortColumn::Memory,
            SortDirection::Descending,
        );
        assert_eq!(apps.len(), 2);
        assert_eq!(apps[0].name, "Safari");

        let high_mem = ProcessService::filter_and_sort(
            &procs,
            "",
            ProcessFilterCategory::HighMemory,
            ProcessSortColumn::Memory,
            SortDirection::Descending,
        );
        assert_eq!(high_mem.len(), 1);
        assert_eq!(high_mem[0].name, "Safari");

        let search = ProcessService::filter_and_sort(
            &procs,
            "term",
            ProcessFilterCategory::All,
            ProcessSortColumn::Memory,
            SortDirection::Descending,
        );
        assert_eq!(search.len(), 1);
        assert_eq!(search[0].name, "Terminal");

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
