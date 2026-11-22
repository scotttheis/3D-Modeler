//! Desktop side of the MCP shell tools (owner batch 7, OF-58, decision D10:
//! the AI can do everything the desktop does, including customising the app).
//!
//! The MCP crate owns these tools' catalog entries, schemas and checks
//! (`R4Server::call_shell_value`); this module performs them with the same
//! shell setters the desktop's own buttons, menus and Settings window call.
//! Nothing here changes the model, so there is no undo entry; settings that
//! the desktop persists are persisted the same way.

use std::path::{Path, PathBuf};

use modeler3d_mcp::AdapterError;
use serde_json::{Map, Value, json};

use super::{Modeler3dApp, all_command_keys, command_enabled, normalized_tab_order, tab_commands};
use crate::commands::{CommandDescriptor, CommandId, DialogKind, ImportUnit, RibbonTab};
use crate::grab_cursor::GrabCursorStyle;
use crate::preferences::{DisplayUnit, MAX_RIBBON_FAVORITES};
use crate::theme::{ThemeMode, UiSize};
use crate::ui_state::{
    RecentEntryKind, WorkspaceSurface, command_key, default_favorites, recent_entry,
};
use modeler3d_viewport::ViewportLighting;

const ABOUT_DESCRIPTION: &str =
    "A reverse-engineering-informed 3D model repair and building application.";

/// What opening a recent entry did.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum RecentOpen {
    /// A `.3dp` project replaced the scene.
    Project,
    /// A model was imported into the current scene.
    Model,
    /// The file no longer exists; its entry was removed.
    Missing,
    /// Unsaved changes: the desktop's save prompt is showing.
    AskedToSave,
}

/// Whether a recent entry opens as a project rather than importing a model.
pub(super) fn recent_is_project(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("3dp"))
}

impl Modeler3dApp {
    // ----- shared setters: desktop widgets and MCP both call these -----

    pub(super) fn apply_theme_mode(&mut self, mode: ThemeMode) {
        self.theme_mode = mode;
        self.commands.preferences_mut().set_theme_mode(mode);
        if let Some(dialog) = self.dialog.as_mut() {
            dialog.sync_theme_mode(mode);
        }
        self.artwork_cache.clear();
        self.persist_preferences();
    }

    pub(super) fn apply_ui_size(&mut self, size: UiSize) {
        self.ui_size = size;
        self.commands.preferences_mut().set_ui_size(size);
        if let Some(dialog) = self.dialog.as_mut() {
            dialog.sync_ui_size(size);
        }
        self.persist_preferences();
    }

    pub(super) fn apply_display_unit(&mut self, unit: DisplayUnit) {
        self.commands.preferences_mut().set_display_unit(unit);
        if let Some(dialog) = self.dialog.as_mut() {
            dialog.set_display_unit(unit);
        }
        self.transform_signature.clear();
        self.persist_preferences();
    }

    pub(super) fn apply_default_import_unit(&mut self, unit: ImportUnit) {
        self.commands
            .preferences_mut()
            .set_default_import_unit(unit);
        if let Some(dialog) = self.dialog.as_mut() {
            dialog.set_import_unit(unit);
        }
        self.persist_preferences();
    }

    /// The Settings lighting sliders (light, highlight, floor shadow).
    pub(super) fn apply_viewport_lighting(&mut self, lighting: ViewportLighting) {
        self.commands
            .preferences_mut()
            .set_viewport_lighting(lighting);
        if let Some(dialog) = self.dialog.as_mut() {
            dialog.set_viewport_lighting(lighting);
        }
        self.persist_preferences();
    }

    /// The Settings grab cursor choice: robot claw or hand.
    pub(super) fn apply_grab_cursor(&mut self, style: GrabCursorStyle) {
        self.commands.preferences_mut().set_grab_cursor(style);
        if let Some(dialog) = self.dialog.as_mut() {
            dialog.set_grab_cursor(style);
        }
        self.persist_preferences();
    }

    /// The saved slicer used by the Slicer button (Settings Browse and
    /// Clear, the first-use chooser, and the Print window "remember").
    pub(super) fn apply_preferred_slicer(&mut self, path: Option<PathBuf>, label: Option<String>) {
        self.commands
            .preferences_mut()
            .set_preferred_slicer(path.clone(), label.clone());
        if let Some(dialog) = self.dialog.as_mut() {
            dialog.set_preferred_slicer(path, label);
        }
        self.persist_preferences();
    }

    /// The Pin button: a pinned row is always open.
    pub(super) fn apply_ribbon_pinned(&mut self, pinned: bool) {
        self.ribbon_pinned = pinned;
        if pinned {
            self.ribbon_expanded = true;
        }
        self.commands.preferences_mut().set_ribbon_pinned(pinned);
        self.persist_preferences();
    }

    /// The menu's Start screen item: the Welcome screen with recent files.
    /// The scene stays in memory.
    pub(super) const fn show_start_screen(&mut self) {
        self.surface = WorkspaceSurface::Welcome;
    }

    /// Clicking another tab: show it with its command row open.
    pub(super) fn show_ribbon_tab(&mut self, tab: RibbonTab) {
        if tab != self.active_tab {
            self.active_tab = tab;
            self.artwork_cache.clear();
        }
        self.ribbon_expanded = true;
    }

    /// A tab's commands in the order the ribbon shows them (the user's saved
    /// order, then any commands it does not name, in default order).
    pub(super) fn ordered_tab_commands(&self, tab: RibbonTab) -> Vec<CommandDescriptor> {
        let default_items = tab_commands(tab);
        let defaults = default_items
            .iter()
            .map(|command| command.label.to_owned())
            .collect::<Vec<_>>();
        let requested = self
            .commands
            .preferences()
            .ribbon_order(tab.label())
            .map(<[String]>::to_vec)
            .unwrap_or_default();
        normalized_tab_order(requested, &defaults)
            .into_iter()
            .filter_map(|label| default_items.iter().find(|item| item.label == label))
            .copied()
            .collect()
    }

    /// Removes one recent entry (the Welcome list's Remove button). The file
    /// itself is never touched.
    pub(super) fn remove_recent_file(&mut self, path: &Path) -> bool {
        let removed = self
            .commands
            .preferences_mut()
            .recent_files_mut()
            .remove(path);
        if removed {
            self.persist_preferences();
        }
        removed
    }

    /// Settings' Clear Recent Files. The files themselves are never touched.
    pub(super) fn clear_recent_files(&mut self) {
        self.commands.preferences_mut().recent_files_mut().clear();
        self.invalidate_recent_previews_after_clear();
        self.persist_preferences();
    }

