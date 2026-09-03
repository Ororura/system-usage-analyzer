use crate::domain::metrics::SystemSnapshot;

pub trait SystemMetricsProvider {
    fn snapshot(&mut self, process_limit: usize) -> SystemSnapshot;
}

pub trait SnapshotRenderer {
    fn render(&self, snapshot: &SystemSnapshot);
}

pub struct SystemMonitor<P, R>
where
    P: SystemMetricsProvider,
    R: SnapshotRenderer,
{
    provider: P,
    renderer: R,
    process_limit: usize,
}

impl<P, R> SystemMonitor<P, R>
where
    P: SystemMetricsProvider,
    R: SnapshotRenderer,
{
    pub fn new(provider: P, renderer: R, process_limit: usize) -> Self {
        Self {
            provider,
            renderer,
            process_limit,
        }
    }

    pub fn tick(&mut self) {
        let snapshot = self.provider.snapshot(self.process_limit);
        self.renderer.render(&snapshot);
    }
}
