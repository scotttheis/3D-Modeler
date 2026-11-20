//! Pure application preferences and recent-file policy.
//!
//! Persistence adapters can serialize these values later; this module keeps
//! ordering, deduplication, unit selection, and bounded lists independent of
//! Qt or a platform settings API.

use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use crate::ImportUnit;
use crate::grab_cursor::GrabCursorStyle;
use crate::theme::{ThemeMode, UiSize};
use modeler3d_platform::LegacySettings;
use modeler3d_viewport::ViewportLighting;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use thiserror::Error;

/// Maximum recent paths retained by the welcome workflow.
pub const MAX_RECENT_FILES: usize = 10;
/// Maximum favorite command identities retained by the ribbon preferences.
pub const MAX_RIBBON_FAVORITES: usize = 64;
/// Maximum size accepted for a JSON settings file before it is parsed.
pub const MAX_SETTINGS_FILE_BYTES: u64 = 1024 * 1024;

/// Whether transform values are displayed in millimetres or inches.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum DisplayUnit {
    /// Display world distances in millimetres.
    Millimetre,
    /// Display world distances in inches.
    Inch,
}

impl DisplayUnit {
    /// Returns the compact persisted/display spelling.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Millimetre => "mm",
            Self::Inch => "in",
        }
    }

    /// Converts millimetres into the selected display unit.
    #[must_use]
    pub const fn from_mm(self, value_mm: f32) -> f32 {
        match self {
            Self::Millimetre => value_mm,
            Self::Inch => value_mm / 25.4,
        }
    }

    /// Converts a display-unit value back into millimetres.
    #[must_use]
    pub const fn to_mm(self, value: f32) -> f32 {
        match self {
            Self::Millimetre => value,
            Self::Inch => value * 25.4,
        }
    }
}

/// A bounded, newest-first recent-file list.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct RecentFiles {
    paths: Vec<PathBuf>,
}

impl RecentFiles {
    /// Creates a recent list from persisted paths, removing duplicates.
    #[must_use]
    pub fn from_paths(paths: impl IntoIterator<Item = PathBuf>) -> Self {
        let mut recent = Self::default();
        for path in paths {
            recent.push_existing_order(path);
        }
        recent
    }

    /// Returns paths in newest-first order.
    #[must_use]
    pub fn paths(&self) -> &[PathBuf] {
        &self.paths
    }

    /// Records a path at the front and caps the list at ten entries.
    pub fn record(&mut self, path: impl Into<PathBuf>) {
        let path = path.into();
        if path.as_os_str().is_empty() {
            return;
        }
        self.paths.retain(|existing| existing != &path);
        self.paths.insert(0, path);
        self.paths.truncate(MAX_RECENT_FILES);
    }

    /// Removes one exact path from the list.
    pub fn remove(&mut self, path: &Path) -> bool {
        let before = self.paths.len();
        self.paths.retain(|existing| existing != path);
        before != self.paths.len()
    }

    /// Clears all recent paths.
    pub fn clear(&mut self) {
        self.paths.clear();
    }

    fn push_existing_order(&mut self, path: PathBuf) {
        if path.as_os_str().is_empty() || self.paths.iter().any(|item| item == &path) {
            return;
        }
        self.paths.push(path);
        self.paths.truncate(MAX_RECENT_FILES);
    }
}

/// Unit policy for an import path whose format has no authoritative units.
#[must_use]
pub fn normalize_import_unit(value: &str, fallback: ImportUnit) -> ImportUnit {
    match value.trim().to_ascii_lowercase().as_str() {
        "um" | "µm" | "μm" => ImportUnit::Micrometre,
        "mm" => ImportUnit::Millimetre,
        "cm" => ImportUnit::Centimetre,
        "m" => ImportUnit::Metre,
        "in" => ImportUnit::Inch,
        "ft" => ImportUnit::Foot,
        _ => fallback,
    }
}

/// Returns whether the file format owns its on-disk unit contract.
#[must_use]
pub fn import_uses_authoritative_units(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|extension| extension.to_str())
            .map(|extension| extension.to_ascii_lowercase())
            .as_deref(),
        Some("3mf" | "gltf" | "glb" | "3dp")
    )
}

/// Resolves an ambiguous import unit while preserving authoritative formats.
#[must_use]
pub fn import_unit_for_path(
    path: &Path,
    default: ImportUnit,
    override_unit: Option<ImportUnit>,
) -> Option<ImportUnit> {
    if import_uses_authoritative_units(path) {
        None
    } else {
        Some(override_unit.unwrap_or(default))
    }
}

/// Pure application preferences that can be persisted by any desktop adapter.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Preferences {
    #[serde(default)]
    last_import_directory: Option<PathBuf>,
    default_import_unit: ImportUnit,
    recent_files: RecentFiles,
    ribbon_favorites: Vec<String>,
    ribbon_pinned: bool,
    edit_ribbon_order: Vec<String>,
    display_unit: DisplayUnit,
    #[serde(default)]
    theme_mode: ThemeMode,
    /// Shell size: Normal or Big (twice the size), OF-20.
    #[serde(default)]
    ui_size: UiSize,
    /// User's durable mesh light, highlight, and floor contact shadow controls.
    #[serde(default)]
    viewport_lighting: ViewportLighting,
    /// Grab cursor look chosen in Settings: the robot claw or a hand.
    #[serde(default)]
    grab_cursor: GrabCursorStyle,
    #[serde(default)]
    ribbon_orders: BTreeMap<String, Vec<String>>,
    /// Persistent landmark/floor snapping preference used by Move gestures.
    #[serde(default = "default_object_snapping_enabled")]
    object_snapping_enabled: bool,
    /// Preferred slicer executable chosen by the user. The executable is
    /// revalidated before a print handoff is launched.
    #[serde(default)]
    preferred_slicer_path: Option<PathBuf>,
    /// Display label retained for paths that are not in the discovery catalog.
    #[serde(default)]
    preferred_slicer_label: Option<String>,
    /// Unknown keys survive a load/save cycle to avoid discarding settings
    /// written by a newer version or a compatible extension.
    #[serde(flatten)]
    extra_settings: BTreeMap<String, Value>,
}

