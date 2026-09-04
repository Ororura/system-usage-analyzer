use crate::domain::process::{ProcessId, ProcessSort};
use std::time::Duration;

pub use crate::application::monitor::View;

#[derive(Debug)]
pub struct Config {
    pub view: View,
    pub refresh_interval: Duration,
    pub process_count: usize,
}

#[derive(Debug, thiserror::Error)]
#[error("{0}\nUsage: system_usage_analyzer [MILLISECONDS] [PROCESS_COUNT]\n       system_usage_analyzer tree [PID] [--sort pid|cpu|memory|name] [--interval MS]\n       system_usage_analyzer power [--interval MS]")]
pub struct ConfigError(String);

impl Config {
    pub fn parse(args: impl IntoIterator<Item = String>) -> Result<Self, ConfigError> {
        let mut args = args.into_iter().peekable();
        let mut config = Self {
            view: View::Dashboard,
            refresh_interval: Duration::from_millis(500),
            process_count: 10,
        };
        match args.peek().map(String::as_str) {
            Some("tree") => {
                args.next();
                let root = if args.peek().is_some_and(|s| !s.starts_with('-')) {
                    args.next()
                        .map(|value| {
                            value
                                .parse::<u32>()
                                .map(ProcessId)
                                .map_err(|_| ConfigError("Invalid PID".into()))
                        })
                        .transpose()?
                } else {
                    None
                };
                let mut sort = ProcessSort::Pid;
                let mut seen_sort = false;
                let mut seen_interval = false;
                while let Some(arg) = args.next() {
                    match arg.as_str() {
                        "--sort" if !seen_sort => {
                            seen_sort = true;
                            sort = match args.next().as_deref() {
                                Some("pid") => ProcessSort::Pid,
                                Some("cpu") => ProcessSort::Cpu,
                                Some("memory") => ProcessSort::Memory,
                                Some("name") => ProcessSort::Name,
                                _ => {
                                    return Err(ConfigError("Invalid or missing sort order".into()))
                                }
                            };
                        }
                        "--interval" if !seen_interval => {
                            seen_interval = true;
                            config.refresh_interval = interval(args.next())?;
                        }
                        _ => {
                            return Err(ConfigError(format!("Unknown or repeated argument: {arg}")))
                        }
                    }
                }
                config.view = View::Tree { root, sort };
            }
            Some("power") => {
                args.next();
                config.view = View::Power;
                config.refresh_interval = Duration::from_secs(2);
                if let Some(arg) = args.next() {
                    if arg != "--interval" {
                        return Err(ConfigError(format!("Unknown argument: {arg}")));
                    }
                    config.refresh_interval = interval(args.next())?;
                }
                reject_extra(args.next())?;
            }
            _ => {
                if let Some(value) = args.next() {
                    config.refresh_interval = interval(Some(value))?;
                }
                if let Some(value) = args.next() {
                    config.process_count = value
                        .parse()
                        .map_err(|_| ConfigError("Invalid process count".into()))?;
                }
                reject_extra(args.next())?;
            }
        }
        Ok(config)
    }
}

fn interval(value: Option<String>) -> Result<Duration, ConfigError> {
    let milliseconds = value
        .and_then(|s| s.parse::<u64>().ok())
        .filter(|&n| n > 0)
        .ok_or_else(|| ConfigError("Interval must be a positive number of milliseconds".into()))?;
    Ok(Duration::from_millis(milliseconds))
}
fn reject_extra(value: Option<String>) -> Result<(), ConfigError> {
    match value {
        Some(value) => Err(ConfigError(format!("Unexpected argument: {value}"))),
        None => Ok(()),
    }
}
