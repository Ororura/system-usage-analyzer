use std::{
    collections::VecDeque,
    time::{Duration, Instant},
};
use system_usage_analyzer::{
    application::power::GetPowerMetrics, domain::power::*, infrastructure::power_values::*,
};

#[test]
fn electrical_units_and_signed_power() {
    assert_eq!(Millivolts(12_000).volts(), 12.0);
    assert_eq!(Milliamps(1_000).amps(), 1.0);
    assert_eq!(Millivolts(12_000).power(Milliamps(1_000)), Watts(12.0));
    assert_eq!(Millivolts(12_000).power(Milliamps(-1_000)), Watts(-12.0));
    assert_eq!(Millivolts(12_000).power(Milliamps(0)), Watts(0.0));
}
#[test]
fn absent_invalid_and_zero_values() {
    assert_eq!(voltage(None), None);
    assert_eq!(current(None), None);
    assert_eq!(voltage(Some(0)), None);
    assert_eq!(voltage(Some(-1)), None);
    assert_eq!(current(Some(i64::MAX)), None);
    assert_eq!(current(Some(0)), Some(Milliamps(0)));
    let b = BatteryMetrics::default();
    assert_eq!(b.power, None);
    assert_eq!(b.percentage, None);
    assert_eq!(b.state, BatteryState::Unknown);
}
#[test]
fn state_mapping_including_ac_without_charging() {
    for (charging, full, ac, expected) in [
        (Some(true), Some(false), Some(true), BatteryState::Charging),
        (
            Some(false),
            Some(false),
            Some(false),
            BatteryState::Discharging,
        ),
        (
            Some(false),
            Some(true),
            Some(true),
            BatteryState::FullyCharged,
        ),
        (
            Some(false),
            Some(false),
            Some(true),
            BatteryState::NotCharging,
        ),
        (None, None, None, BatteryState::Unknown),
        (Some(false), None, None, BatteryState::Unknown),
    ] {
        assert_eq!(BatteryState::from_flags(charging, full, ac), expected);
    }
}
#[test]
fn capacities_do_not_confuse_charge_and_health() {
    assert_eq!(charge_percent(Some(71), Some(100)), Some(71.0));
    assert_eq!(charge_percent(Some(0), Some(100)), Some(0.0));
    assert_eq!(charge_percent(Some(100), Some(0)), None);
    assert_eq!(charge_percent(Some(-1), Some(100)), None);
    assert_eq!(charge_percent(Some(101), Some(100)), None);
    assert_eq!(health_percent(Some(4400), Some(5000)), Some(88.0));
    assert_eq!(health_percent(Some(100), Some(5000)), None);
    assert_eq!(health_percent(None, Some(5000)), None);
    assert_eq!(health_percent(Some(5100), Some(5000)), Some(102.0));
}
#[test]
fn time_sentinels_and_temperature() {
    assert_eq!(minutes(Some(342)), Some(Duration::from_secs(20_520)));
    assert_eq!(minutes(Some(0)), Some(Duration::ZERO));
    for value in [None, Some(-1), Some(65535), Some(i64::MAX)] {
        assert_eq!(minutes(value), None);
    }
    assert!((smart_battery_temperature(Some(3074)).unwrap().0 - 34.25).abs() < 0.001);
    assert_eq!(smart_battery_temperature(None), None);
    assert_eq!(smart_battery_temperature(Some(0)), None);
    assert_eq!(smart_battery_temperature(Some(25_000)), None);
}
struct Fake {
    values: VecDeque<Result<PowerSnapshot, PowerError>>,
}
impl PowerRepository for Fake {
    fn power(&mut self) -> Result<PowerSnapshot, PowerError> {
        self.values.pop_front().expect("too many polls")
    }
}
#[test]
fn cache_throttles_errors_and_recovers() {
    let fake = Fake {
        values: [
            Ok(PowerSnapshot::NoBattery),
            Err(PowerError::PermissionDenied("IOKit".into())),
            Ok(PowerSnapshot::Battery(BatteryMetrics::default())),
        ]
        .into(),
    };
    let mut use_case = GetPowerMetrics::new(fake);
    let start = Instant::now();
    assert!(matches!(
        use_case.execute(start),
        Ok(PowerSnapshot::NoBattery)
    ));
    assert!(matches!(
        use_case.execute(start + Duration::from_millis(1999)),
        Ok(PowerSnapshot::NoBattery)
    ));
    assert!(matches!(
        use_case.execute(start + Duration::from_secs(2)),
        Err(PowerError::PermissionDenied(_))
    ));
    assert!(matches!(
        use_case.execute(start + Duration::from_secs(3)),
        Err(PowerError::PermissionDenied(_))
    ));
    assert!(matches!(
        use_case.execute(start + Duration::from_secs(4)),
        Ok(PowerSnapshot::Battery(_))
    ));
}
