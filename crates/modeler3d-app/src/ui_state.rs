//! Toolkit-neutral presentation state helpers for the native shell.
//!
//! These functions keep ribbon preference normalization, recent-file fallback
//! presentation, and keyboard-friendly ordering decisions testable without a
//! native window or GPU context.

use std::path::{Path, PathBuf};

use crate::commands::{CommandDescriptor, RibbonTab, ribbon_spec};

/// The surface currently shown in the main content area.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum WorkspaceSurface {
    /// The start page with new/open/import actions and recent files.
    Welcome,
    /// The editable scene workspace.
    Scene,
}

/// The bounded state used to render one recent-file tile.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum RecentEntryKind {
    /// The path exists and has a supported 3D Modeler format.
    Supported,
    /// The path no longer exists.
    Missing,
    /// The path exists but is not a supported project/model format.
    Unsupported,
}

/// A recent path plus the information needed for safe fallback presentation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct RecentEntry {
    /// The persisted path, never rewritten by presentation code.
    pub path: PathBuf,
    /// Compact display name shown in the tile.
    pub label: String,
    /// Whether opening the tile can be attempted.
    pub kind: RecentEntryKind,
}

/// Builds one safe recent-file presentation record.
#[must_use]
pub(crate) fn recent_entry(path: &Path) -> RecentEntry {
    let label = path
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.trim().is_empty())
        .map_or_else(|| path.display().to_string(), ToOwned::to_owned);
    let kind = if !path.is_file() {
        RecentEntryKind::Missing
    } else if supported_extension(path) {
        RecentEntryKind::Supported
    } else {
        RecentEntryKind::Unsupported
    };
    RecentEntry {
        path: path.to_path_buf(),
        label,
        kind,
    }
}

/// Returns whether the native file workflow owns this extension.
#[must_use]
pub(crate) fn supported_extension(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|extension| extension.to_str())
            .map(|extension| extension.to_ascii_lowercase())
            .as_deref(),
        Some("3dp" | "stl" | "obj" | "ply" | "3mf" | "gltf" | "glb" | "wrl")
    )
}

/// Creates a stable preference key for a ribbon command.
#[must_use]
pub(crate) fn command_key(tab: RibbonTab, command: CommandDescriptor) -> String {
    format!("{}:{}", tab.label(), command.label)
}

/// Returns all command descriptors in one tab, excluding separators.
#[must_use]
pub(crate) fn tab_commands(tab: RibbonTab) -> Vec<CommandDescriptor> {
    ribbon_spec()
        .into_iter()
        .find(|(candidate, _)| *candidate == tab)
        .map(|(_, items)| {
            items
                .into_iter()
                .filter_map(|item| match item {
                    crate::commands::RibbonItem::Command(command) => Some(command),
                    crate::commands::RibbonItem::Separator => None,
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Removes stale and duplicate favorites while preserving the user's order.
#[must_use]
pub(crate) fn normalize_favorites(
    values: impl IntoIterator<Item = String>,
    available: impl IntoIterator<Item = String>,
) -> Vec<String> {
    let available = available.into_iter().collect::<Vec<_>>();
    let mut normalized = Vec::new();
    for value in values {
        if available.iter().any(|candidate| candidate == &value)
            && !normalized.iter().any(|candidate| candidate == &value)
        {
            normalized.push(value);
        }
    }
    normalized
}

/// Moves one favorite by one slot, returning `None` when no move is possible.
#[must_use]
pub(crate) fn move_favorite(values: &[String], key: &str, delta: isize) -> Option<Vec<String>> {
    let index = values.iter().position(|value| value == key)?;
    let target = index.checked_add_signed(delta)?;
    if target >= values.len() {
        return None;
    }
    let mut next = values.to_vec();
    next.swap(index, target);
    Some(next)
}

/// Moves a dragged favorite to the position of the favorite it was dropped on.
#[must_use]
pub(crate) fn move_favorite_to(
    values: &[String],
    dragged: &str,
    target: &str,
) -> Option<Vec<String>> {
    let source_index = values.iter().position(|value| value == dragged)?;
    let target_index = values.iter().position(|value| value == target)?;
    if source_index == target_index {
        return None;
    }
    let mut next = values.to_vec();
    let item = next.remove(source_index);
    next.insert(target_index, item);
    Some(next)
}

/// Produces a complete tab order from a persisted partial order.
#[must_use]
pub(crate) fn normalized_tab_order(
    requested: impl IntoIterator<Item = String>,
    defaults: &[String],
) -> Vec<String> {
    let mut order = Vec::new();
    for value in requested {
        if defaults.iter().any(|candidate| candidate == &value)
            && !order.iter().any(|candidate| candidate == &value)
        {
            order.push(value);
        }
    }
    for value in defaults {
        if !order.iter().any(|candidate| candidate == value) {
            order.push(value.clone());
        }
    }
    order
}

/// Returns a small default favorite set that fits the compact strip.
#[must_use]
pub(crate) fn default_favorites() -> Vec<String> {
    [
        (RibbonTab::Insert, "Cube"),
        (RibbonTab::Object, "Duplicate"),
        (RibbonTab::Object, "Delete"),
        (RibbonTab::View, "Reset View"),
    ]
    .into_iter()
    .filter_map(|(tab, label)| {
        tab_commands(tab)
            .into_iter()
            .find(|command| command.label == label)
            .map(|command| command_key(tab, command))
    })
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recent_paths_have_bounded_missing_and_unsupported_fallbacks() {
        let missing = recent_entry(Path::new("missing-model.stl"));
        assert_eq!(missing.kind, RecentEntryKind::Missing);
        assert_eq!(missing.label, "missing-model.stl");
        let unsupported_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
        let unsupported = recent_entry(&unsupported_path);
        assert_eq!(unsupported.kind, RecentEntryKind::Unsupported);
    }

    #[test]
    fn favorites_drop_stale_duplicates_and_support_reordering() {
        let available = vec![String::from("Insert:Cube"), String::from("Object:Delete")];
        let values = normalize_favorites(
            [
                String::from("Object:Delete"),
                String::from("Object:Delete"),
                String::from("stale"),
                String::from("Insert:Cube"),
            ],
            available,
        );
        assert_eq!(values, ["Object:Delete", "Insert:Cube"]);
        assert_eq!(
            move_favorite(&values, "Object:Delete", 1),
            Some(vec![
                String::from("Insert:Cube"),
                String::from("Object:Delete")
            ])
        );
        assert_eq!(
            move_favorite_to(&values, "Insert:Cube", "Object:Delete"),
            Some(vec![
                String::from("Insert:Cube"),
                String::from("Object:Delete")
            ])
        );
        assert!(move_favorite(&values, "Object:Delete", -1).is_none());
    }

    #[test]
    fn tab_order_fills_missing_commands_without_stale_entries() {
        let defaults = vec![String::from("Cube"), String::from("Cylinder")];
        assert_eq!(
            normalized_tab_order([String::from("stale"), String::from("Cylinder")], &defaults),
            vec![String::from("Cylinder"), String::from("Cube")]
        );
    }
}
