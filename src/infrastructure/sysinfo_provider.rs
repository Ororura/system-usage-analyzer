use std::{thread, time::Duration};

use sysinfo::{Pid, System};

use crate::{
    application::monitor::SystemMetricsProvider,
    domain::metrics::{CpuMetrics, MemoryMetrics, ProcessMetrics, SystemSnapshot},
};

pub struct SysinfoMetricsProvider {
    system: System,
    current_pid: Pid,
}

impl SysinfoMetricsProvider {
    pub fn new() -> Self {
        Self {
            system: System::new_all(),
            current_pid: Pid::from(std::process::id() as usize),
        }
    }

    fn process_metrics(pid: Pid, process: &sysinfo::Process) -> ProcessMetrics {
        ProcessMetrics {
            pid: pid.as_u32(),
            name: process.name().to_string_lossy().into_owned(),
            cpu_usage: process.cpu_usage(),
            memory_mb: bytes_to_mb(process.memory()),
        }
    }
}

impl SystemMetricsProvider for SysinfoMetricsProvider {
    fn snapshot(&mut self, process_limit: usize) -> SystemSnapshot {
        self.system.refresh_all();

        thread::sleep(sysinfo::MINIMUM_CPU_UPDATE_INTERVAL);
        self.system.refresh_cpu_all();

        let application = self
            .system
            .process(self.current_pid)
            .map(|process| Self::process_metrics(self.current_pid, process));

        let cpus = self
            .system
            .cpus()
            .iter()
            .map(|cpu| CpuMetrics {
                name: cpu.name().to_owned(),
                usage: cpu.cpu_usage(),
            })
            .collect();

        let memory = MemoryMetrics {
            used_gb: bytes_to_gb(self.system.used_memory()),
            total_gb: bytes_to_gb(self.system.total_memory()),
        };

        let mut processes = self
            .system
            .processes()
            .iter()
            .map(|(pid, process)| Self::process_metrics(*pid, process))
            .collect::<Vec<_>>();

        processes.sort_by(|a, b| {
            b.memory_mb
                .partial_cmp(&a.memory_mb)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let top_processes = processes.into_iter().take(process_limit).collect();

        SystemSnapshot {
            application,
            cpus,
            memory,
            top_processes,
        }
    }
}

fn bytes_to_mb(bytes: u64) -> f64 {
    bytes as f64 / 1024.0 / 1024.0
}

fn bytes_to_gb(bytes: u64) -> f64 {
    bytes as f64 / 1024.0 / 1024.0 / 1024.0
}