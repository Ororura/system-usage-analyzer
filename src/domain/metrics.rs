#[derive(Debug, Clone)]
pub struct ProcessMetrics {
    pub pid: u32,
    pub name: String,
    pub cpu_usage: f32,
    pub memory_mb: f64,
}

#[derive(Debug, Clone)]
pub struct CpuMetrics {
    pub name: String,
    pub usage: f32,
}

#[derive(Debug, Clone)]
pub struct MemoryMetrics {
    pub used_gb: f64,
    pub total_gb: f64,
}

#[derive(Debug, Clone)]
pub struct SystemSnapshot {
    pub application: Option<ProcessMetrics>,
    pub cpus: Vec<CpuMetrics>,
    pub memory: MemoryMetrics,
    pub top_processes: Vec<ProcessMetrics>,
}
