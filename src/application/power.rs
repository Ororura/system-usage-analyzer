use crate::domain::power::{PowerError, PowerRepository, PowerSnapshot};
use std::time::{Duration, Instant};

pub const POWER_INTERVAL: Duration = Duration::from_secs(2);

pub struct GetPowerMetrics<R> {
    repository: R,
    last_sample: Option<Instant>,
    cached: Result<PowerSnapshot, PowerError>,
}

impl<R: PowerRepository> GetPowerMetrics<R> {
    pub fn new(repository: R) -> Self {
        Self {
            repository,
            last_sample: None,
            cached: Err(PowerError::DataUnavailable("Not sampled yet".into())),
        }
    }

    pub fn execute(&mut self, now: Instant) -> &Result<PowerSnapshot, PowerError> {
        if self
            .last_sample
            .is_none_or(|last| now.saturating_duration_since(last) >= POWER_INTERVAL)
        {
            self.cached = self.repository.power();
            self.last_sample = Some(now);
        }
        &self.cached
    }
}
