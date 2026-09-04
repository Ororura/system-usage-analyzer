use super::ffi::{self, Dictionary};
use crate::{
    domain::power::{BatteryMetrics, BatteryState, PowerError, PowerRepository, PowerSnapshot},
    infrastructure::power_values,
};

pub struct MacOsPowerRepository;
impl PowerRepository for MacOsPowerRepository {
    fn power(&mut self) -> Result<PowerSnapshot, PowerError> {
        let Some(source) = ffi::internal_battery()? else {
            return Ok(PowerSnapshot::NoBattery);
        };
        if source.boolean("Is Present")? == Some(false) {
            return Ok(PowerSnapshot::NoBattery);
        }
        let mut battery = base_metrics(&source);
        // Registry enrichment is optional: failures must not discard IOPS data.
        if let Ok(Some(registry)) = ffi::smart_battery() {
            enrich(&mut battery, &registry);
        }
        Ok(PowerSnapshot::Battery(battery))
    }
}

fn base_metrics(source: &Dictionary) -> BatteryMetrics {
    let number = |key| source.number(key).ok().flatten();
    let flag = |key| source.boolean(key).ok().flatten();
    let ac = match source
        .string("Power Source State")
        .ok()
        .flatten()
        .as_deref()
    {
        Some("AC Power") => Some(true),
        Some("Battery Power") => Some(false),
        _ => None,
    };
    let state = BatteryState::from_flags(flag("Is Charging"), flag("Is Charged"), ac);
    BatteryMetrics {
        percentage: power_values::charge_percent(
            number("Current Capacity"),
            number("Max Capacity"),
        ),
        ac_connected: ac,
        state,
        time_to_empty: (state == BatteryState::Discharging)
            .then(|| power_values::minutes(number("Time to Empty")))
            .flatten(),
        time_to_full: (state == BatteryState::Charging)
            .then(|| power_values::minutes(number("Time to Full Charge")))
            .flatten(),
        ..BatteryMetrics::default()
    }
}

fn enrich(battery: &mut BatteryMetrics, registry: &Dictionary) {
    let number = |key| registry.number(key).ok().flatten();
    // Physical external connection can differ from the active IOPS power source.
    if let Ok(Some(ac)) = registry.boolean("ExternalConnected") {
        battery.ac_connected = Some(ac);
    }
    battery.cycle_count = number("CycleCount").and_then(|n| u64::try_from(n).ok());
    battery.voltage = power_values::voltage(number("Voltage"));
    battery.current = power_values::current(number("Amperage"));
    battery.power = battery
        .voltage
        .zip(battery.current)
        .map(|(v, a)| v.power(a));
    // AppleRawMaxCapacity and DesignCapacity use physical mAh, unlike MaxCapacity
    // which can be normalized to 100 on Apple Silicon. Never mix those scales.
    battery.maximum_capacity_percent =
        power_values::health_percent(number("AppleRawMaxCapacity"), number("DesignCapacity"));
    // macOS AppleSmartBattery publishes Smart Battery temperature in 0.1 Kelvin.
    // https://github.com/apple-oss-distributions/PowerManagement/blob/main/AppleSmartBatteryManager/AppleSmartBattery.cpp
    battery.temperature = power_values::smart_battery_temperature(number("Temperature"));
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn malformed_optional_fields_preserve_valid_metrics() {
        use core_foundation::{
            base::TCFType, boolean::CFBoolean, number::CFNumber, string::CFString,
        };
        let source = Dictionary::from_entries(&[
            (
                CFString::new("Current Capacity"),
                CFNumber::from(71i64).as_CFType(),
            ),
            (
                CFString::new("Max Capacity"),
                CFNumber::from(100i64).as_CFType(),
            ),
            (
                CFString::new("Is Charging"),
                CFBoolean::false_value().as_CFType(),
            ),
            (
                CFString::new("Power Source State"),
                CFString::new("Battery Power").as_CFType(),
            ),
            (
                CFString::new("Time to Empty"),
                CFNumber::from(-1i64).as_CFType(),
            ),
        ]);
        let mut battery = base_metrics(&source);
        let registry = Dictionary::from_entries(&[
            (
                CFString::new("CycleCount"),
                CFString::new("invalid").as_CFType(),
            ),
            (
                CFString::new("Voltage"),
                CFNumber::from(12_000i64).as_CFType(),
            ),
            (
                CFString::new("Amperage"),
                CFNumber::from(-1_000i64).as_CFType(),
            ),
        ]);
        enrich(&mut battery, &registry);
        assert_eq!(battery.percentage, Some(71.0));
        assert_eq!(battery.state, BatteryState::Discharging);
        assert_eq!(battery.time_to_empty, None);
        assert_eq!(battery.cycle_count, None);
        assert_eq!(battery.power, Some(crate::domain::power::Watts(-12.0)));
    }

    #[test]
    #[ignore = "Reads live macOS IOKit; run explicitly on a Mac"]
    fn live_iokit_snapshot() {
        let result = MacOsPowerRepository.power();
        assert!(result.is_ok(), "{result:?}");
    }
}