const fn default_object_snapping_enabled() -> bool {
    true
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            last_import_directory: None,
            default_import_unit: ImportUnit::Millimetre,
            recent_files: RecentFiles::default(),
            ribbon_favorites: Vec::new(),
            ribbon_pinned: false,
            edit_ribbon_order: Vec::new(),
            display_unit: DisplayUnit::Millimetre,
            theme_mode: ThemeMode::Dark,
            ui_size: UiSize::Normal,
            viewport_lighting: ViewportLighting::default(),
            grab_cursor: GrabCursorStyle::Claw,
            ribbon_orders: BTreeMap::new(),
            object_snapping_enabled: true,
            preferred_slicer_path: None,
            preferred_slicer_label: None,
            extra_settings: BTreeMap::new(),
        }
    }
}

/// A fieldwise settings load result with visible recovery/migration notes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PreferenceLoadReport {
    /// Recovered preferences after bounds and aliases have been normalized.
    pub preferences: Preferences,
    /// Non-fatal values that were ignored or migrated from the Python app.
    pub warnings: Vec<String>,
    /// True when at least one value came from the read-only legacy settings.
    pub migrated_legacy: bool,
    /// True when an existing JSON file was parsed successfully.
    pub json_file_loaded: bool,
    /// True when the settings (or the legacy settings) already held the
    /// person's shortcut bar, even an empty one. Only a first start, with no
    /// saved bar at all, gets the default shortcuts.
    pub ribbon_favorites_saved: bool,
}

impl Preferences {
    /// Last folder used to choose an import, retained across application restarts.
    pub fn last_import_directory(&self) -> Option<&Path> {
        self.last_import_directory.as_deref()
    }

    /// Remembers the selected file's parent even when the file cannot be imported.
    pub fn remember_import_path(&mut self, path: &Path) {
        self.last_import_directory = path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .map(Path::to_path_buf);
    }

    /// Loads preferences from a JSON settings file and normalizes its bounds.
    pub fn load_from_path(path: &Path) -> Result<Self, PreferencesError> {
        Self::load_with_legacy(path, &LegacySettings::default()).map(|report| report.preferences)
    }

