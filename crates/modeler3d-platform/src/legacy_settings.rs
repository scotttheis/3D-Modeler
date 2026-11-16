//! Read-only import of the Windows QSettings values written by the Python app.
//!
//! 3D Modeler's Rust settings file is the writable authority. This adapter only
//! reads the current (`Builder3D/Builder3D`) and historical
//! (`3DModeler/3DModeler`) QSettings keys so the shell can migrate supported
//! values once into its JSON file. It never creates, changes, or removes a
//! registry value.

use std::collections::{BTreeMap, BTreeSet};

/// Normalized values found in the two Python QSettings locations.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct LegacySettings {
    /// Values under `HKCU\\Software\\Builder3D\\Builder3D`.
    pub current: BTreeMap<String, Vec<String>>,
    /// Values under `HKCU\\Software\\3DModeler\\3DModeler`.
    pub legacy: BTreeMap<String, Vec<String>>,
    /// Names that existed in the current application key, including values
    /// whose registry representation was not supported. Presence is separate
    /// from decoding so an empty current recent list will not revive old paths.
    pub current_present: BTreeSet<String>,
    /// Names that existed in the old application key.
    pub legacy_present: BTreeSet<String>,
    /// Bounded, non-sensitive warnings for unsupported values or read errors.
    pub warnings: Vec<String>,
}

impl LegacySettings {
    /// Returns normalized values from the current Python settings location.
    #[must_use]
    pub fn current_values(&self, name: &str) -> Option<&[String]> {
        self.current.get(name).map(Vec::as_slice)
    }

    /// Returns normalized values from the historical Python settings location.
    #[must_use]
    pub fn legacy_values(&self, name: &str) -> Option<&[String]> {
        self.legacy.get(name).map(Vec::as_slice)
    }
}

/// Reads the supported Python settings without writing to the registry.
#[must_use]
pub fn read_legacy_settings() -> LegacySettings {
    #[cfg(windows)]
    {
        windows_registry::read()
    }
    #[cfg(not(windows))]
    {
        LegacySettings::default()
    }
}

/// Decodes the bounded registry formats emitted by QSettings on Windows.
///
/// `REG_SZ` and `REG_MULTI_SZ` are the usual scalar and QStringList encodings;
/// `REG_DWORD`/`REG_QWORD` cover scalar values if a QSettings version chose an
/// integer representation. Other encodings are reported to the caller.
fn decode_registry_value(registry_type: u32, bytes: &[u8]) -> Result<Vec<String>, String> {
    const REG_SZ: u32 = 1;
    const REG_DWORD: u32 = 4;
    const REG_MULTI_SZ: u32 = 7;
    const REG_QWORD: u32 = 11;

    match registry_type {
        REG_SZ | REG_MULTI_SZ => {
            let (chunks, remainder) = bytes.as_chunks::<2>();
            if !remainder.is_empty() {
                return Err(String::from("UTF-16 registry data has an odd byte count"));
            }
            let units = chunks
                .iter()
                .map(|chunk| u16::from_le_bytes(*chunk))
                .collect::<Vec<_>>();
            let mut values = Vec::new();
            let mut start = 0;
            for (index, unit) in units.iter().enumerate() {
                if *unit == 0 {
                    if index > start {
                        let value = String::from_utf16(&units[start..index])
                            .map_err(|_| String::from("registry value is invalid UTF-16"))?;
                        values.push(value);
                    }
                    start = index + 1;
                    if registry_type == REG_SZ {
                        break;
                    }
                }
            }
            if registry_type == REG_SZ && start == 0 {
                values.push(
                    String::from_utf16(&units)
                        .map_err(|_| String::from("registry value is invalid UTF-16"))?,
                );
            }
            Ok(values)
        }
        REG_DWORD if bytes.len() == 4 => Ok(vec![
            u32::from_le_bytes(bytes.try_into().expect("DWORD length checked")).to_string(),
        ]),
        REG_QWORD if bytes.len() == 8 => Ok(vec![
            u64::from_le_bytes(bytes.try_into().expect("QWORD length checked")).to_string(),
        ]),
        REG_DWORD => Err(String::from("REG_DWORD had an invalid byte count")),
        REG_QWORD => Err(String::from("REG_QWORD had an invalid byte count")),
        _ => Err(format!("unsupported registry type {registry_type}")),
    }
}

#[cfg(windows)]
#[allow(unsafe_code)]
mod windows_registry {
    use std::{
        collections::{BTreeMap, BTreeSet},
        ffi::OsStr,
        os::windows::ffi::OsStrExt,
        ptr,
    };

    use windows::{
        Win32::System::Registry::{
            HKEY, HKEY_CURRENT_USER, KEY_READ, KEY_WOW64_64KEY, REG_VALUE_TYPE, RegCloseKey,
            RegOpenKeyExW, RegQueryValueExW,
        },
        core::PCWSTR,
    };

    use super::{LegacySettings, decode_registry_value};

    const MAX_VALUE_BYTES: u32 = 64 * 1024;
    const VALUE_NAMES: &[&str] = &[
        "theme_mode",
        "default_import_unit",
        "preferred_slicer_path",
        "preferred_slicer_label",
        "object_snapping_enabled",
        "ribbon_favorites",
        "ribbon_pinned",
        "ribbon_edit_order",
        "ribbon_home_order",
        "ribbon_insert_order",
        "ribbon_object_order",
        "ribbon_view_order",
        "ribbon_tools_order",
        "recent_files",
    ];

