use super::{power::GetPowerMetrics, process::GetProcessTree};
use crate::domain::{
    metrics::SystemSnapshot,
    power::{PowerError, PowerRepository, PowerSnapshot},
    process::{ProcessError, ProcessId, ProcessRepository, ProcessSort, ProcessTree},
};
use std::{io, time::Instant};

#[derive(Debug, Clone, Copy)]
pub enum View {
    Dashboard,
    Tree {
        root: Option<ProcessId>,
        sort: ProcessSort,
    },
    Power,
}

pub trait SystemMetricsProvider {
    fn snapshot(&mut self, process_limit: usize) -> SystemSnapshot;
}

pub enum MonitorFrame<'a> {
    Dashboard {
        system: &'a SystemSnapshot,
        power: &'a Result<PowerSnapshot, PowerError>,
    },
    Tree(&'a Result<ProcessTree, ProcessError>),
    Power(&'a Result<PowerSnapshot, PowerError>),
}

pub trait SnapshotRenderer {
    fn render(&mut self, frame: MonitorFrame<'_>) -> io::Result<()>;
}

pub struct SystemMonitor<P, B, R> {
    provider: P,
    power: GetPowerMetrics<B>,
    renderer: R,
    process_limit: usize,
    view: View,
}

impl<P: SystemMetricsProvider + ProcessRepository, B: PowerRepository, R: SnapshotRenderer>
    SystemMonitor<P, B, R>
{
    pub fn new(provider: P, power: B, renderer: R, process_limit: usize, view: View) -> Self {
        Self {
            provider,
            power: GetPowerMetrics::new(power),
            renderer,
            process_limit,
            view,
        }
    }
    pub fn tick(&mut self, now: Instant) -> io::Result<()> {
        match self.view {
            View::Dashboard => {
                let power = self.power.execute(now);
                let system = self.provider.snapshot(self.process_limit);
                self.renderer.render(MonitorFrame::Dashboard {
                    system: &system,
                    power,
                })
            }
            View::Tree { root, sort } => {
                let tree = GetProcessTree::execute(&mut self.provider, root, sort);
                self.renderer.render(MonitorFrame::Tree(&tree))
            }
            View::Power => self
                .renderer
                .render(MonitorFrame::Power(self.power.execute(now))),
        }
    }
}
