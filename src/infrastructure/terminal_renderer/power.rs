use super::{safe_text, section};
use crate::domain::power::{BatteryMetrics, BatteryState, PowerError, PowerSnapshot};
use std::{
    io::{self, Write},
    time::Duration,
};

pub(super) fn render(
    w: &mut impl Write,
    result: &Result<PowerSnapshot, PowerError>,
) -> io::Result<()> {
    section(w, "Battery")?;
    match result {
        Ok(PowerSnapshot::NoBattery) => writeln!(w, "Not available on this device"),
        Err(error) => writeln!(w, "{}", safe_text(&error.to_string())),
        Ok(PowerSnapshot::Battery(battery)) => metrics(w, battery),
    }
}
fn metrics(w: &mut impl Write, b: &BatteryMetrics) -> io::Result<()> {
    row(w, "Charge", b.percentage.map(|p| format!("{p:.0}%")))?;
    row(
        w,
        "State",
        Some(
            match b.state {
                BatteryState::Charging => "Charging",
                BatteryState::Discharging => "Discharging",
                BatteryState::FullyCharged => "Fully charged",
                BatteryState::NotCharging => "Not charging",
                BatteryState::Unknown => "Unknown",
            }
            .into(),
        ),
    )?;
    row(
        w,
        "AC connected",
        b.ac_connected
            .map(|ac| if ac { "Yes" } else { "No" }.into()),
    )?;
    row(
        w,
        "Battery power",
        b.power.map(|p| format!("{:.1} W", p.0.abs())),
    )?;
    if b.state == BatteryState::Charging {
        row(w, "Time to full", b.time_to_full.map(format_time))?;
    } else {
        row(w, "Time remaining", b.time_to_empty.map(format_time))?;
    }
    row(
        w,
        "Voltage",
        b.voltage.map(|v| format!("{:.2} V", v.volts())),
    )?;
    row(
        w,
        "Current",
        b.current.map(|a| format!("{:+.3} A", a.amps())),
    )?;
    section(w, "Health")?;
    row(
        w,
        "Maximum capacity",
        b.maximum_capacity_percent.map(|p| format!("{p:.0}%")),
    )?;
    row(w, "Cycle count", b.cycle_count.map(|n| n.to_string()))?;
    row(
        w,
        "Temperature",
        b.temperature.map(|t| format!("{:.1} °C", t.0)),
    )
}
fn row(w: &mut impl Write, label: &str, value: Option<String>) -> io::Result<()> {
    writeln!(w, "{label:<20}{}", value.as_deref().unwrap_or("N/A"))
}
fn format_time(time: Duration) -> String {
    format!(
        "{}h {:02}m",
        time.as_secs() / 3600,
        time.as_secs() / 60 % 60
    )
}