    /// Loads settings field by field, then fills missing/unsupported values
    /// from the read-only Python QSettings adapter.
    pub fn load_with_legacy(
        path: &Path,
        legacy: &LegacySettings,
    ) -> Result<PreferenceLoadReport, PreferencesError> {
        validate_settings_path(path)?;
        let json_file_loaded = path.is_file();
        let root = if json_file_loaded {
            let metadata = fs::metadata(path)?;
            if metadata.len() > MAX_SETTINGS_FILE_BYTES {
                return Err(PreferencesError::Decode(format!(
                    "settings file exceeds the {MAX_SETTINGS_FILE_BYTES}-byte limit"
                )));
            }
            let bytes = fs::read(path)?;
            serde_json::from_slice::<Value>(&bytes)
                .map_err(|error| PreferencesError::Decode(error.to_string()))?
        } else {
            Value::Object(Map::new())
        };
        let mut object = root.as_object().cloned().ok_or_else(|| {
            PreferencesError::Decode(String::from("settings root must be an object"))
        })?;
        let mut preferences = Self::default();
        let mut warnings = legacy.warnings.clone();
        let mut migrated_legacy = false;

        macro_rules! json_field {
            ($name:literal, $parse:expr, $assign:expr) => {
                if let Some(value) = object.remove($name) {
                    match ($parse)(&value) {
                        Some(parsed) => $assign(&mut preferences, parsed),
                        None => {
                            push_warning(
                                &mut warnings,
                                format!("Ignored unsupported setting '{}'.", $name),
                            );
                            if let Some(parsed) = legacy_scalar(legacy, $name, &mut warnings) {
                                $assign(&mut preferences, parsed);
                                migrated_legacy = true;
                            }
                        }
                    }
                } else if let Some(parsed) = legacy_scalar(legacy, $name, &mut warnings) {
                    $assign(&mut preferences, parsed);
                    migrated_legacy = true;
                }
            };
        }

        json_field!(
            "default_import_unit",
            parse_import_unit,
            |p: &mut Self, v| p.default_import_unit = v
        );
        json_field!("theme_mode", parse_theme_mode, |p: &mut Self, v| p
            .theme_mode =
            v);
        if let Some(value) = object.remove("ui_size") {
            match value.as_str().and_then(UiSize::from_key) {
                Some(parsed) => preferences.ui_size = parsed,
                None => push_warning(
                    &mut warnings,
                    "Ignored unsupported setting 'ui_size'.".to_owned(),
                ),
            }
        }
        if let Some(value) = object.remove("viewport_lighting") {
            match parse_viewport_lighting(&value) {
                Some(parsed) => preferences.viewport_lighting = parsed,
                None => push_warning(
                    &mut warnings,
                    "Ignored unsupported setting 'viewport_lighting'.".to_owned(),
                ),
            }
        }
        if let Some(value) = object.remove("grab_cursor") {
            match value.as_str().and_then(GrabCursorStyle::from_key) {
                Some(parsed) => preferences.grab_cursor = parsed,
                None => push_warning(
                    &mut warnings,
                    "Ignored unsupported setting 'grab_cursor'.".to_owned(),
                ),
            }
        }
        json_field!("display_unit", parse_display_unit, |p: &mut Self, v| p
            .display_unit =
            v);
        json_field!("ribbon_pinned", parse_bool, |p: &mut Self, v| p
            .ribbon_pinned =
            v);
        json_field!("object_snapping_enabled", parse_bool, |p: &mut Self, v| p
            .object_snapping_enabled =
            v);

        if let Some(value) = object.remove("recent_files") {
            match parse_recent_files(&value) {
                Some(parsed) => preferences.recent_files = parsed,
                None => {
                    push_warning(
                        &mut warnings,
                        "Ignored unsupported setting 'recent_files'.".to_owned(),
                    );
                    if let Some(paths) = legacy_recent_files(legacy, &mut warnings) {
                        preferences.recent_files = RecentFiles::from_paths(paths);
                        migrated_legacy = true;
                    }
                }
            }
        } else if let Some(paths) = legacy_recent_files(legacy, &mut warnings) {
            preferences.recent_files = RecentFiles::from_paths(paths);
            migrated_legacy = true;
        }

        if let Some(value) = object.remove("last_import_directory") {
            preferences.last_import_directory =
                value.as_str().filter(|s| !s.is_empty()).map(PathBuf::from);
        }
        let mut ribbon_favorites_saved = false;
        if let Some(value) = object.remove("ribbon_favorites") {
            match parse_string_list(&value) {
                Some(items) => {
                    preferences.set_ribbon_favorites(items);
                    ribbon_favorites_saved = true;
                }
                None => {
                    push_warning(
                        &mut warnings,
                        "Ignored unsupported setting 'ribbon_favorites'.".to_owned(),
                    );
                    if let Some(items) =
                        legacy_string_list(legacy, "ribbon_favorites", &mut warnings)
                    {
                        preferences.set_ribbon_favorites(items);
                        ribbon_favorites_saved = true;
                        migrated_legacy = true;
                    }
                }
            }
        } else if let Some(items) = legacy_string_list(legacy, "ribbon_favorites", &mut warnings) {
            preferences.set_ribbon_favorites(items);
            ribbon_favorites_saved = true;
            migrated_legacy = true;
        }

        let edit_order = object
            .remove("edit_ribbon_order")
            .or_else(|| object.remove("ribbon_edit_order"));
        if let Some(value) = edit_order {
            match parse_string_list(&value) {
                Some(items) => preferences.set_edit_ribbon_order(items),
                None => push_warning(
                    &mut warnings,
                    "Ignored unsupported setting 'ribbon_edit_order'.".to_owned(),
                ),
            }
        } else if let Some(items) = legacy_string_list(legacy, "ribbon_edit_order", &mut warnings) {
            preferences.set_edit_ribbon_order(items);
            migrated_legacy = true;
        }

        if let Some(value) = object.remove("ribbon_orders") {
            if let Some(orders) = value.as_object() {
                for (tab, order) in orders {
                    if let Some(items) = parse_string_list(order) {
                        preferences.set_ribbon_order(normalize_tab_label(tab), items);
                    } else {
                        push_warning(
                            &mut warnings,
                            format!("Ignored unsupported ribbon order for '{tab}'."),
                        );
                    }
                }
            } else {
                push_warning(
                    &mut warnings,
                    "Ignored unsupported setting 'ribbon_orders'.".to_owned(),
                );
            }
        }
        for tab in ["insert", "object", "edit", "paint", "view"] {
            let key = format!("ribbon_{tab}_order");
            if let Some(value) = object.remove(&key) {
                if let Some(items) = parse_string_list(&value) {
                    preferences.set_ribbon_order(tab, items);
                } else {
                    push_warning(
                        &mut warnings,
                        format!("Ignored unsupported setting '{key}'."),
                    );
                }
            } else if let Some(items) = legacy_string_list(legacy, &key, &mut warnings) {
                preferences.set_ribbon_order(normalize_tab_label(tab), items);
                migrated_legacy = true;
            }
        }

        json_field!(
            "preferred_slicer_path",
            parse_optional_path,
            |p: &mut Self, v| p.preferred_slicer_path = v
        );
        json_field!(
            "preferred_slicer_label",
            parse_optional_string,
            |p: &mut Self, v| p.preferred_slicer_label = v
        );

        // All recognized Python keys have now been interpreted. Preserve every
        // other JSON value across a subsequent save.
        preferences.extra_settings = object
            .into_iter()
            .filter(|(key, _)| !is_legacy_alias(key))
            .collect();
        if migrated_legacy {
            push_warning(&mut warnings, "Imported supported Python settings; the Rust JSON settings file will be the writable copy.".to_owned());
        }
        preferences.normalize_loaded();
        Ok(PreferenceLoadReport {
            preferences,
            warnings,
            migrated_legacy,
            json_file_loaded,
            ribbon_favorites_saved,
        })
    }

