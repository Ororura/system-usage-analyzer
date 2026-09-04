use std::{env, io, thread, time::Instant};
use system_usage_analyzer::{
    application::monitor::SystemMonitor,
    config::{Config, View},
    infrastructure::{
        power_provider::PlatformPowerRepository, sysinfo_provider::SysinfoMetricsProvider,
        terminal_renderer::TerminalRenderer,
    },
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::parse(env::args().skip(1))?;
    let provider = SysinfoMetricsProvider::new();
    let interval = match config.view {
        View::Power => config
            .refresh_interval
            .max(std::time::Duration::from_secs(2)),
        _ => config
            .refresh_interval
            .max(sysinfo::MINIMUM_CPU_UPDATE_INTERVAL),
    };
    let stdout = io::stdout();
    let renderer = TerminalRenderer::new(stdout.lock());
    let mut monitor = SystemMonitor::new(
        provider,
        PlatformPowerRepository,
        renderer,
        config.process_count,
        config.view,
    );
    if !matches!(config.view, View::Power) {
        thread::sleep(sysinfo::MINIMUM_CPU_UPDATE_INTERVAL);
    }
    loop {
        let start = Instant::now();
        if let Err(error) = monitor.tick(start) {
            if error.kind() == io::ErrorKind::BrokenPipe {
                return Ok(());
            }
            return Err(error.into());
        }
        thread::sleep(interval.saturating_sub(start.elapsed()));
    }
}
