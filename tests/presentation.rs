use std::io;
use system_usage_analyzer::{
    application::{
        monitor::{MonitorFrame, SnapshotRenderer},
        process::build_tree,
    },
    config::{Config, View},
    domain::{power::*, process::*},
    infrastructure::terminal_renderer::TerminalRenderer,
};

fn config(args: &[&str]) -> Result<Config, system_usage_analyzer::config::ConfigError> {
    Config::parse(args.iter().map(|s| s.to_string()))
}
#[test]
fn cli_modes_and_legacy_arguments() {
    let c = config(&["750", "25"]).unwrap();
    assert_eq!(c.refresh_interval.as_millis(), 750);
    assert_eq!(c.process_count, 25);
    assert!(matches!(c.view, View::Dashboard));
    assert_eq!(config(&[]).unwrap().refresh_interval.as_millis(), 500);
    assert!(matches!(
        config(&["tree", "123", "--sort", "cpu", "--interval", "1000"])
            .unwrap()
            .view,
        View::Tree {
            root: Some(ProcessId(123)),
            sort: ProcessSort::Cpu
        }
    ));
    assert!(matches!(
        config(&["tree"]).unwrap().view,
        View::Tree { root: None, .. }
    ));
    assert!(matches!(config(&["power"]).unwrap().view, View::Power));
    assert_eq!(config(&["power"]).unwrap().refresh_interval.as_secs(), 2);
}
#[test]
fn cli_rejects_invalid_input() {
    for args in [
        &["tree", "bad"][..],
        &["tree", "--sort", "bad"],
        &["tree", "--sort"],
        &["tree", "--interval", "0"],
        &["power", "extra"],
        &["power", "--interval"],
        &["-1"],
        &["500", "ten"],
        &["500", "10", "extra"],
        &["tree", "--interval", "2", "--interval", "3"],
    ] {
        assert!(config(args).is_err(), "{args:?}");
    }
}
fn render(frame: MonitorFrame<'_>) -> String {
    let mut bytes = Vec::new();
    TerminalRenderer::new(&mut bytes).render(frame).unwrap();
    String::from_utf8(bytes).unwrap()
}
#[test]
fn power_unknowns_absent_battery_errors_and_direction() {
    let empty = render(MonitorFrame::Power(&Ok(PowerSnapshot::NoBattery)));
    assert!(empty.contains("Not available on this device"));
    let unknown = render(MonitorFrame::Power(&Ok(PowerSnapshot::Battery(
        BatteryMetrics::default(),
    ))));
    assert!(unknown.contains("N/A"));
    assert!(unknown.contains("Unknown"));
    assert!(!unknown.contains("0.0 W"));
    let metrics = BatteryMetrics {
        power: Some(Watts(-12.8)),
        current: Some(Milliamps(-1000)),
        ..BatteryMetrics::default()
    };
    let output = render(MonitorFrame::Power(&Ok(PowerSnapshot::Battery(metrics))));
    assert!(output.contains("12.8 W"));
    assert!(output.contains("-1.000 A"));
    assert!(render(MonitorFrame::Power(&Err(PowerError::Unsupported))).contains("unsupported"));
}
#[test]
fn tree_branches_unicode_and_terminal_control_sanitization() {
    let p = |pid, parent: Option<u32>, name: &str| ProcessInfo {
        pid: ProcessId(pid),
        parent_pid: parent.map(ProcessId),
        name: name.into(),
        executable: None,
        command: None,
        cpu_usage: 0.5,
        memory_bytes: 1_048_576,
        status: ProcessStatus::Running,
    };
    let tree = build_tree(
        vec![
            p(1, None, "Code"),
            p(2, Some(1), "Дочерний"),
            p(3, Some(2), "node\x1b[2J\n"),
            p(4, Some(1), "git"),
        ],
        None,
        ProcessSort::Pid,
    );
    let output = render(MonitorFrame::Tree(&tree));
    assert!(output.contains("├── Дочерний"));
    assert!(output.contains("│   └── node�[2J�"));
    assert!(output.contains("└── git"));
    assert!(output.contains("1.0 MB"));
    assert!(render(MonitorFrame::Tree(&Err(ProcessError::NotFound(99))))
        .contains("Process not found: 99"));
}
struct BrokenWriter;
impl io::Write for BrokenWriter {
    fn write(&mut self, _: &[u8]) -> io::Result<usize> {
        Err(io::ErrorKind::BrokenPipe.into())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
#[test]
fn output_errors_propagate_without_panic() {
    let error = TerminalRenderer::new(BrokenWriter)
        .render(MonitorFrame::Power(&Ok(PowerSnapshot::NoBattery)))
        .unwrap_err();
    assert_eq!(error.kind(), io::ErrorKind::BrokenPipe);
}