    /// Saves preferences as a bounded JSON settings file.
    pub fn save_to_path(&self, path: &Path) -> Result<(), PreferencesError> {
        validate_settings_path(path)?;
        let bytes = serde_json::to_vec_pretty(self)
            .map_err(|error| PreferencesError::Encode(error.to_string()))?;
        if let Some(parent) = path.parent()
            && !parent.as_os_str().is_empty()
        {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, bytes)?;
        Ok(())
    }

    /// Returns the persisted default for ambiguous imports.
    #[must_use]
    pub const fn default_import_unit(&self) -> ImportUnit {
        self.default_import_unit
    }

    /// Sets the default ambiguous-import unit.
    pub const fn set_default_import_unit(&mut self, unit: ImportUnit) {
        self.default_import_unit = unit;
    }

    /// Returns the bounded recent-file list.
    #[must_use]
    pub const fn recent_files(&self) -> &RecentFiles {
        &self.recent_files
    }

    /// Returns mutable access for the persistence adapter.
    pub const fn recent_files_mut(&mut self) -> &mut RecentFiles {
        &mut self.recent_files
    }

    /// Returns the validated favorite command identities.
    #[must_use]
    pub fn ribbon_favorites(&self) -> &[String] {
        &self.ribbon_favorites
    }

    /// Replaces favorites while removing duplicates and empty identities.
    pub fn set_ribbon_favorites(&mut self, values: impl IntoIterator<Item = String>) {
        self.ribbon_favorites.clear();
        for value in values {
            let value = value.trim().to_owned();
            if value.is_empty()
                || self
                    .ribbon_favorites
                    .iter()
                    .any(|existing| existing == &value)
            {
                continue;
            }
            self.ribbon_favorites.push(value);
            if self.ribbon_favorites.len() == MAX_RIBBON_FAVORITES {
                break;
            }
        }
    }

    /// Returns whether the ribbon is pinned open.
    #[must_use]
    pub const fn ribbon_pinned(&self) -> bool {
        self.ribbon_pinned
    }

    /// Sets the ribbon pinned preference.
    pub const fn set_ribbon_pinned(&mut self, pinned: bool) {
        self.ribbon_pinned = pinned;
    }

    /// Returns the persisted edit-ribbon order.
    #[must_use]
    pub fn edit_ribbon_order(&self) -> &[String] {
        &self.edit_ribbon_order
    }

    /// Stores a bounded, duplicate-free edit-ribbon order.
    pub fn set_edit_ribbon_order(&mut self, values: impl IntoIterator<Item = String>) {
        self.edit_ribbon_order.clear();
        for value in values {
            let value = value.trim().to_owned();
            if !value.is_empty() && !self.edit_ribbon_order.iter().any(|item| item == &value) {
                self.edit_ribbon_order.push(value);
            }
        }
    }

    /// Returns the transform display unit.
    #[must_use]
    pub const fn display_unit(&self) -> DisplayUnit {
        self.display_unit
    }

    /// Sets the transform display unit.
    pub const fn set_display_unit(&mut self, unit: DisplayUnit) {
        self.display_unit = unit;
    }

    /// Returns the persisted shell theme.
    #[must_use]
    pub const fn theme_mode(&self) -> ThemeMode {
        self.theme_mode
    }

    /// Sets the shell theme preference.
    pub const fn set_theme_mode(&mut self, mode: ThemeMode) {
        self.theme_mode = mode;
    }

    /// Returns the persisted shell size.
    #[must_use]
    pub const fn ui_size(&self) -> UiSize {
        self.ui_size
    }

    /// Sets the shell size preference.
    pub const fn set_ui_size(&mut self, size: UiSize) {
        self.ui_size = size;
    }

    /// Returns the persisted viewport lighting controls.
    #[must_use]
    pub const fn viewport_lighting(&self) -> ViewportLighting {
        self.viewport_lighting
    }

    /// Sets viewport lighting after enforcing the renderer's supported ranges.
    pub fn set_viewport_lighting(&mut self, lighting: ViewportLighting) {
        self.viewport_lighting = lighting.normalized();
    }

    /// Returns the grab cursor look chosen in Settings.
    #[must_use]
    pub const fn grab_cursor(&self) -> GrabCursorStyle {
        self.grab_cursor
    }

    /// Sets the grab cursor look.
    pub const fn set_grab_cursor(&mut self, style: GrabCursorStyle) {
        self.grab_cursor = style;
    }

    /// Returns whether landmark and build-floor snapping is enabled.
    #[must_use]
    pub const fn object_snapping_enabled(&self) -> bool {
        self.object_snapping_enabled
    }

    /// Sets the persistent landmark/floor snapping preference.
    pub const fn set_object_snapping_enabled(&mut self, enabled: bool) {
        self.object_snapping_enabled = enabled;
    }

    /// Returns the preferred slicer executable path, if one was saved.
    #[must_use]
    pub fn preferred_slicer_path(&self) -> Option<&Path> {
        self.preferred_slicer_path.as_deref()
    }

    /// Returns the preferred slicer label, if one was saved.
    #[must_use]
    pub fn preferred_slicer_label(&self) -> Option<&str> {
        self.preferred_slicer_label.as_deref()
    }

