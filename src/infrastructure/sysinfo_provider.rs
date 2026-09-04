use crate::{
    application::monitor::SystemMetricsProvider,
    domain::{
        metrics::{CpuMetrics, MemoryMetrics, ProcessMetrics, SystemSnapshot},
        process::{ProcessError, ProcessId, ProcessInfo, ProcessRepository, ProcessStatus},
    },
};
use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind};

pub struct SysinfoMetricsProvider {
    system: System,
    current_pid: Pid,
}

impl Default for SysinfoMetricsProvider {
    fn default() -> Self {
        Self::new()
    }
}
impl SysinfoMetricsProvider {
    /// Seeds CPU counters without sleeping. The caller waits before the first sample.
    pub fn new() -> Self {
        let mut provider = Self {
            system: System::new(),
            current_pid: Pid::from_u32(std::process::id()),
        };
        provider.refresh_processes();
        provider.system.refresh_cpu_all();
        provider
    }
    fn refresh_processes(&mut self) {
        self.system.refresh_processes_specifics(
            ProcessesToUpdate::All,
            true,
            ProcessRefreshKind::nothing()
                .with_cpu()
                .with_memory()
                .with_exe(UpdateKind::OnlyIfNotSet)
                .with_cmd(UpdateKind::OnlyIfNotSet),
        );
    }
    fn process_info(pid: Pid, process: &sysinfo::Process) -> ProcessInfo {
        let status = match process.status() {
            sysinfo::ProcessStatus::Run => ProcessStatus::Running,
            sysinfo::ProcessStatus::Sleep => ProcessStatus::Sleeping,
            sysinfo::ProcessStatus::Idle => ProcessStatus::Idle,
            sysinfo::ProcessStatus::Stop | sysinfo::ProcessStatus::Tracing => {
                ProcessStatus::Stopped
            }
            sysinfo::ProcessStatus::Zombie => ProcessStatus::Zombie,
            sysinfo::ProcessStatus::Dead => ProcessStatus::Dead,
            _ => ProcessStatus::Other,
        };
        ProcessInfo {
            pid: ProcessId(pid.as_u32()),
            parent_pid: process.parent().map(|p| ProcessId(p.as_u32())),
            name: process.name().to_string_lossy().into_owned(),
            executable: process.exe().map(ToOwned::to_owned),
            command: (!process.cmd().is_empty()).then(|| {
                process
                    .cmd()
                    .iter()
                    .map(|s| s.to_string_lossy())
                    .collect::<Vec<_>>()
                    .join(" ")
            }),
            cpu_usage: process.cpu_usage(),
            memory_bytes: process.memory(),
            status,
        }
    }
}

impl ProcessRepository for SysinfoMetricsProvider {
    fn processes(&mut self) -> Result<Vec<ProcessInfo>, ProcessError> {
        self.refresh_processes();
        Ok(self
            .system
            .processes()
            .iter()
            .map(|(&pid, p)| Self::process_info(pid, p))
            .collect())
    }
}

impl SystemMetricsProvider for SysinfoMetricsProvider {
    fn snapshot(&mut self, process_limit: usize) -> SystemSnapshot {
        self.refresh_processes();
        self.system.refresh_cpu_all();
        self.system.refresh_memory();
        let mut processes: Vec<_> = self.system.processes().iter().collect();
        processes.sort_unstable_by(|(a_pid, a), (b_pid, b)| {
            b.memory().cmp(&a.memory()).then_with(|| a_pid.cmp(b_pid))
        });
        let metrics = |pid: Pid, p: &sysinfo::Process| ProcessMetrics {
            pid: pid.as_u32(),
            name: p.name().to_string_lossy().into_owned(),
            cpu_usage: p.cpu_usage(),
            memory_mb: p.memory() as f64 / 1_048_576.0,
        };
        SystemSnapshot {
            application: self
                .system
                .process(self.current_pid)
                .map(|p| metrics(self.current_pid, p)),
            cpus: self
                .system
                .cpus()
                .iter()
                .map(|cpu| CpuMetrics {
                    name: cpu.name().to_owned(),
                    usage: cpu.cpu_usage(),
                })
                .collect(),
            memory: MemoryMetrics {
                used_gb: self.system.used_memory() as f64 / 1_073_741_824.0,
                total_gb: self.system.total_memory() as f64 / 1_073_741_824.0,
            },
            top_processes: processes
                .into_iter()
                .take(process_limit)
                .map(|(&pid, p)| metrics(pid, p))
                .collect(),
        }
    }
}
