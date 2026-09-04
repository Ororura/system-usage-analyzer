use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ProcessId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessStatus {
    Running,
    Sleeping,
    Idle,
    Stopped,
    Zombie,
    Dead,
    Other,
}

#[derive(Debug)]
pub struct ProcessInfo {
    pub pid: ProcessId,
    pub parent_pid: Option<ProcessId>,
    pub name: String,
    pub executable: Option<PathBuf>,
    pub command: Option<String>,
    pub cpu_usage: f32,
    pub memory_bytes: u64,
    pub status: ProcessStatus,
}

#[derive(Debug)]
pub struct ProcessNode {
    pub process: ProcessInfo,
    pub children: Vec<usize>,
    pub depth: usize,
}

/// An arena avoids recursive traversal and recursive destruction of deep trees.
#[derive(Debug, Default)]
pub struct ProcessTree {
    pub nodes: Vec<ProcessNode>,
    pub roots: Vec<usize>,
}

#[derive(Debug, Clone, Copy, Default)]
pub enum ProcessSort {
    #[default]
    Pid,
    Cpu,
    Memory,
    Name,
}

#[derive(Debug, thiserror::Error)]
pub enum ProcessError {
    #[error("Invalid process snapshot: duplicate PID {0}")]
    DuplicatePid(u32),
    #[error("Process not found: {0}")]
    NotFound(u32),
    #[error("Process data unavailable: {0}")]
    Unavailable(String),
}

pub trait ProcessRepository {
    fn processes(&mut self) -> Result<Vec<ProcessInfo>, ProcessError>;
}