    // ----- MCP entry point -----

    /// Performs one MCP shell tool (all but `capture_window`, which the
    /// bridge completes with a later frame) and returns its raw result.
    pub(crate) fn call_shell_tool(
        &mut self,
        name: &str,
        args: &Map<String, Value>,
    ) -> Result<Value, AdapterError> {
        match name {
            "get_update_status" => Ok(self.updater.status()),
            "respond_to_update" => {
                let response = optional_str(args, "response").unwrap_or("");
                let version = optional_str(args, "version").unwrap_or("");
                let close = self
                    .updater
                    .respond(response, version)
                    .map_err(AdapterError::Conflict)?;
                if close {
                    self.updater.hide();
                    self.request_exit();
                }
                Ok(self.updater.status())
            }
            "app_settings" => Ok(self.app_settings_value(None)),
            "set_app_settings" => self.mcp_set_app_settings(args),
            "customize_ribbon" => self.mcp_customize_ribbon(args),
            "show_screen" => self.mcp_show_screen(args),
            "recent_files" => self.mcp_recent_files(args),
            "open_recent" => self.mcp_open_recent(args),
            "exit_app" => self.mcp_exit_app(args),
            other => Err(AdapterError::Internal(format!(
                "{other} is not carried out by the desktop shell"
            ))),
        }
    }

    fn app_settings_value(&self, message: Option<String>) -> Value {
        let preferences = self.commands.preferences();
        let favorites = preferences.ribbon_favorites().to_vec();
        let tabs = RibbonTab::all()
            .into_iter()
            .map(|tab| {
                let commands = self
                    .ordered_tab_commands(tab)
                    .into_iter()
                    .map(|command| {
                        let key = command_key(tab, command);
                        json!({
                            "favorite": favorites.contains(&key),
                            "key": key,
                            "label": command.label,
                            "enabled": command_enabled(command.id, &self.commands),
                        })
                    })
                    .collect::<Vec<_>>();
                json!({"tab": tab.label(), "commands": commands})
            })
            .collect::<Vec<_>>();
        let open_window = self.dialog.as_ref().and_then(|dialog| {
            (dialog.kind() == DialogKind::Settings).then(|| {
                if dialog.about_is_open() {
                    "about"
                } else {
                    "settings"
                }
            })
        });
        json!({
            "theme": theme_name(self.theme_mode),
            "size": size_name(self.ui_size),
            "display_unit": preferences.display_unit().label(),
            "default_import_unit": preferences.default_import_unit().label(),
            "sticky_selection": self.sticky_selection,
            "resize_basis": if self.object_resize_basis { "object" } else { "grid" },
            "keep_proportions": self.proportional_resize,
            "object_snapping": self.commands.snap_enabled(),
            "screen": match self.surface {
                WorkspaceSurface::Welcome => "start",
                WorkspaceSurface::Scene => "scene",
            },
            "open_window": open_window,
            "ribbon": {
                "pinned": self.ribbon_pinned,
                "open": self.ribbon_expanded,
                "active_tab": self.active_tab.label(),
                "favorites": favorites,
                "tabs": tabs,
            },
            "recent_file_count": preferences.recent_files().paths().len(),
            "about": {
                "product": "3D Modeler",
                "version": env!("CARGO_PKG_VERSION"),
                "description": ABOUT_DESCRIPTION,
            },
            "window_title": self.window_title(),
            "lighting": lighting_value(preferences.viewport_lighting()),
            "grab_cursor": preferences.grab_cursor().key(),
            "saved_slicer": preferences.preferred_slicer_path().map(|path| json!({
                "path": path.display().to_string(),
                "label": preferences.preferred_slicer_label(),
            })),
            "settings_notice": self.settings_warning,
            "message": message,
        })
    }

    fn mcp_set_app_settings(&mut self, args: &Map<String, Value>) -> Result<Value, AdapterError> {
        // Read every value before changing anything, so a bad one changes
        // nothing.
        let theme = optional_str(args, "theme")
            .map(|value| {
                parse_choice(
                    value,
                    &[("light", ThemeMode::Light), ("dark", ThemeMode::Dark)],
                )
            })
            .transpose()?;
        let size = optional_str(args, "size")
            .map(|value| parse_choice(value, &[("normal", UiSize::Normal), ("big", UiSize::Big)]))
            .transpose()?;
        let display_unit = optional_str(args, "display_unit")
            .map(|value| {
                parse_choice(
                    value,
                    &[("mm", DisplayUnit::Millimetre), ("in", DisplayUnit::Inch)],
                )
            })
            .transpose()?;
        let import_unit = optional_str(args, "default_import_unit")
            .map(|value| {
                let choices = ImportUnit::all().map(|unit| (unit.label(), unit));
                parse_choice(value, &choices)
            })
            .transpose()?;
        let resize_basis = optional_str(args, "resize_basis")
            .map(|value| parse_choice(value, &[("grid", false), ("object", true)]))
            .transpose()?;
        let grab_cursor = optional_str(args, "grab_cursor")
            .map(|value| {
                let choices = GrabCursorStyle::ALL.map(|style| (style.key(), style));
                parse_choice(value, &choices)
            })
            .transpose()?;
        let sticky = optional_bool(args, "sticky_selection");
        let keep_proportions = optional_bool(args, "keep_proportions");
        let pinned = optional_bool(args, "ribbon_pinned");
        let current_lighting = self.commands.preferences().viewport_lighting();
        let base_lighting = if optional_bool(args, "reset_lighting") == Some(true) {
            ViewportLighting::default()
        } else {
            current_lighting
        };
        let lighting = match args.get("lighting").and_then(Value::as_object) {
            Some(change) => merge_lighting(base_lighting, change)?,
            None => base_lighting,
        };
        let dismiss_notice = optional_bool(args, "dismiss_settings_notice") == Some(true);
        let clear_slicer = optional_bool(args, "clear_slicer") == Some(true);
        let slicer = match optional_str(args, "slicer") {
            Some(_) if clear_slicer => {
                return Err(AdapterError::Validation(
                    "give slicer or clear_slicer, not both".to_owned(),
                ));
            }
            // Only a slicer 3D Modeler found itself can be saved: the same
            // ones launch_slicer may start.
            Some(wanted) => Some(
                modeler3d_platform::discover_slicers()
                    .into_iter()
                    .find(|slicer| slicer.key == wanted || slicer.path.to_string_lossy() == wanted)
                    .ok_or_else(|| {
                        AdapterError::NotFound(format!(
                            "{wanted} is not a slicer found on this computer; see discover_slicers"
                        ))
                    })?,
            ),
            None => None,
        };

        let mut changed = Vec::new();
        if let Some(mode) = theme.filter(|mode| *mode != self.theme_mode) {
            self.apply_theme_mode(mode);
            changed.push("theme");
        }
        if let Some(size) = size.filter(|size| *size != self.ui_size) {
            self.apply_ui_size(size);
            changed.push("size");
        }
        if let Some(unit) =
            display_unit.filter(|unit| *unit != self.commands.preferences().display_unit())
        {
            self.apply_display_unit(unit);
            changed.push("display unit");
        }
        if let Some(unit) =
            import_unit.filter(|unit| *unit != self.commands.preferences().default_import_unit())
        {
            self.apply_default_import_unit(unit);
            changed.push("default import unit");
        }
        if let Some(sticky) = sticky.filter(|sticky| *sticky != self.sticky_selection) {
            self.sticky_selection = sticky;
            changed.push("sticky selection");
        }
        if let Some(object) = resize_basis.filter(|object| *object != self.object_resize_basis) {
            self.object_resize_basis = object;
            self.transform_signature.clear();
            changed.push("resize basis");
        }
        if let Some(keep) = keep_proportions.filter(|keep| *keep != self.proportional_resize) {
            self.proportional_resize = keep;
            changed.push("keep proportions");
        }
        if let Some(pinned) = pinned.filter(|pinned| *pinned != self.ribbon_pinned) {
            self.apply_ribbon_pinned(pinned);
            changed.push("ribbon pin");
        }
        if lighting != current_lighting {
            self.apply_viewport_lighting(lighting);
            changed.push("lighting");
        }
        if let Some(style) =
            grab_cursor.filter(|style| *style != self.commands.preferences().grab_cursor())
        {
            self.apply_grab_cursor(style);
            changed.push("grab cursor");
        }
        if let Some(slicer) = slicer {
            self.apply_preferred_slicer(Some(slicer.path), Some(slicer.label));
            changed.push("saved slicer");
        } else if clear_slicer
            && self
                .commands
                .preferences()
                .preferred_slicer_path()
                .is_some()
        {
            self.apply_preferred_slicer(None, None);
            changed.push("saved slicer");
        }
        if dismiss_notice && self.dismiss_settings_note() {
            changed.push("settings note dismissed");
        }
        let message = if changed.is_empty() {
            String::from("No settings changed")
        } else {
            format!("Changed {}", changed.join(", "))
        };
        Ok(self.app_settings_value(Some(message)))
    }