    struct Key(HKEY);

    impl Drop for Key {
        fn drop(&mut self) {
            // This handle is created by RegOpenKeyExW and closed once here.
            let _ = unsafe { RegCloseKey(self.0) };
        }
    }

    pub(super) fn read() -> LegacySettings {
        let mut settings = LegacySettings::default();
        read_location(
            r"Software\Builder3D\Builder3D",
            &mut settings.current,
            &mut settings.current_present,
            &mut settings.warnings,
            "current",
        );
        read_location(
            r"Software\3DModeler\3DModeler",
            &mut settings.legacy,
            &mut settings.legacy_present,
            &mut settings.warnings,
            "legacy",
        );
        settings
    }

    fn wide(value: &str) -> Vec<u16> {
        OsStr::new(value).encode_wide().chain(Some(0)).collect()
    }

    fn read_location(
        path: &str,
        output: &mut BTreeMap<String, Vec<String>>,
        present: &mut BTreeSet<String>,
        warnings: &mut Vec<String>,
        label: &str,
    ) {
        let path = wide(path);
        let mut key = HKEY::default();
        let result = unsafe {
            RegOpenKeyExW(
                HKEY_CURRENT_USER,
                PCWSTR(path.as_ptr()),
                None,
                KEY_READ | KEY_WOW64_64KEY,
                &mut key,
            )
        };
        if result.0 != 0 {
            if result.0 != 2 {
                warnings.push(format!(
                    "Could not read {label} Python settings (Windows error {}).",
                    result.0
                ));
            }
            return;
        }
        let key = Key(key);
        for name in VALUE_NAMES {
            let wide_name = wide(name);
            let mut value_type = REG_VALUE_TYPE::default();
            let mut size = 0_u32;
            let query = unsafe {
                RegQueryValueExW(
                    key.0,
                    PCWSTR(wide_name.as_ptr()),
                    None,
                    Some(&mut value_type),
                    None,
                    Some(&mut size),
                )
            };
            if query.0 == 2 {
                continue;
            }
            if query.0 != 0 {
                warnings.push(format!(
                    "Could not read {label} Python setting {name} (Windows error {}).",
                    query.0
                ));
                continue;
            }
            present.insert((*name).to_owned());
            if size > MAX_VALUE_BYTES {
                warnings.push(format!("Ignored oversized {label} Python setting {name}."));
                continue;
            }
            let mut bytes = vec![0_u8; size as usize];
            let read_size = if size == 0 {
                ptr::null_mut()
            } else {
                bytes.as_mut_ptr()
            };
            let read = unsafe {
                RegQueryValueExW(
                    key.0,
                    PCWSTR(wide_name.as_ptr()),
                    None,
                    Some(&mut value_type),
                    if size == 0 { None } else { Some(read_size) },
                    Some(&mut size),
                )
            };
            if read.0 != 0 {
                warnings.push(format!(
                    "Could not read {label} Python setting {name} (Windows error {}).",
                    read.0
                ));
                continue;
            }
            bytes.truncate(size as usize);
            match decode_registry_value(value_type.0, &bytes) {
                Ok(values) => {
                    output.insert((*name).to_owned(), values);
                }
                Err(reason) => {
                    warnings.push(format!("Ignored {label} Python setting {name}: {reason}."))
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn utf16(value: &str) -> Vec<u8> {
        value.encode_utf16().flat_map(u16::to_le_bytes).collect()
    }

    #[test]
    fn decodes_qsettings_string_and_string_list_without_empty_elements() {
        assert_eq!(
            decode_registry_value(1, &utf16("light\0")),
            Ok(vec![String::from("light")])
        );
        assert_eq!(
            decode_registry_value(7, &utf16("a\0b\0\0")),
            Ok(vec![String::from("a"), String::from("b")])
        );
        assert_eq!(decode_registry_value(7, &utf16("\0\0")), Ok(Vec::new()));
    }

    #[test]
    fn decodes_integer_encodings_and_rejects_unsupported_registry_data() {
        assert_eq!(
            decode_registry_value(4, &17_u32.to_le_bytes()),
            Ok(vec![String::from("17")])
        );
        assert_eq!(
            decode_registry_value(11, &23_u64.to_le_bytes()),
            Ok(vec![String::from("23")])
        );
        assert!(decode_registry_value(3, &[0, 1, 2, 3]).is_err());
        assert!(decode_registry_value(4, &[1, 2]).is_err());
        assert!(decode_registry_value(1, &[0]).is_err());
    }

    #[test]
    fn legacy_presence_and_values_are_distinct_for_empty_current_recents() {
        let settings = LegacySettings {
            current: BTreeMap::from([(String::from("recent_files"), Vec::new())]),
            legacy: BTreeMap::from([(String::from("recent_files"), vec![String::from("old.stl")])]),
            current_present: BTreeSet::from([String::from("recent_files")]),
            legacy_present: BTreeSet::from([String::from("recent_files")]),
            warnings: Vec::new(),
        };
        assert_eq!(settings.current_values("recent_files"), Some([].as_slice()));
        assert_eq!(
            settings.legacy_values("recent_files"),
            Some(["old.stl".to_owned()].as_slice())
        );
        assert!(settings.current_present.contains("recent_files"));
    }
}