    /// Saves a slicer preference without granting it automatic launch rights.
    pub fn set_preferred_slicer(&mut self, path: Option<PathBuf>, label: Option<String>) {
        let path = path.filter(|value| !value.as_os_str().is_empty());
        let label = label
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty());
        self.preferred_slicer_path = path;
        self.preferred_slicer_label = label;
    }

    /// Returns a persisted command order for a ribbon tab, if present.
    #[must_use]
    pub fn ribbon_order(&self, tab: &str) -> Option<&[String]> {
        self.ribbon_orders.get(tab).map(Vec::as_slice)
    }

    /// Stores a bounded, duplicate-free command order for a ribbon tab.
    pub fn set_ribbon_order(
        &mut self,
        tab: impl Into<String>,
        values: impl IntoIterator<Item = String>,
    ) {
        let mut order = Vec::new();
        for value in values {
            let value = value.trim().to_owned();
            if !value.is_empty() && !order.iter().any(|item| item == &value) {
                order.push(value);
            }
            if order.len() >= MAX_RIBBON_FAVORITES {
                break;
            }
        }
        self.ribbon_orders.insert(tab.into(), order);
    }

    fn normalize_loaded(&mut self) {
        let recent_paths = std::mem::take(&mut self.recent_files.paths);
        self.recent_files = RecentFiles::from_paths(recent_paths);
        let favorites = std::mem::take(&mut self.ribbon_favorites);
        self.set_ribbon_favorites(favorites);
        let edit_order = std::mem::take(&mut self.edit_ribbon_order);
        self.set_edit_ribbon_order(edit_order);
        let ribbon_orders = std::mem::take(&mut self.ribbon_orders);
        self.ribbon_orders.clear();
        for (tab, order) in ribbon_orders {
            self.set_ribbon_order(tab, order);
        }
        self.viewport_lighting = self.viewport_lighting.normalized();
    }
}

fn parse_bool(value: &Value) -> Option<bool> {
    if let Some(value) = value.as_bool() {
        return Some(value);
    }
    match value.as_str()?.trim().to_ascii_lowercase().as_str() {
        "true" | "1" | "yes" | "on" => Some(true),
        "false" | "0" | "no" | "off" => Some(false),
        _ => None,
    }
}

fn parse_import_unit(value: &Value) -> Option<ImportUnit> {
    let spelling = value.as_str()?.trim().to_ascii_lowercase();
    match spelling.as_str() {
        "um" | "µm" | "μm" | "micrometre" | "micrometer" => Some(ImportUnit::Micrometre),
        "mm" | "millimetre" | "millimeter" => Some(ImportUnit::Millimetre),
        "cm" | "centimetre" | "centimeter" => Some(ImportUnit::Centimetre),
        "m" | "metre" | "meter" => Some(ImportUnit::Metre),
        "in" | "inch" | "inches" => Some(ImportUnit::Inch),
        "ft" | "foot" | "feet" => Some(ImportUnit::Foot),
        _ => None,
    }
}

fn parse_theme_mode(value: &Value) -> Option<ThemeMode> {
    let spelling = value.as_str()?.trim().to_ascii_lowercase();
    match spelling.as_str() {
        "dark" | "dark mode" | "true" | "1" | "on" => Some(ThemeMode::Dark),
        "light" | "light mode" | "false" | "0" | "off" => Some(ThemeMode::Light),
        _ => None,
    }
}

fn parse_viewport_lighting(value: &Value) -> Option<ViewportLighting> {
    serde_json::from_value(value.clone())
        .ok()
        .map(ViewportLighting::normalized)
}

fn parse_display_unit(value: &Value) -> Option<DisplayUnit> {
    match value.as_str()?.trim().to_ascii_lowercase().as_str() {
        "mm" | "millimetre" | "millimeter" => Some(DisplayUnit::Millimetre),
        "in" | "inch" | "inches" => Some(DisplayUnit::Inch),
        _ => None,
    }
}

fn parse_string_list(value: &Value) -> Option<Vec<String>> {
    value
        .as_array()?
        .iter()
        .map(|item| item.as_str().map(str::to_owned))
        .collect()
}

fn parse_recent_files(value: &Value) -> Option<RecentFiles> {
    let paths = value
        .as_array()
        .or_else(|| value.get("paths")?.as_array())?;
    let paths = paths
        .iter()
        .map(|item| item.as_str().map(PathBuf::from))
        .collect::<Option<Vec<_>>>()?;
    Some(RecentFiles::from_paths(paths))
}

fn parse_optional_path(value: &Value) -> Option<Option<PathBuf>> {
    match value {
        Value::Null => Some(None),
        Value::String(path) if !path.trim().is_empty() => Some(Some(PathBuf::from(path))),
        Value::String(_) => Some(None),
        _ => None,
    }
}

fn parse_optional_string(value: &Value) -> Option<Option<String>> {
    match value {
        Value::Null => Some(None),
        Value::String(text) => Some((!text.trim().is_empty()).then(|| text.trim().to_owned())),
        _ => None,
    }
}

fn legacy_scalar<T>(legacy: &LegacySettings, name: &str, warnings: &mut Vec<String>) -> Option<T>
where
    T: LegacyScalar,
{
    let (value, present) = legacy_value(legacy, name);
    let value = value?;
    match T::parse(value) {
        Some(parsed) => Some(parsed),
        None => {
            if present {
                push_warning(
                    warnings,
                    format!("Ignored unsupported Python setting '{name}'."),
                );
            }
            None
        }
    }
}

trait LegacyScalar: Sized {
    fn parse(value: &str) -> Option<Self>;
}

