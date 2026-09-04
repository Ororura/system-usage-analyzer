#[cfg(target_os = "macos")]
pub use super::macos::power::MacOsPowerRepository as PlatformPowerRepository;

#[cfg(not(target_os = "macos"))]
pub struct PlatformPowerRepository;

#[cfg(not(target_os = "macos"))]
impl crate::domain::power::PowerRepository for PlatformPowerRepository {
    fn power(
        &mut self,
    ) -> Result<crate::domain::power::PowerSnapshot, crate::domain::power::PowerError> {
        Err(crate::domain::power::PowerError::Unsupported)
    }
}