    fn mcp_customize_ribbon(&mut self, args: &Map<String, Value>) -> Result<Value, AdapterError> {
        let available = all_command_keys();
        let check_key = |key: &str| -> Result<String, AdapterError> {
            let key = key.trim();
            if available.iter().any(|candidate| candidate == key) {
                Ok(key.to_owned())
            } else {
                Err(AdapterError::Validation(format!(
                    "unknown ribbon command {key:?}; app_settings lists every key as 'Tab:Label'"
                )))
            }
        };
        let tab = optional_str(args, "tab").map(parse_tab).transpose()?;
        let favorites = match args.get("favorites").and_then(Value::as_array) {
            Some(values) => Some(
                values
                    .iter()
                    .map(|value| check_key(value.as_str().unwrap_or_default()))
                    .collect::<Result<Vec<_>, _>>()?,
            ),
            None => None,
        };
        let add = optional_str(args, "add_favorite")
            .map(check_key)
            .transpose()?;
        let remove = optional_str(args, "remove_favorite")
            .map(check_key)
            .transpose()?;
        let tab_order = match args.get("tab_order").and_then(Value::as_array) {
            Some(values) => {
                let tab =
                    tab.ok_or_else(|| AdapterError::Validation("tab_order needs tab".to_owned()))?;
                let labels = tab_commands(tab)
                    .into_iter()
                    .map(|command| command.label.to_owned())
                    .collect::<Vec<_>>();
                let order = values
                    .iter()
                    .map(|value| {
                        let label = value.as_str().unwrap_or_default().trim();
                        labels
                            .iter()
                            .find(|candidate| candidate.as_str() == label)
                            .cloned()
                            .ok_or_else(|| {
                                AdapterError::Validation(format!(
                                    "{label:?} is not a command on the {} tab",
                                    tab.label()
                                ))
                            })
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                Some((tab, normalized_tab_order(order, &labels)))
            }
            None => None,
        };
        let reset = optional_str(args, "reset");
        if reset == Some("tab_order") && tab.is_none() {
            return Err(AdapterError::Validation(
                "reset=tab_order needs tab".to_owned(),
            ));
        }

        let before_favorites = self.commands.preferences().ribbon_favorites().to_vec();
        let before_orders = RibbonTab::all().map(|tab| self.ordered_tab_commands(tab));
        let mut next = before_favorites.clone();
        if matches!(reset, Some("favorites" | "all")) {
            next = default_favorites();
        }
        let reset_tabs: Vec<RibbonTab> = match (reset, tab) {
            (Some("all"), _) => RibbonTab::all().to_vec(),
            (Some("tab_order"), Some(tab)) => vec![tab],
            _ => Vec::new(),
        };
        if let Some(favorites) = favorites {
            next = favorites;
        }
        if let Some(add) = add
            && !next.contains(&add)
        {
            next.push(add);
        }
        if let Some(remove) = remove {
            next.retain(|favorite| favorite != &remove);
        }
        if next.len() > MAX_RIBBON_FAVORITES {
            return Err(AdapterError::Validation(format!(
                "the favorites strip holds at most {MAX_RIBBON_FAVORITES} commands"
            )));
        }
        self.commands.preferences_mut().set_ribbon_favorites(next);
        for tab in reset_tabs {
            let defaults = tab_commands(tab)
                .into_iter()
                .map(|command| command.label.to_owned())
                .collect::<Vec<_>>();
            self.commands
                .preferences_mut()
                .set_ribbon_order(tab.label(), defaults);
        }
        if let Some((tab, order)) = tab_order {
            self.commands
                .preferences_mut()
                .set_ribbon_order(tab.label(), order);
        }
        let favorites_changed =
            self.commands.preferences().ribbon_favorites() != before_favorites.as_slice();
        let orders_changed =
            RibbonTab::all().map(|tab| self.ordered_tab_commands(tab)) != before_orders;
        let message = if favorites_changed || orders_changed {
            self.artwork_cache.clear();
            self.persist_preferences();
            "Ribbon updated"
        } else {
            "Ribbon unchanged"
        };
        Ok(self.app_settings_value(Some(message.to_owned())))
    }

    fn mcp_show_screen(&mut self, args: &Map<String, Value>) -> Result<Value, AdapterError> {
        let screen = optional_str(args, "screen");
        let tab = optional_str(args, "ribbon_tab")
            .map(parse_tab)
            .transpose()?;
        let open = optional_bool(args, "ribbon_open");
        let mut notes = Vec::new();
        // On the desktop, reaching the menu or the scene is a press outside
        // Settings, which closes it (OF-17).
        if matches!(screen, Some("start" | "scene"))
            && self.dialog.as_ref().map(|dialog| dialog.kind()) == Some(DialogKind::Settings)
        {
            self.dialog = None;
        }
        match screen {
            Some("start") => {
                self.show_start_screen();
                notes.push("Showing the start screen");
            }
            Some("scene") => {
                self.surface = WorkspaceSurface::Scene;
                self.invalidate_presentation();
                notes.push("Showing the scene");
            }
            Some(window @ ("settings" | "about")) => {
                if self.dialog.as_ref().map(|dialog| dialog.kind()) != Some(DialogKind::Settings) {
                    if self.modal_active() {
                        return Err(AdapterError::Conflict(
                            "another window is waiting for the user; close it first".to_owned(),
                        ));
                    }
                    self.dispatch(CommandId::Settings);
                }
                if window == "about"
                    && let Some(dialog) = self.dialog.as_mut()
                {
                    dialog.open_about();
                }
                notes.push(if window == "about" {
                    "About is open"
                } else {
                    "Settings is open"
                });
            }
            Some(other) => {
                return Err(AdapterError::Validation(format!(
                    "unknown screen {other:?}"
                )));
            }
            None => {}
        }
        if let Some(tab) = tab {
            self.show_ribbon_tab(tab);
            notes.push("Ribbon tab shown");
        }
        match open {
            Some(false) if self.ribbon_pinned => {
                notes.push("The ribbon is pinned, so its command row stays open");
            }
            Some(open) => self.ribbon_expanded = open,
            None => {}
        }
        let message = if notes.is_empty() {
            String::from("Nothing to show")
        } else {
            notes.join("; ")
        };
        Ok(self.app_settings_value(Some(message)))
    }

    fn recent_list_value(&self) -> Vec<Value> {
        self.commands
            .preferences()
            .recent_files()
            .paths()
            .iter()
            .enumerate()
            .map(|(index, path)| {
                let entry = recent_entry(path);
                let kind = match entry.kind {
                    RecentEntryKind::Missing => "missing",
                    RecentEntryKind::Unsupported => "unsupported",
                    RecentEntryKind::Supported if recent_is_project(path) => "project",
                    RecentEntryKind::Supported => "model",
                };
                json!({
                    "index": index,
                    "path": path.display().to_string(),
                    "name": entry.label,
                    "kind": kind,
                })
            })
            .collect()
    }

    /// Finds a listed recent entry by index or exact listed path. MCP can
    /// only open or remove what the desktop's Welcome list shows.
    fn listed_recent(&self, args: &Map<String, Value>) -> Result<PathBuf, AdapterError> {
        let paths = self.commands.preferences().recent_files().paths();
        match (
            args.get("index").and_then(Value::as_u64),
            optional_str(args, "path"),
        ) {
            (Some(_), Some(_)) => Err(AdapterError::Validation(
                "give index or path, not both".to_owned(),
            )),
            (Some(index), None) => usize::try_from(index)
                .ok()
                .and_then(|index| paths.get(index))
                .cloned()
                .ok_or_else(|| {
                    AdapterError::NotFound(format!(
                        "no recent file at index {index}; the list has {}",
                        paths.len()
                    ))
                }),
            (None, Some(raw)) => paths
                .iter()
                .find(|path| path.display().to_string() == raw || path.as_path() == Path::new(raw))
                .cloned()
                .ok_or_else(|| {
                    AdapterError::NotFound(format!("{raw} is not in the recent files list"))
                }),
            (None, None) => Err(AdapterError::Validation(
                "give the recent entry's index or path".to_owned(),
            )),
        }
    }

    fn mcp_recent_files(&mut self, args: &Map<String, Value>) -> Result<Value, AdapterError> {
        let mut removed = Vec::new();
        let message = match optional_str(args, "action").unwrap_or("list") {
            "list" => String::from("Recent files, newest first"),
            "remove" => {
                let path = self.listed_recent(args)?;
                self.remove_recent_file(&path);
                removed.push(path.display().to_string());
                String::from("Removed the recent entry; the file was not changed")
            }
            "clear" => {
                removed = self
                    .commands
                    .preferences()
                    .recent_files()
                    .paths()
                    .iter()
                    .map(|path| path.display().to_string())
                    .collect();
                self.clear_recent_files();
                String::from("Recent files cleared. The source files were not changed.")
            }
            other => {
                return Err(AdapterError::Validation(format!(
                    "unknown action {other:?}"
                )));
            }
        };
        Ok(json!({
            "recent_files": self.recent_list_value(),
            "removed": removed,
            "message": message,
        }))
    }

    fn mcp_open_recent(&mut self, args: &Map<String, Value>) -> Result<Value, AdapterError> {
        let path = self.listed_recent(args)?;
        if path.exists() && recent_is_project(&path) && self.commands.is_dirty() {
            // Same rule as MCP open_project: never drop unsaved work.
            return Err(AdapterError::Conflict(
                "the current project has unsaved changes; save it before opening a project"
                    .to_owned(),
            ));
        }
        let outcome = self
            .try_open_recent(&path, false)
            .map_err(|error| AdapterError::Validation(error.to_string()))?;
        let (kind, opened, message) = match outcome {
            RecentOpen::Project => ("project", true, format!("Opened {}", path.display())),
            RecentOpen::Model => ("model", true, format!("Imported {}", path.display())),
            RecentOpen::Missing => (
                "missing",
                false,
                format!("File not found; removed recent entry: {}", path.display()),
            ),
            RecentOpen::AskedToSave => {
                return Err(AdapterError::Internal(
                    "open_recent must not prompt the user".to_owned(),
                ));
            }
        };
        self.commands.set_status(message.clone());
        Ok(json!({
            "path": path.display().to_string(),
            "kind": kind,
            "opened": opened,
            "message": message,
            "project_id": self.commands.scene().project_id(),
            "scene_revision": self.commands.scene().revision(),
        }))
    }

    fn mcp_exit_app(&mut self, args: &Map<String, Value>) -> Result<Value, AdapterError> {
        let dirty = self.commands.is_dirty();
        let (exiting, asked_user, message) =
            match (dirty, optional_str(args, "if_unsaved").unwrap_or("refuse")) {
                (false, _) => (true, false, "Closing 3D Modeler"),
                (true, "discard") => (true, false, "Closing 3D Modeler without saving"),
                (true, "ask") => {
                    // Exactly the Exit button: the Save / Don't save / Cancel
                    // prompt, answered by the user.
                    self.request_exit();
                    (false, true, "Asked the user to save, discard or cancel")
                }
                (true, "refuse") => {
                    return Err(AdapterError::Conflict(
                        "the current project has unsaved changes; save it first, or pass \
                         if_unsaved=ask or if_unsaved=discard"
                            .to_owned(),
                    ));
                }
                (true, other) => {
                    return Err(AdapterError::Validation(format!(
                        "unknown if_unsaved {other:?}"
                    )));
                }
            };
        if exiting {
            self.close_requested = true;
        }
        Ok(json!({"exiting": exiting, "asked_user": asked_user, "message": message}))
    }
}

fn lighting_value(lighting: ViewportLighting) -> Value {
    json!({
        "screen_aligned": lighting.screen_aligned,
        "azimuth_degrees": lighting.azimuth_degrees,
        "elevation_degrees": lighting.elevation_degrees,
        "intensity_percent": lighting.intensity_percent,
        "ambient_percent": lighting.ambient_percent,
        "specular_percent": lighting.specular_percent,
        "highlight_sharpness": lighting.highlight_sharpness,
        "contact_shadow_percent": lighting.contact_shadow_percent,
    })
}

/// Applies the given lighting values over `base`. The schema has already
/// checked each range; a value that still does not fit is refused.
fn merge_lighting(
    base: ViewportLighting,
    change: &Map<String, Value>,
) -> Result<ViewportLighting, AdapterError> {
    fn number<T: TryFrom<i64>>(
        change: &Map<String, Value>,
        key: &str,
        current: T,
    ) -> Result<T, AdapterError> {
        match change.get(key).and_then(Value::as_i64) {
            Some(value) => T::try_from(value)
                .map_err(|_| AdapterError::Validation(format!("{key} is out of range"))),
            None => Ok(current),
        }
    }
    Ok(ViewportLighting {
        screen_aligned: change
            .get("screen_aligned")
            .and_then(Value::as_bool)
            .unwrap_or(base.screen_aligned),
        azimuth_degrees: number(change, "azimuth_degrees", base.azimuth_degrees)?,
        elevation_degrees: number(change, "elevation_degrees", base.elevation_degrees)?,
        intensity_percent: number(change, "intensity_percent", base.intensity_percent)?,
        ambient_percent: number(change, "ambient_percent", base.ambient_percent)?,
        specular_percent: number(change, "specular_percent", base.specular_percent)?,
        highlight_sharpness: number(change, "highlight_sharpness", base.highlight_sharpness)?,
        contact_shadow_percent: number(
            change,
            "contact_shadow_percent",
            base.contact_shadow_percent,
        )?,
    }
    .normalized())
}

fn optional_str<'a>(args: &'a Map<String, Value>, key: &str) -> Option<&'a str> {
    args.get(key).and_then(Value::as_str)
}

