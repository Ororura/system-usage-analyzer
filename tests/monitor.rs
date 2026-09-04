use std::{
    cell::Cell,
    io,
    rc::Rc,
    time::{Duration, Instant},
};
use system_usage_analyzer::{
    application::monitor::*,
    config::View,
    domain::{
        metrics::{MemoryMetrics, SystemSnapshot},
        power::*,
        process::*,
    },
};

struct SystemFake {
    samples: Rc<Cell<usize>>,
    trees: Rc<Cell<usize>>,
}
impl SystemMetricsProvider for SystemFake {
    fn snapshot(&mut self, _: usize) -> SystemSnapshot {
        self.samples.set(self.samples.get() + 1);
        SystemSnapshot {
            application: None,
            cpus: vec![],
            memory: MemoryMetrics {
                used_gb: 0.0,
                total_gb: 0.0,
            },
            top_processes: vec![],
        }
    }
}
impl ProcessRepository for SystemFake {
    fn processes(&mut self) -> Result<Vec<ProcessInfo>, ProcessError> {
        self.trees.set(self.trees.get() + 1);
        Ok(vec![])
    }
}
struct PowerFake(Rc<Cell<usize>>);
impl PowerRepository for PowerFake {
    fn power(&mut self) -> Result<PowerSnapshot, PowerError> {
        self.0.set(self.0.get() + 1);
        Err(PowerError::DataUnavailable("temporary".into()))
    }
}
struct RendererFake {
    frames: Rc<Cell<usize>>,
    view: View,
}
impl SnapshotRenderer for RendererFake {
    fn render(&mut self, frame: MonitorFrame<'_>) -> io::Result<()> {
        assert!(matches!(
            (self.view, frame),
            (View::Dashboard, MonitorFrame::Dashboard { .. })
                | (View::Power, MonitorFrame::Power(_))
                | (View::Tree { .. }, MonitorFrame::Tree(_))
        ));
        self.frames.set(self.frames.get() + 1);
        Ok(())
    }
}
#[test]
fn modes_sample_only_needed_sources_and_power_errors_do_not_stop_rendering() {
    for (view, expected_system, expected_trees, expected_power) in [
        (View::Dashboard, 3, 0, 2),
        (View::Power, 0, 0, 2),
        (
            View::Tree {
                root: None,
                sort: ProcessSort::Pid,
            },
            0,
            3,
            0,
        ),
    ] {
        let samples = Rc::new(Cell::new(0));
        let trees = Rc::new(Cell::new(0));
        let power = Rc::new(Cell::new(0));
        let frames = Rc::new(Cell::new(0));
        let mut monitor = SystemMonitor::new(
            SystemFake {
                samples: samples.clone(),
                trees: trees.clone(),
            },
            PowerFake(power.clone()),
            RendererFake {
                frames: frames.clone(),
                view,
            },
            10,
            view,
        );
        let start = Instant::now();
        for offset in [0, 1, 2] {
            monitor.tick(start + Duration::from_secs(offset)).unwrap();
        }
        assert_eq!(samples.get(), expected_system);
        assert_eq!(trees.get(), expected_trees);
        assert_eq!(power.get(), expected_power);
        assert_eq!(frames.get(), 3);
    }
}
