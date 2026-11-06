use thiserror::Error;

#[derive(Error, Debug)]
pub enum MemoraError {
    #[error("System error: {0}")]
    SystemError(String),

    #[error("Sysctl failed for key '{key}': {message}")]
    SysctlError { key: String, message: String },

    #[error("Mach host statistics error: return code {0}")]
    MachError(i32),

    #[error("Process inspection error for PID {pid}: {message}")]
    ProcessInspectionError { pid: i32, message: String },

    #[error("Process with PID {0} not found")]
    ProcessNotFound(i32),

    #[error("Permission denied when accessing or terminating PID {0}")]
    PermissionDenied(i32),

    #[error("Protected system process (PID {pid}: {name}) cannot be terminated")]
    ProtectedProcess { pid: i32, name: String },

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
}