impl LegacyScalar for ImportUnit {
    fn parse(value: &str) -> Option<Self> {
        parse_import_unit(&Value::String(value.to_owned()))
    }
}

impl LegacyScalar for ThemeMode {
    fn parse(value: &str) -> Option<Self> {
        parse_theme_mode(&Value::String(value.to_owned()))
    }
}

impl LegacyScalar for DisplayUnit {
    fn parse(value: &str) -> Option<Self> {
        parse_display_unit(&Value::String(value.to_owned()))
    }
}

impl LegacyScalar for bool {
    fn parse(value: &str) -> Option<Self> {
        parse_bool(&Value::String(value.to_owned()))
    }
}

impl LegacyScalar for Option<PathBuf> {
    fn parse(value: &str) -> Option<Self> {
        Some((!value.trim().is_empty()).then(|| PathBuf::from(value)))
    }
}

impl LegacyScalar for Option<String> {
    fn parse(value: &str) -> Option<Self> {
        Some((!value.trim().is_empty()).then(|| value.trim().to_owned()))
    }
}

fn legacy_value<'a>(legacy: &'a LegacySettings, name: &str) -> (Option<&'a str>, bool) {
    let (values, present) = if legacy.current_present.contains(name) {
        (legacy.current_values(name), true)
    } else if name == "recent_files" && legacy.legacy_present.contains(name) {
        (legacy.legacy_values(name), true)
    } else {
        (None, false)
    };
    (
        values.and_then(|items| items.first()).map(String::as_str),
        present,
    )
}

fn legacy_string_list(
    legacy: &LegacySettings,
    name: &str,
    warnings: &mut Vec<String>,
) -> Option<Vec<String>> {
    let (values, present) = if legacy.current_present.contains(name) {
        (legacy.current_values(name), true)
    } else {
        (
            legacy.legacy_values(name),
            legacy.legacy_present.contains(name),
        )
    };
    if !present {
        return None;
    }
    values.map(<[String]>::to_vec).or_else(|| {
        push_warning(
            warnings,
            format!("Ignored unsupported Python setting '{name}'."),
        );
        None
    })
}

fn legacy_recent_files(
    legacy: &LegacySettings,
    warnings: &mut Vec<String>,
) -> Option<Vec<PathBuf>> {
    let (values, present) = if legacy.current_present.contains("recent_files") {
        (legacy.current_values("recent_files"), true)
    } else {
        (
            legacy.legacy_values("recent_files"),
            legacy.legacy_present.contains("recent_files"),
        )
    };
    if !present {
        return None;
    }
    values
        .map(|items| items.iter().map(PathBuf::from).collect())
        .or_else(|| {
            push_warning(
                warnings,
                "Ignored unsupported Python setting 'recent_files'.".to_owned(),
            );
            None
        })
}

fn push_warning(warnings: &mut Vec<String>, value: String) {
    if warnings.len() < 64 && !warnings.contains(&value) {
        warnings.push(value);
    }
}

fn is_legacy_alias(key: &str) -> bool {
    matches!(key, "ribbon_edit_order")
        || ["insert", "object", "edit", "paint", "view"]
            .iter()
            .any(|tab| key == format!("ribbon_{tab}_order"))
}

fn normalize_tab_label(tab: &str) -> String {
    ["Insert", "Object", "Edit", "Paint", "View"]
        .into_iter()
        .find(|candidate| candidate.eq_ignore_ascii_case(tab))
        .unwrap_or(tab)
        .to_owned()
}

