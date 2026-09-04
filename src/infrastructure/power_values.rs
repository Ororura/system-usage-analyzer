//! Pure decoding rules for platform data. Unknown/sentinel values stay absent.
use crate::domain::power::{Celsius, Milliamps, Millivolts};
use std::time::Duration;

pub fn charge_percent(current: Option<i64>, maximum: Option<i64>) -> Option<f64> {
    let (current, maximum) = current.zip(maximum)?;
    (current >= 0 && maximum > 0 && current <= maximum)
        .then(|| current as f64 / maximum as f64 * 100.0)
}

pub fn health_percent(full_mah: Option<i64>, design_mah: Option<i64>) -> Option<f64> {
    let (full, design) = full_mah.zip(design_mah)?;
    // Reject normalized percentages and malformed capacities. Health may exceed
    // 100% slightly on a new battery; preserve it instead of inventing a clamp.
    (full > 100 && design > 100 && full <= design.saturating_mul(2))
        .then(|| full as f64 / design as f64 * 100.0)
}

pub fn minutes(value: Option<i64>) -> Option<Duration> {
    let minutes = u64::try_from(value?).ok()?;
    if minutes == 65535 {
        return None;
    }
    minutes.checked_mul(60).map(Duration::from_secs)
}

pub fn voltage(value: Option<i64>) -> Option<Millivolts> {
    value.filter(|n| (1..=100_000).contains(n)).map(Millivolts)
}
pub fn current(value: Option<i64>) -> Option<Milliamps> {
    // Signed CFNumber decoding preserves negative discharging current.
    value
        .filter(|n| (-100_000..=100_000).contains(n))
        .map(Milliamps)
}
pub fn smart_battery_temperature(value: Option<i64>) -> Option<Celsius> {
    // Do not infer other encodings from magnitude. Values outside the supported
    // physical range are unavailable, rather than interpreted as centi-Celsius.
    let celsius = value? as f64 / 10.0 - 273.15;
    (-40.0..=100.0)
        .contains(&celsius)
        .then_some(Celsius(celsius))
}
