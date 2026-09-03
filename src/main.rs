mod application;
mod domain;
mod infrastructure;

use std::{
    env,
    thread,
    time::Duration,
};

use application::monitor::SystemMonitor;
use infrastructure::{
    sysinfo_provider::SysinfoMetricsProvider,
    terminal_renderer::TerminalRenderer,
};

const DEFAULT_REFRESH_INTERVAL_MS: u64 = 500;
const DEFAULT_PROCESS_COUNT: usize = 10;

fn main() {
    let config = Config::from_args();

    let provider = SysinfoMetricsProvider::new();
    let renderer = TerminalRenderer;

    let mut monitor = SystemMonitor::new(
        provider,
        renderer,
        config.process_count,
    );

    let delay = Duration::from_millis(config.refresh_interval_ms);

    loop {
        monitor.tick();
        thread::sleep(delay);
    }
}

struct Config {
    refresh_interval_ms: u64,
    process_count: usize,
}

impl Config {
    fn from_args() -> Self {
        let mut args = env::args().skip(1);

        let refresh_interval_ms = args
            .next()
            .and_then(|value| value.parse().ok())
            .unwrap_or(DEFAULT_REFRESH_INTERVAL_MS);

        let process_count = args
            .next()
            .and_then(|value| value.parse().ok())
            .unwrap_or(DEFAULT_PROCESS_COUNT);

        Self {
            refresh_interval_ms,
            process_count,
        }
    }
}