fn optional_bool(args: &Map<String, Value>, key: &str) -> Option<bool> {
    args.get(key).and_then(Value::as_bool)
}

fn parse_choice<T: Copy>(value: &str, choices: &[(&str, T)]) -> Result<T, AdapterError> {
    choices
        .iter()
        .find(|(name, _)| *name == value)
        .map(|(_, choice)| *choice)
        .ok_or_else(|| AdapterError::Validation(format!("unknown value {value:?}")))
}

fn parse_tab(value: &str) -> Result<RibbonTab, AdapterError> {
    RibbonTab::all()
        .into_iter()
        .find(|tab| tab.label() == value)
        .ok_or_else(|| AdapterError::Validation(format!("unknown ribbon tab {value:?}")))
}

const fn theme_name(mode: ThemeMode) -> &'static str {
    match mode {
        ThemeMode::Light => "light",
        ThemeMode::Dark => "dark",
    }
}

const fn size_name(size: UiSize) -> &'static str {
    match size {
        UiSize::Normal => "normal",
        UiSize::Big => "big",
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use modeler3d_core::PrimitiveKind;
    use modeler3d_mcp::{AdapterError, ApprovedRoots, Profile, R4Config, R4Server};
    use serde_json::{Value, json};

    use super::super::{Modeler3dApp, PendingReplacement};
    use crate::commands::{CommandId, ImportUnit, RibbonTab};
    use crate::grab_cursor::GrabCursorStyle;
    use crate::preferences::DisplayUnit;
    use crate::theme::{ThemeMode, UiSize};
    use crate::ui_state::{WorkspaceSurface, default_favorites, tab_commands};

    /// An app with its own settings file, so tests never touch the user's.
    fn app(label: &str) -> (Modeler3dApp, PathBuf) {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |duration| duration.as_nanos());
        let directory = std::env::temp_dir()
            .join("modeler3d-of58-tests")
            .join(format!("{label}-{}-{unique}", std::process::id()));
        std::fs::create_dir_all(&directory).expect("test directory");
        let app = Modeler3dApp {
            settings_path: directory.join("settings.json"),
            ..Modeler3dApp::default()
        };
        (app, directory)
    }

    /// Runs one shell tool the way the live bridge does.
    fn mcp(app: &mut Modeler3dApp, name: &str, arguments: Value) -> Result<Value, AdapterError> {
        let roots = ApprovedRoots::from_paths(Vec::new()).expect("deny-all roots");
        let server = R4Server::new(R4Config::for_profile(Profile::Full, roots));
        server.call_shell_value(name, arguments, |args| app.call_shell_tool(name, args))
    }

    fn saved_settings(app: &Modeler3dApp) -> Value {
        serde_json::from_slice(&std::fs::read(&app.settings_path).expect("settings saved"))
            .expect("settings json")
    }

    #[test]
    fn mcp_update_consent_matches_the_card_and_rejects_stale_or_unknown_arguments() {
        let (mut ai, directory) = app("updates");
        assert_eq!(
            mcp(&mut ai, "get_update_status", json!({})).unwrap()["available_version"],
            Value::Null
        );
        assert!(matches!(
            mcp(
                &mut ai,
                "respond_to_update",
                json!({"response":"install","version":"0.2.0"})
            ),
            Err(AdapterError::Conflict(_))
        ));
        ai.updater = crate::updates::Updater::review();
        let mut desktop = crate::updates::Updater::review();
        assert!(!desktop.respond("later", "0.2.0").unwrap());
        let later = mcp(
            &mut ai,
            "respond_to_update",
            json!({"response":"later","version":"0.2.0"}),
        )
        .unwrap();
        assert_eq!(later, desktop.status());
        let repeated = mcp(
            &mut ai,
            "respond_to_update",
            json!({"response":"later","version":"0.2.0"}),
        )
        .unwrap();
        assert_eq!(later, repeated);
        assert!(matches!(
            mcp(
                &mut ai,
                "respond_to_update",
                json!({"response":"install","version":"0.3.0"})
            ),
            Err(AdapterError::Conflict(_))
        ));
        assert!(matches!(
            mcp(
                &mut ai,
                "respond_to_update",
                json!({"response":"later","version":"0.2.0","url":"https://example.com"})
            ),
            Err(AdapterError::Validation(_))
        ));
        assert!(!ai.close_requested);
        assert!(!ai.commands.is_dirty());
        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn mcp_settings_reports_all_edit_tools_available_without_a_selection() {
        let (mut app, directory) = app("edit-availability");
        let settings = mcp(&mut app, "app_settings", json!({})).expect("settings");
        let edit = settings["ribbon"]["tabs"]
            .as_array()
            .expect("tabs")
            .iter()
            .find(|tab| tab["tab"] == "Edit")
            .expect("Edit");
        let commands = edit["commands"].as_array().expect("commands");
        assert_eq!(commands.len(), 11);
        for command in commands {
            assert_eq!(command["enabled"], true, "{command}");
        }
        assert!(std::fs::remove_dir_all(directory).is_ok());
    }

    #[test]
    fn mcp_set_app_settings_matches_the_desktop_controls() {
        let (mut desktop, desktop_dir) = app("settings-desktop");
        let (mut ai, ai_dir) = app("settings-mcp");

        // Desktop: the top-bar theme toggle, the Settings size and units, the
        // Sticky box, the Resize basis and lock, and the Pin button.
        desktop.toggle_theme();
        desktop.apply_ui_size(UiSize::Big);
        desktop.apply_display_unit(DisplayUnit::Inch);
        desktop.apply_default_import_unit(ImportUnit::Centimetre);
        desktop.sticky_selection = true;
        desktop.set_resize_basis(true);
        desktop.proportional_resize = true;
        desktop.apply_ribbon_pinned(true);
        desktop.apply_dialog_action(crate::dialogs::DialogAction::GrabCursor(
            GrabCursorStyle::Hand,
        ));

        let result = mcp(
            &mut ai,
            "set_app_settings",
            json!({"theme":"light","size":"big","display_unit":"in","default_import_unit":"cm",
                   "sticky_selection":true,"resize_basis":"object","keep_proportions":true,
                   "ribbon_pinned":true,"grab_cursor":"hand"}),
        )
        .expect("set settings");

        for app in [&desktop, &ai] {
            assert_eq!(app.theme_mode, ThemeMode::Light);
            assert_eq!(app.ui_size, UiSize::Big);
            assert_eq!(app.commands.preferences().display_unit(), DisplayUnit::Inch);
            assert_eq!(
                app.commands.preferences().default_import_unit(),
                ImportUnit::Centimetre
            );
            assert!(app.sticky_selection && app.object_resize_basis && app.proportional_resize);
            assert!(app.ribbon_pinned && app.ribbon_expanded);
            assert_eq!(
                app.commands.preferences().grab_cursor(),
                GrabCursorStyle::Hand
            );
        }
        // What is saved to disk is the same.
        assert_eq!(saved_settings(&desktop), saved_settings(&ai));
        assert_eq!(result["theme"], json!("light"));
        assert_eq!(result["display_unit"], json!("in"));
        assert_eq!(result["resize_basis"], json!("object"));
        assert!(
            result["message"]
                .as_str()
                .is_some_and(|message| message.contains("theme"))
        );
        // app_settings reads back the same state; the same call again
        // changes nothing.
        let read = mcp(&mut ai, "app_settings", json!({})).expect("read");
        assert_eq!(read["size"], json!("big"));
        assert_eq!(read["default_import_unit"], json!("cm"));
        assert_eq!(read["grab_cursor"], json!("hand"));
        let again = mcp(&mut ai, "set_app_settings", json!({"theme":"light"})).expect("again");
        assert_eq!(again["message"], json!("No settings changed"));
        // A bad value is rejected before anything changes.
        assert!(matches!(
            mcp(
                &mut ai,
                "set_app_settings",
                json!({"theme":"dark","size":"huge"})
            ),
            Err(AdapterError::Validation(_))
        ));
        assert_eq!(ai.theme_mode, ThemeMode::Light);
        // No model change and no history entry.
        assert!(!ai.commands.is_dirty());

        // The Settings lighting sliders and the saved slicer.
        let lighting = modeler3d_viewport::ViewportLighting {
            intensity_percent: 120,
            specular_percent: 40,
            ..desktop.commands.preferences().viewport_lighting()
        };
        desktop.apply_viewport_lighting(lighting);
        desktop.apply_preferred_slicer(Some(PathBuf::from("C:/Slicers/slicer.exe")), None);
        desktop.apply_preferred_slicer(None, None);
        ai.apply_preferred_slicer(Some(PathBuf::from("C:/Slicers/slicer.exe")), None);
        let lit = mcp(
            &mut ai,
            "set_app_settings",
            json!({"lighting":{"intensity_percent":120,"specular_percent":40},"clear_slicer":true}),
        )
        .expect("lighting");
        assert_eq!(lit["lighting"]["intensity_percent"], json!(120));
        assert_eq!(lit["saved_slicer"], Value::Null);
        assert_eq!(
            desktop.commands.preferences().viewport_lighting(),
            ai.commands.preferences().viewport_lighting()
        );
        assert_eq!(saved_settings(&desktop), saved_settings(&ai));
        assert!(matches!(
            mcp(
                &mut ai,
                "set_app_settings",
                json!({"lighting":{"intensity_percent":900}})
            ),
            Err(AdapterError::Validation(_))
        ));
        assert!(matches!(
            mcp(
                &mut ai,
                "set_app_settings",
                json!({"slicer":"no-such-slicer"})
            ),
            Err(AdapterError::NotFound(_))
        ));
        let reset =
            mcp(&mut ai, "set_app_settings", json!({"reset_lighting":true})).expect("reset");
        assert_eq!(reset["lighting"]["intensity_percent"], json!(100));
        let _ = std::fs::remove_dir_all(desktop_dir);
        let _ = std::fs::remove_dir_all(ai_dir);
    }

    #[test]
    fn mcp_customize_ribbon_matches_the_ribbon_customizer() {
        let (mut desktop, desktop_dir) = app("ribbon-desktop");
        let (mut ai, ai_dir) = app("ribbon-mcp");
        for app in [&mut desktop, &mut ai] {
            app.commands
                .preferences_mut()
                .set_ribbon_favorites(default_favorites());
        }

        // Desktop: pin Mirror as a favorite, unpin Delete, and move Mirror to
        // the front of the Object tab in the customizer.
        desktop.set_favorite("Object:Mirror", true);
        desktop.set_favorite("Object:Delete", false);
        let mut order = desktop
            .ordered_tab_commands(RibbonTab::Object)
            .iter()
            .map(|command| command.label.to_owned())
            .collect::<Vec<_>>();
        order.retain(|label| label != "Mirror");
        order.insert(0, "Mirror".to_owned());
        desktop
            .commands
            .preferences_mut()
            .set_ribbon_order("Object", order);
        desktop.persist_preferences();

        let result = mcp(
            &mut ai,
            "customize_ribbon",
            json!({"add_favorite":"Object:Mirror","remove_favorite":"Object:Delete",
                   "tab":"Object","tab_order":["Mirror"]}),
        )
        .expect("customize");
        assert_eq!(
            desktop.commands.preferences().ribbon_favorites(),
            ai.commands.preferences().ribbon_favorites()
        );
        assert_eq!(
            desktop.ordered_tab_commands(RibbonTab::Object),
            ai.ordered_tab_commands(RibbonTab::Object)
        );
        let object_tab = result["ribbon"]["tabs"]
            .as_array()
            .and_then(|tabs| tabs.iter().find(|tab| tab["tab"] == json!("Object")))
            .cloned()
            .expect("object tab");
        assert_eq!(object_tab["commands"][0]["label"], json!("Mirror"));
        assert_eq!(object_tab["commands"][0]["favorite"], json!(true));

        // Unknown commands are rejected and change nothing.
        let before = ai.commands.preferences().ribbon_favorites().to_vec();
        assert!(matches!(
            mcp(
                &mut ai,
                "customize_ribbon",
                json!({"add_favorite":"Object:Teleport"})
            ),
            Err(AdapterError::Validation(_))
        ));
        assert!(matches!(
            mcp(&mut ai, "customize_ribbon", json!({"tab_order":["Mirror"]})),
            Err(AdapterError::Validation(_))
        ));
        assert_eq!(
            ai.commands.preferences().ribbon_favorites(),
            before.as_slice()
        );

        // Reset all: default favorites and every tab in default order.
        mcp(&mut ai, "customize_ribbon", json!({"reset":"all"})).expect("reset");
        assert_eq!(
            ai.commands.preferences().ribbon_favorites(),
            default_favorites().as_slice()
        );
        assert_eq!(
            ai.ordered_tab_commands(RibbonTab::Object),
            tab_commands(RibbonTab::Object)
        );
        let _ = std::fs::remove_dir_all(desktop_dir);
        let _ = std::fs::remove_dir_all(ai_dir);
    }

    #[test]
    fn mcp_show_screen_matches_the_menu_tabs_and_settings() {
        let (mut desktop, desktop_dir) = app("screen-desktop");
        let (mut ai, ai_dir) = app("screen-mcp");
        for app in [&mut desktop, &mut ai] {
            app.commands
                .dispatch(CommandId::AddPrimitive(PrimitiveKind::Cube))
                .expect("cube");
            app.surface = WorkspaceSurface::Scene;
        }
        // The menu item Start screen keeps the scene.
        desktop.show_start_screen();
        let result = mcp(&mut ai, "show_screen", json!({"screen":"start"})).expect("start");
        assert_eq!(result["screen"], json!("start"));
        for app in [&desktop, &ai] {
            assert_eq!(app.surface, WorkspaceSurface::Welcome);
            assert_eq!(app.commands.scene().objects().len(), 1);
        }
        // A tab click opens that tab; the row folds unless pinned.
        desktop.show_ribbon_tab(RibbonTab::Paint);
        mcp(
            &mut ai,
            "show_screen",
            json!({"screen":"scene","ribbon_tab":"Paint"}),
        )
        .expect("tab");
        assert_eq!(ai.active_tab, desktop.active_tab);
        assert_eq!(ai.surface, WorkspaceSurface::Scene);
        let folded = mcp(&mut ai, "show_screen", json!({"ribbon_open":false})).expect("fold");
        assert_eq!(folded["ribbon"]["open"], json!(false));
        ai.apply_ribbon_pinned(true);
        let pinned = mcp(&mut ai, "show_screen", json!({"ribbon_open":false})).expect("pinned");
        assert_eq!(pinned["ribbon"]["open"], json!(true));
        // Settings, then About inside it, as the Settings About button does.
        desktop.dispatch(CommandId::Settings);
        let settings = mcp(&mut ai, "show_screen", json!({"screen":"settings"})).expect("settings");
        assert_eq!(settings["open_window"], json!("settings"));
        assert_eq!(
            ai.dialog.as_ref().map(crate::dialogs::DialogState::kind),
            desktop
                .dialog
                .as_ref()
                .map(crate::dialogs::DialogState::kind)
        );
        let about = mcp(&mut ai, "show_screen", json!({"screen":"about"})).expect("about");
        assert_eq!(about["open_window"], json!("about"));
        assert_eq!(about["about"]["product"], json!("3D Modeler"));
        assert_eq!(about["about"]["version"], json!(env!("CARGO_PKG_VERSION")));
        // Going back to the start screen closes Settings, like a press outside it.
        let start = mcp(&mut ai, "show_screen", json!({"screen":"start"})).expect("start");
        assert_eq!(start["open_window"], Value::Null);
        assert!(ai.dialog.is_none());
        let _ = std::fs::remove_dir_all(desktop_dir);
        let _ = std::fs::remove_dir_all(ai_dir);
    }

    #[test]
    fn mcp_recent_files_and_open_recent_match_the_welcome_list() {
        let (mut ai, directory) = app("recent-mcp");
        let (mut desktop, desktop_dir) = app("recent-desktop");
        // A real project, a real model and a file that has gone away.
        let project = directory.join("bracket.3dp");
        let model = directory.join("part.stl");
        let missing = directory.join("gone.3dp");
        ai.commands
            .dispatch(CommandId::AddPrimitive(PrimitiveKind::Cube))
            .expect("cube");
        ai.commands
            .save_document_to_path(&project)
            .expect("save project");
        ai.commands.export_to_path(&model).expect("export model");
        for app in [&mut ai, &mut desktop] {
            app.commands.preferences_mut().recent_files_mut().clear();
            for path in [&missing, &model, &project] {
                app.commands
                    .preferences_mut()
                    .recent_files_mut()
                    .record(path);
            }
        }

        let listed = mcp(&mut ai, "recent_files", json!({})).expect("list");
        let kinds = listed["recent_files"]
            .as_array()
            .map(|entries| {
                entries
                    .iter()
                    .map(|entry| entry["kind"].clone())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        assert_eq!(
            kinds,
            vec![json!("project"), json!("model"), json!("missing")]
        );

        // A missing entry is removed, exactly as the Welcome list does.
        desktop.open_recent(&missing);
        let gone = mcp(&mut ai, "open_recent", json!({"index":2})).expect("missing");
        assert_eq!(gone["kind"], json!("missing"));
        assert_eq!(gone["opened"], json!(false));
        assert_eq!(
            desktop.commands.preferences().recent_files().paths(),
            ai.commands.preferences().recent_files().paths()
        );

        // A model is imported into the current scene, as on the desktop.
        desktop.open_recent(&model);
        let imported = mcp(&mut ai, "open_recent", json!({"index":1})).expect("model");
        assert_eq!(imported["kind"], json!("model"));
        assert_eq!(desktop.commands.scene().objects().len(), 1);
        assert_eq!(ai.commands.scene().objects().len(), 2);

        // With unsaved changes a project is refused (the desktop asks).
        assert!(ai.commands.is_dirty());
        assert!(matches!(
            mcp(
                &mut ai,
                "open_recent",
                json!({"path": project.display().to_string()})
            ),
            Err(AdapterError::Conflict(_))
        ));
        assert_eq!(ai.commands.scene().objects().len(), 2);
        desktop.open_recent(&project);
        assert!(matches!(
            desktop.confirm_action,
            Some(PendingReplacement::OpenProject(_))
        ));
        // Saved, it opens as the project.
        ai.commands.save_document_to_path(&project).expect("save");
        let opened = mcp(
            &mut ai,
            "open_recent",
            json!({"path": project.display().to_string()}),
        )
        .expect("project");
        assert_eq!(opened["kind"], json!("project"));
        assert_eq!(opened["opened"], json!(true));
        assert_eq!(ai.surface, WorkspaceSurface::Scene);
        // Only listed entries can be opened.
        assert!(matches!(
            mcp(&mut ai, "open_recent", json!({"path":"C:/elsewhere/x.3dp"})),
            Err(AdapterError::NotFound(_))
        ));

        // Remove one, then clear: the files stay on disk.
        let removed = mcp(
            &mut ai,
            "recent_files",
            json!({"action":"remove","index":0}),
        )
        .expect("remove");
        assert_eq!(removed["removed"].as_array().map(Vec::len), Some(1));
        let cleared = mcp(&mut ai, "recent_files", json!({"action":"clear"})).expect("clear");
        assert_eq!(cleared["recent_files"], json!([]));
        assert!(project.is_file() && model.is_file());
        assert!(ai.commands.preferences().recent_files().paths().is_empty());
        let _ = std::fs::remove_dir_all(directory);
        let _ = std::fs::remove_dir_all(desktop_dir);
    }

    #[test]
    fn mcp_exit_app_keeps_the_unsaved_check() {
        let (mut clean, clean_dir) = app("exit-clean");
        let done = mcp(&mut clean, "exit_app", json!({})).expect("clean exit");
        assert_eq!(done["exiting"], json!(true));
        assert!(clean.close_requested);

        let (mut dirty, dirty_dir) = app("exit-dirty");
        dirty
            .commands
            .dispatch(CommandId::AddPrimitive(PrimitiveKind::Cube))
            .expect("cube");
        assert!(matches!(
            mcp(&mut dirty, "exit_app", json!({})),
            Err(AdapterError::Conflict(_))
        ));
        assert!(!dirty.close_requested && dirty.confirm_action.is_none());
        // ask = the desktop Exit button: the save prompt for the user.
        let asked = mcp(&mut dirty, "exit_app", json!({"if_unsaved":"ask"})).expect("ask");
        assert_eq!(asked["asked_user"], json!(true));
        assert!(matches!(
            dirty.confirm_action,
            Some(PendingReplacement::Exit)
        ));
        assert!(!dirty.close_requested);
        dirty.confirm_action = None;
        let discarded =
            mcp(&mut dirty, "exit_app", json!({"if_unsaved":"discard"})).expect("discard");
        assert_eq!(discarded["exiting"], json!(true));
        assert!(dirty.close_requested);
        let _ = std::fs::remove_dir_all(clean_dir);
        let _ = std::fs::remove_dir_all(dirty_dir);
    }

    #[test]
    fn settings_note_is_read_and_dismissed_like_its_button() {
        const NOTE: &str = "Imported supported Python settings; the Rust JSON settings file will be the writable copy.";
        let (mut desktop, desktop_dir) = app("note-desktop");
        let (mut ai, ai_dir) = app("note-mcp");
        desktop.settings_warning = Some(NOTE.to_owned());
        ai.settings_warning = Some(NOTE.to_owned());

        let shown = mcp(&mut ai, "app_settings", json!({})).expect("app settings");
        assert_eq!(shown["settings_notice"], json!(NOTE));

        // Desktop: the note's Dismiss button.
        assert!(desktop.dismiss_settings_note());
        assert!(!desktop.dismiss_settings_note());

        let result = mcp(
            &mut ai,
            "set_app_settings",
            json!({"dismiss_settings_notice": true}),
        )
        .expect("dismiss");
        assert_eq!(result["settings_notice"], Value::Null);
        assert_eq!(result["message"], json!("Changed settings note dismissed"));
        assert_eq!(ai.settings_warning, desktop.settings_warning);

        let again = mcp(
            &mut ai,
            "set_app_settings",
            json!({"dismiss_settings_notice": true}),
        )
        .expect("dismiss again");
        assert_eq!(again["message"], json!("No settings changed"));
        let _ = std::fs::remove_dir_all(desktop_dir);
        let _ = std::fs::remove_dir_all(ai_dir);
    }
}
