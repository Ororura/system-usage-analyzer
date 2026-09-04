use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Millivolts(pub i64);
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Milliamps(pub i64);
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Watts(pub f64);
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Celsius(pub f64);

impl Millivolts {
    pub fn volts(self) -> f64 {
        self.0 as f64 / 1000.0
    }
    pub fn power(self, current: Milliamps) -> Watts {
        Watts(self.volts() * current.amps())
    }
}
impl Milliamps {
    /// Positive means charging; negative means energy leaving the battery.
    pub fn amps(self) -> f64 {
        self.0 as f64 / 1000.0
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum BatteryState {
    Charging,
    Discharging,
    FullyCharged,
    NotCharging,
    #[default]
    Unknown,
}

impl BatteryState {
    pub fn from_flags(charging: Option<bool>, full: Option<bool>, ac: Option<bool>) -> Self {
        match (charging, full, ac) {
            (Some(true), _, _) => Self::Charging,
            (_, Some(true), _) => Self::FullyCharged,
            (Some(false), _, Some(false)) => Self::Discharging,
            (Some(false), _, Some(true)) => Self::NotCharging,
            _ => Self::Unknown,
        }
    }
}

#[derive(Debug, Default)]
pub struct BatteryMetrics {
    pub percentage: Option<f64>,
    pub state: BatteryState,
    pub ac_connected: Option<bool>,
    pub time_to_empty: Option<Duration>,
    pub time_to_full: Option<Duration>,
    pub cycle_count: Option<u64>,
    pub maximum_capacity_percent: Option<f64>,
    pub voltage: Option<Millivolts>,
    pub current: Option<Milliamps>,
    pub power: Option<Watts>,
    pub temperature: Option<Celsius>,
}

#[derive(Debug)]
pub enum PowerSnapshot {
    NoBattery,
    Battery(BatteryMetrics),
}

#[derive(Debug, thiserror::Error)]
pub enum PowerError {
    #[error("Battery monitoring is unsupported on this platform")]
    Unsupported,
    #[error("Permission denied: {0}")]
    PermissionDenied(String),
    #[error("System API failure: {0}")]
    SystemApi(String),
    #[error("Power data unavailable: {0}")]
    DataUnavailable(String),
    #[error("Invalid power data: {0}")]
    InvalidData(String),
}

pub trait PowerRepository {
    fn power(&mut self) -> Result<PowerSnapshot, PowerError>;
}