/// Errors from the application settings file boundary.
#[derive(Debug, Error)]
pub enum PreferencesError {
    /// The settings path is empty or exceeds the desktop bound.
    #[error("invalid preferences path: {0}")]
    InvalidPath(String),
    /// The settings file could not be read or written.
    #[error("preferences I/O failed: {0}")]
    Io(#[from] std::io::Error),
    /// The settings file was not valid JSON for the preferences schema.
    #[error("preferences decode failed: {0}")]
    Decode(String),
    /// The preferences value could not be encoded.
    #[error("preferences encode failed: {0}")]
    Encode(String),
}

fn validate_settings_path(path: &Path) -> Result<(), PreferencesError> {
    let display = path.to_string_lossy();
    if display.trim().is_empty() || display.len() > 4096 {
        return Err(PreferencesError::InvalidPath(display.into_owned()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    fn legacy_with_current(
        values: impl IntoIterator<Item = (String, Vec<String>)>,
    ) -> LegacySettings {
        let current = values.into_iter().collect::<BTreeMap<_, _>>();
        LegacySettings {
            current_present: current.keys().cloned().collect(),
            current,
            ..LegacySettings::default()
        }
    }

    #[test]
    fn import_unit_policy_preserves_authoritative_formats() {
        assert_eq!(
            normalize_import_unit(" um ", ImportUnit::Millimetre),
            ImportUnit::Micrometre
        );
        assert_eq!(
            normalize_import_unit("µm", ImportUnit::Millimetre),
            ImportUnit::Micrometre
        );
        assert_eq!(
            normalize_import_unit("μm", ImportUnit::Millimetre),
            ImportUnit::Micrometre
        );
        assert_eq!(
            normalize_import_unit(" CM ", ImportUnit::Millimetre),
            ImportUnit::Centimetre
        );
        assert_eq!(
            normalize_import_unit("unknown", ImportUnit::Inch),
            ImportUnit::Inch
        );
        assert_eq!(
            import_unit_for_path(
                Path::new("model.obj"),
                ImportUnit::Millimetre,
                Some(ImportUnit::Centimetre)
            ),
            Some(ImportUnit::Centimetre)
        );
        assert_eq!(
            import_unit_for_path(Path::new("model.GLB"), ImportUnit::Inch, None),
            None
        );
    }

    #[test]
    fn recent_files_are_newest_first_deduplicated_and_bounded() {
        let paths = (0..12)
            .map(|index| PathBuf::from(format!("model-{index}.stl")))
            .collect::<Vec<_>>();
        let mut recent = RecentFiles::from_paths(paths.clone());
        assert_eq!(recent.paths().len(), MAX_RECENT_FILES);
        recent.record(PathBuf::from("model-4.stl"));
        assert_eq!(recent.paths().first(), Some(&PathBuf::from("model-4.stl")));
        assert_eq!(recent.paths().len(), MAX_RECENT_FILES);
        assert!(recent.remove(Path::new("model-4.stl")));
        assert!(!recent.remove(Path::new("missing.stl")));
        recent.clear();
        assert!(recent.paths().is_empty());
    }

    #[test]
    fn ribbon_preferences_remove_duplicates_and_keep_display_units_separate() {
        let mut preferences = Preferences::default();
        preferences.set_ribbon_favorites([
            String::from(" Object:Group "),
            String::from("Object:Group"),
            String::from("Object:Cut"),
        ]);
        assert_eq!(
            preferences.ribbon_favorites(),
            &[String::from("Object:Group"), String::from("Object:Cut")]
        );
        preferences.set_display_unit(DisplayUnit::Inch);
        assert_eq!(preferences.display_unit().from_mm(25.4), 1.0);
        assert_eq!(preferences.display_unit().to_mm(2.0), 50.8);
    }

    #[test]
    fn preferences_round_trip_and_normalize_persisted_bounds() {
        let path = std::env::temp_dir().join(format!(
            "modeler3d-r7-preferences-{}.json",
            std::process::id()
        ));
        let mut preferences = Preferences::default();
        preferences.set_default_import_unit(ImportUnit::Micrometre);
        preferences.remember_import_path(&std::env::temp_dir().join("model.stl"));
        for index in 0..12 {
            preferences
                .recent_files_mut()
                .record(format!("model-{index}.stl"));
        }
        preferences
            .recent_files_mut()
            .record(PathBuf::from("model-5.stl"));
        assert_eq!(preferences.recent_files().paths().len(), MAX_RECENT_FILES);
        assert_eq!(
            preferences.recent_files().paths().first(),
            Some(&PathBuf::from("model-5.stl"))
        );
        preferences.set_ribbon_favorites([String::from("Object:Group")]);
        preferences.set_display_unit(DisplayUnit::Inch);
        preferences.set_grab_cursor(GrabCursorStyle::Hand);
        assert!(preferences.save_to_path(&path).is_ok());
        let loaded = Preferences::load_from_path(&path).expect("preferences");
        assert_eq!(loaded, preferences);
        assert_eq!(loaded.grab_cursor(), GrabCursorStyle::Hand);
        assert_eq!(loaded.recent_files().paths().len(), MAX_RECENT_FILES);
        assert_eq!(
            loaded.recent_files().paths().first(),
            Some(&PathBuf::from("model-5.stl"))
        );
        let _ = fs::remove_file(path);
    }

    #[test]
    fn malformed_preferences_are_typed_errors() {
        let path = std::env::temp_dir().join(format!(
            "modeler3d-r7-preferences-invalid-{}.json",
            std::process::id()
        ));
        assert!(fs::write(&path, b"not-json").is_ok());
        assert!(matches!(
            Preferences::load_from_path(&path),
            Err(PreferencesError::Decode(_))
        ));
        let _ = fs::remove_file(path);
    }

    #[test]
    fn persisted_micrometre_spellings_deserialize_without_changing_the_schema() {
        let baseline = serde_json::to_value(Preferences::default()).expect("preferences JSON");
        for spelling in ["um", "µm", "μm"] {
            let mut value = baseline.clone();
            value["default_import_unit"] = serde_json::Value::String(spelling.to_owned());
            let loaded: Preferences =
                serde_json::from_value(value).expect("micrometre alias should deserialize");
            assert_eq!(loaded.default_import_unit(), ImportUnit::Micrometre);
        }
    }

    #[test]
    fn partial_settings_recover_fields_independently_and_preserve_unknown_keys() {
        let path = std::env::temp_dir().join(format!(
            "modeler3d-r8-partial-preferences-{}.json",
            std::process::id()
        ));
        let bytes = br#"{
          "default_import_unit":"in",
          "theme_mode":"Light Mode",
          "display_unit":"unknown-unit",
          "recent_files":["first.stl","first.stl","second.obj"],
          "ribbon_pinned":true,
          "object_snapping_enabled":true,
          "future_extension":{"enabled":true,"mode":"preserve"}
        }"#;
        fs::write(&path, bytes).expect("partial settings file");
        let legacy = legacy_with_current([
            (
                String::from("default_import_unit"),
                vec![String::from("cm")],
            ),
            (String::from("theme_mode"), vec![String::from("dark")]),
            (String::from("display_unit"), vec![String::from("in")]),
            (
                String::from("object_snapping_enabled"),
                vec![String::from("false")],
            ),
        ]);

        let report = Preferences::load_with_legacy(&path, &legacy).expect("partial recovery");

        assert_eq!(report.preferences.default_import_unit(), ImportUnit::Inch);
        assert_eq!(report.preferences.theme_mode(), ThemeMode::Light);
        assert_eq!(report.preferences.display_unit(), DisplayUnit::Inch);
        assert!(report.preferences.ribbon_pinned());
        assert!(report.preferences.object_snapping_enabled());
        assert_eq!(
            report.preferences.recent_files().paths(),
            &[PathBuf::from("first.stl"), PathBuf::from("second.obj")]
        );
        assert_eq!(
            report
                .preferences
                .extra_settings
                .get("future_extension")
                .and_then(Value::as_object)
                .and_then(|object| object.get("enabled"))
                .and_then(Value::as_bool),
            Some(true)
        );
        assert!(
            report
                .warnings
                .iter()
                .any(|warning| warning.contains("display_unit"))
        );
        assert!(report.migrated_legacy);
        report
            .preferences
            .save_to_path(&path)
            .expect("preserving save");
        let saved: Value = serde_json::from_slice(&fs::read(&path).expect("saved JSON"))
            .expect("valid saved JSON");
        assert_eq!(saved["future_extension"]["mode"], "preserve");
        let _ = fs::remove_file(path);
    }

    #[test]
    fn python_aliases_and_flat_ribbon_orders_migrate_to_rust_fields() {
        let path = std::env::temp_dir().join(format!(
            "modeler3d-r8-python-aliases-{}.json",
            std::process::id()
        ));
        fs::write(
            &path,
            br#"{"ribbon_edit_order":["Insert:Cube"],"ribbon_orders":{}}"#,
        )
        .expect("settings file");
        let legacy = legacy_with_current([
            (
                String::from("default_import_unit"),
                vec![String::from("um")],
            ),
            (String::from("theme_mode"), vec![String::from("light")]),
            (
                String::from("ribbon_favorites"),
                vec![String::from("Insert:Cube"), String::from("View:Reset View")],
            ),
            (
                String::from("ribbon_insert_order"),
                vec![String::from("Cube"), String::from("Cylinder")],
            ),
            (
                String::from("ribbon_edit_order"),
                vec![String::from("Delete"), String::from("Duplicate")],
            ),
            (
                String::from("preferred_slicer_path"),
                vec![String::from("C:\\Apps\\slicer.exe")],
            ),
            (
                String::from("preferred_slicer_label"),
                vec![String::from("My Slicer")],
            ),
        ]);

        let report = Preferences::load_with_legacy(&path, &legacy).expect("legacy migration");

        assert_eq!(
            report.preferences.default_import_unit(),
            ImportUnit::Micrometre
        );
        assert_eq!(report.preferences.theme_mode(), ThemeMode::Light);
        assert_eq!(
            report.preferences.edit_ribbon_order(),
            &[String::from("Insert:Cube")]
        );
        assert_eq!(
            report.preferences.ribbon_order("Insert"),
            Some([String::from("Cube"), String::from("Cylinder")].as_slice())
        );
        assert_eq!(
            report.preferences.preferred_slicer_path(),
            Some(Path::new("C:\\Apps\\slicer.exe"))
        );
        assert_eq!(
            report.preferences.preferred_slicer_label(),
            Some("My Slicer")
        );
        assert!(report.migrated_legacy);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn current_empty_recent_list_wins_over_old_python_recent_paths() {
        let path = std::env::temp_dir().join(format!(
            "modeler3d-r8-empty-current-recents-{}.json",
            std::process::id()
        ));
        fs::write(&path, br#"{"recent_files":[]}"#).expect("settings file");
        let legacy = LegacySettings {
            current: BTreeMap::from([(String::from("theme_mode"), vec![String::from("dark")])]),
            legacy: BTreeMap::from([(String::from("recent_files"), vec![String::from("old.stl")])]),
            current_present: BTreeSet::from([
                String::from("recent_files"),
                String::from("theme_mode"),
            ]),
            legacy_present: BTreeSet::from([String::from("recent_files")]),
            ..LegacySettings::default()
        };

        let report = Preferences::load_with_legacy(&path, &legacy).expect("settings");
        assert!(report.preferences.recent_files().paths().is_empty());
        let _ = fs::remove_file(path);
    }

    #[test]
    fn missing_json_migrates_python_values_but_malformed_json_is_not_overwritten() {
        let base =
            std::env::temp_dir().join(format!("modeler3d-r8-migration-{}", std::process::id()));
        let missing = base.with_extension("missing.json");
        let legacy = legacy_with_current([
            (String::from("theme_mode"), vec![String::from("light")]),
            (
                String::from("default_import_unit"),
                vec![String::from("ft")],
            ),
        ]);
        let report =
            Preferences::load_with_legacy(&missing, &legacy).expect("migrate missing file");
        assert!(!report.json_file_loaded);
        assert!(report.migrated_legacy);
        assert_eq!(report.preferences.theme_mode(), ThemeMode::Light);
        assert_eq!(report.preferences.default_import_unit(), ImportUnit::Foot);
        assert!(!missing.exists());

        let malformed = base.with_extension("malformed.json");
        fs::write(&malformed, b"{ broken").expect("malformed settings file");
        assert!(matches!(
            Preferences::load_with_legacy(&malformed, &legacy),
            Err(PreferencesError::Decode(_))
        ));
        assert_eq!(fs::read(&malformed).expect("original remains"), b"{ broken");
        let _ = fs::remove_file(malformed);
    }
}
