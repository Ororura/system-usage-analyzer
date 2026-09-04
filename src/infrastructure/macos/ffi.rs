//! Minimal IOKit ownership boundary. No borrowed CF pointers escape this module.
use crate::domain::power::PowerError;
use core_foundation::{
    array::{CFArray, CFArrayRef},
    base::{CFType, CFTypeRef, TCFType},
    boolean::CFBoolean,
    dictionary::{CFDictionary, CFDictionaryRef},
    number::CFNumber,
    string::CFString,
};
use std::{
    ffi::{c_char, c_void},
    ptr,
};

#[link(name = "IOKit", kind = "framework")]
unsafe extern "C" {
    fn IOPSCopyPowerSourcesInfo() -> CFTypeRef;
    fn IOPSCopyPowerSourcesList(info: CFTypeRef) -> CFArrayRef;
    fn IOPSGetPowerSourceDescription(info: CFTypeRef, source: CFTypeRef) -> CFDictionaryRef;
    fn IOServiceMatching(name: *const c_char) -> CFDictionaryRef;
    fn IOServiceGetMatchingServices(
        port: u32,
        matching: CFDictionaryRef,
        iterator: *mut u32,
    ) -> i32;
    fn IOIteratorNext(iterator: u32) -> u32;
    fn IORegistryEntryCreateCFProperties(
        entry: u32,
        properties: *mut CFDictionaryRef,
        allocator: *const c_void,
        options: u32,
    ) -> i32;
    fn IOObjectRelease(object: u32) -> i32;
}

struct IoObject(u32);
impl Drop for IoObject {
    fn drop(&mut self) {
        // SAFETY: this wrapper exclusively owns a nonzero +1 IOKit reference.
        unsafe {
            IOObjectRelease(self.0);
        }
    }
}

pub(super) struct Dictionary(CFDictionary<CFString, CFType>);
impl Dictionary {
    #[cfg(test)]
    pub(super) fn from_entries(entries: &[(CFString, CFType)]) -> Self {
        Self(CFDictionary::from_CFType_pairs(entries))
    }

    pub fn number(&self, key: &str) -> Result<Option<i64>, PowerError> {
        self.0
            .find(CFString::new(key))
            .map(|v| {
                v.downcast::<CFNumber>()
                    .and_then(|n| n.to_i64())
                    .ok_or_else(|| PowerError::InvalidData(key.into()))
            })
            .transpose()
    }
    pub fn boolean(&self, key: &str) -> Result<Option<bool>, PowerError> {
        self.0
            .find(CFString::new(key))
            .map(|v| {
                v.downcast::<CFBoolean>()
                    .map(bool::from)
                    .ok_or_else(|| PowerError::InvalidData(key.into()))
            })
            .transpose()
    }
    pub fn string(&self, key: &str) -> Result<Option<String>, PowerError> {
        self.0
            .find(CFString::new(key))
            .map(|v| {
                v.downcast::<CFString>()
                    .map(|s| s.to_string())
                    .ok_or_else(|| PowerError::InvalidData(key.into()))
            })
            .transpose()
    }
}

pub(super) fn internal_battery() -> Result<Option<Dictionary>, PowerError> {
    // SAFETY: no arguments; Copy returns either null or an owned CF object.
    let raw = unsafe { IOPSCopyPowerSourcesInfo() };
    if raw.is_null() {
        return Err(PowerError::DataUnavailable("Power source info".into()));
    }
    // SAFETY: nonnull +1 result, released by CFType's Drop.
    let info = unsafe { CFType::wrap_under_create_rule(raw) };
    // SAFETY: info remains alive throughout the call and subsequent description reads.
    let raw_list = unsafe { IOPSCopyPowerSourcesList(info.as_CFTypeRef()) };
    if raw_list.is_null() {
        return Err(PowerError::SystemApi("Power source list".into()));
    }
    // SAFETY: API returns an owned array of CF power source handles.
    let sources = unsafe { CFArray::<CFType>::wrap_under_create_rule(raw_list) };
    for source in sources.iter() {
        // SAFETY: source belongs to sources and info is still retained.
        let raw_dict =
            unsafe { IOPSGetPowerSourceDescription(info.as_CFTypeRef(), source.as_CFTypeRef()) };
        if raw_dict.is_null() {
            return Err(PowerError::DataUnavailable(
                "Power source description".into(),
            ));
        }
        // SAFETY: Get returns a borrowed dictionary of CF string keys/CF values;
        // wrap_under_get_rule retains it before info can be dropped.
        let dict = Dictionary(unsafe { CFDictionary::wrap_under_get_rule(raw_dict) });
        if dict.string("Type")?.as_deref() == Some("InternalBattery") {
            return Ok(Some(dict));
        }
    }
    Ok(None)
}

pub(super) fn smart_battery() -> Result<Option<Dictionary>, PowerError> {
    // SAFETY: static NUL-terminated class name; matching returns a +1 dictionary.
    let matching = unsafe { IOServiceMatching(c"AppleSmartBattery".as_ptr()) };
    if matching.is_null() {
        return Err(PowerError::SystemApi("IOServiceMatching".into()));
    }
    let mut iterator = 0;
    // SAFETY: consumes matching on success and failure; iterator is writable.
    let code = unsafe { IOServiceGetMatchingServices(0, matching, &mut iterator) };
    check_return(code, "IOServiceGetMatchingServices")?;
    if iterator == 0 {
        return Err(PowerError::SystemApi("Missing iterator".into()));
    }
    let iterator = IoObject(iterator);
    // SAFETY: live iterator; returns a +1 service or zero when exhausted.
    let service = unsafe { IOIteratorNext(iterator.0) };
    if service == 0 {
        return Ok(None);
    }
    let service = IoObject(service);
    let mut properties = ptr::null();
    // SAFETY: live service, valid out pointer, default allocator, no options.
    let code =
        unsafe { IORegistryEntryCreateCFProperties(service.0, &mut properties, ptr::null(), 0) };
    check_return(code, "IORegistryEntryCreateCFProperties")?;
    if properties.is_null() {
        return Err(PowerError::DataUnavailable("Battery properties".into()));
    }
    // SAFETY: successful Create returns +1 dictionary containing CF objects.
    Ok(Some(Dictionary(unsafe {
        CFDictionary::wrap_under_create_rule(properties)
    })))
}

fn check_return(code: i32, operation: &str) -> Result<(), PowerError> {
    match code as u32 {
        0 => Ok(()),
        0xe00002c1 | 0xe00002e2 => Err(PowerError::PermissionDenied(operation.into())),
        _ => Err(PowerError::SystemApi(format!("{operation}: {code:#x}"))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn dictionary_checks_types_and_preserves_signed_numbers() {
        let dict = Dictionary::from_entries(&[
            (
                CFString::new("current"),
                CFNumber::from(-2131i64).as_CFType(),
            ),
            (CFString::new("flag"), CFBoolean::true_value().as_CFType()),
            (
                CFString::new("name"),
                CFString::new("InternalBattery").as_CFType(),
            ),
        ]);
        assert_eq!(dict.number("current").unwrap(), Some(-2131));
        assert_eq!(dict.boolean("flag").unwrap(), Some(true));
        assert_eq!(
            dict.string("name").unwrap().as_deref(),
            Some("InternalBattery")
        );
        assert_eq!(dict.number("absent").unwrap(), None);
        assert!(matches!(
            dict.number("name"),
            Err(PowerError::InvalidData(_))
        ));
        assert!(matches!(
            dict.boolean("current"),
            Err(PowerError::InvalidData(_))
        ));
    }
}
