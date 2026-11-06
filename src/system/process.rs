use crate::error::MemoraError;
use crate::models::ProcessInfo;

pub trait ProcessCollector: Send + Sync {
    fn collect_processes(&self) -> Result<Vec<ProcessInfo>, MemoraError>;
}

pub trait ProcessManager: Send + Sync {
    fn terminate_process(&self, pid: i32) -> Result<(), MemoraError>;
    fn force_kill_process(&self, pid: i32) -> Result<(), MemoraError>;
    fn is_protected_process(&self, pid: i32, name: &str) -> bool;
}
