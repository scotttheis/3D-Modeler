//! Typed, bounded shell dialogs.
//!
//! Dialogs collect user intent only.  The shell applies the returned action to
//! [`crate::commands::CommandState`], keeping file I/O and core mutations out
//! of the widget code.

use std::path::{Path, PathBuf};

use builder3d_core::{
    EmbossFont, HollowOptions, TORUS_RADIUS_MAX_MM, TORUS_RADIUS_MIN_MM, TORUS_TUBE_MIN_MM,
    TextEmbossOptions, TorusParameters,
};
use builder3d_platform::{SlicerInfo, slicer_info_from_path};
use eframe::egui::{self, Align2, Color32, Context, RichText};

use crate::commands::{CommandState, DialogKind, ImportUnit};
use crate::numeric_controls::parse_distance_mm;
use crate::preferences::DisplayUnit;
use crate::preferences::import_uses_authoritative_units;
use crate::print_handoff::{PreparedPrint, discard_prepared_print, discover_print_slicers};
use crate::print_prep::DEFAULT_BED_SIZE_MM;
use crate::theme::ThemeMode;

/// A result emitted by one dialog interaction.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum DialogAction {
    /// Close the current dialog.
    Close,
    /// Open a project path.
    Open(PathBuf),
    /// Import a mesh or scene path.
    Import(PathBuf, ImportUnit),
    /// Save a project path and whether the native picker confirmed replacement.
    Save(PathBuf, bool),
    /// Export a selected mesh path and whether the native picker confirmed replacement.
    Export(PathBuf, bool),
    /// Run selected-mesh repair.
    Repair,
    /// Run print preparation with a selected square bed size.
    PrintPrep(f32),
    /// Select the default unit for ambiguous imports.
    DefaultImportUnit(ImportUnit),
    /// Select the display unit for transform controls.
    DisplayUnit(DisplayUnit),
    /// Stage the current scene for one discovered slicer.
    PreparePrint(SlicerInfo, bool),
    /// Open the native executable picker from Settings.
    BrowsePreferredSlicer,
    /// Open the native executable picker from Print Prep.
    BrowsePrintSlicer,
    /// Remove the saved slicer preference.
    ClearPreferredSlicer,
    /// Clear recent paths without touching their files.
    ClearRecentFiles,
    /// Launch the currently staged print handoff.
    LaunchPrint,
    /// Apply an object color.
    Color([f32; 4]),
    /// Sample and apply an image's average color.
    ColorFromImage(PathBuf),
    /// Apply torus parameters.
    Torus(TorusParameters),
    /// Update the isolated torus render preview without creating history.
    TorusPreview(TorusParameters),
    /// Apply a bounded text relief.
    Emboss(TextEmbossOptions),
    /// Apply a bounded constant-thickness shell.
    Hollow(HollowOptions),
    /// Select a theme mode.
    Theme(ThemeMode),
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
enum MessageTone {
    #[default]
    Info,
    Warning,
    Error,
}

/// State retained while a typed dialog is open.
pub(crate) struct DialogState {
    kind: DialogKind,
    path_text: String,
    native_overwrite_confirmed: bool,
    import_unit: ImportUnit,
    display_unit: DisplayUnit,
    message: String,
    message_tone: MessageTone,
    color: [f32; 4],
    selected_object_count: usize,
    color_image_path: String,
    torus: TorusParameters,
    emboss_text: String,
    emboss_font: EmbossFont,
    emboss_font_size: u32,
    emboss_depth_mm: f32,
    emboss_raised: bool,
    emboss_offset_x_mm: f32,
    emboss_offset_z_mm: f32,
    hollow_wall_thickness_mm: f32,
    theme_mode: ThemeMode,
    print_bed_size: String,
    print_slicers: Vec<SlicerInfo>,
    selected_slicer_key: String,
    prepared_print: Option<PreparedPrint>,
    prepared_print_launched: bool,
    about_open: bool,
    preferred_slicer_path: Option<PathBuf>,
    preferred_slicer_label: Option<String>,
    recent_file_count: usize,
    remember_slicer: bool,
}

impl DialogState {
    /// Creates a bounded dialog state with safe defaults.
    #[must_use]
    pub(crate) fn new(kind: DialogKind, theme_mode: ThemeMode) -> Self {
        let path_text = match kind {
            DialogKind::OpenProject => String::from("Builder3D.3dp"),
            DialogKind::ImportModel => String::from("model.stl"),
            DialogKind::SaveProject => String::from("Builder3D.3dp"),
            DialogKind::Export => String::from("Builder3D.stl"),
            _ => String::new(),
        };
        let print_slicers = if kind == DialogKind::PrintPrep {
            discover_print_slicers()
        } else {
            Vec::new()
        };
        let selected_slicer_key = print_slicers
            .first()
            .map_or_else(String::new, |slicer| slicer.key.clone());
        Self {
            kind,
            path_text,
            native_overwrite_confirmed: false,
            import_unit: ImportUnit::Millimetre,
            display_unit: DisplayUnit::Millimetre,
            message: String::new(),
            message_tone: MessageTone::Info,
            color: [0.36, 0.44, 0.92, 1.0],
            selected_object_count: 0,
            color_image_path: String::new(),
            torus: TorusParameters::default(),
            emboss_text: String::from("Builder3D"),
            emboss_font: EmbossFont::Default,
            emboss_font_size: 10,
            emboss_depth_mm: 1.0,
            emboss_raised: true,
            emboss_offset_x_mm: 0.0,
            emboss_offset_z_mm: 0.0,
            hollow_wall_thickness_mm: 1.0,
            theme_mode,
            print_bed_size: format!("{DEFAULT_BED_SIZE_MM:.1} mm"),
            print_slicers,
            selected_slicer_key,
            prepared_print: None,
            prepared_print_launched: false,
            about_open: false,
            preferred_slicer_path: None,
            preferred_slicer_label: None,
            recent_file_count: 0,
            remember_slicer: kind == DialogKind::PrintPrep,
        }
    }

    /// Updates the inline result/error message without closing the dialog.
    pub(crate) fn set_message(&mut self, message: impl Into<String>) {
        self.message = message.into();
        self.message_tone = MessageTone::Info;
    }

    /// Sets a visible warning or failure in the current native dialog.
    pub(crate) fn set_error(&mut self, message: impl Into<String>) {
        self.message = message.into();
        self.message_tone = MessageTone::Error;
    }

    /// Sets a print report using a tone that reflects its blocking findings.
    pub(crate) fn set_report(
        &mut self,
        message: impl Into<String>,
        has_errors: bool,
        has_warnings: bool,
    ) {
        self.message = message.into();
        self.message_tone = if has_errors {
            MessageTone::Error
        } else if has_warnings {
            MessageTone::Warning
        } else {
            MessageTone::Info
        };
    }

    /// Applies the persisted default import unit to a newly opened dialog.
    pub(crate) fn set_import_unit(&mut self, unit: ImportUnit) {
        self.import_unit = unit;
    }

    /// Applies the persisted display unit to settings-aware dialog state.
    pub(crate) fn set_display_unit(&mut self, unit: DisplayUnit) {
        self.display_unit = unit;
    }

    /// Supplies the stored slicer path and adds it to the print chooser only
    /// after the platform boundary confirms it is still a file.
    pub(crate) fn set_preferred_slicer(&mut self, path: Option<PathBuf>, label: Option<String>) {
        self.preferred_slicer_path = path.clone();
        self.preferred_slicer_label = label.clone();
        if self.kind != DialogKind::PrintPrep {
            return;
        }
        let Some(path) = path else { return };
        match slicer_info_from_path(&path, label.as_deref()) {
            Ok(slicer) => {
                if !self
                    .print_slicers
                    .iter()
                    .any(|candidate| candidate.path == slicer.path)
                {
                    self.print_slicers.push(slicer.clone());
                }
                self.selected_slicer_key = slicer.key;
                self.remember_slicer = true;
            }
            Err(_) => {
                self.message = format!(
                    "Saved slicer is unavailable: {}. Browse for a valid executable or clear the preference.",
                    path.display()
                );
                self.message_tone = MessageTone::Warning;
            }
        }
    }

    /// Updates the count displayed beside the non-destructive recent-list action.
    pub(crate) fn set_recent_file_count(&mut self, count: usize) {
        self.recent_file_count = count;
    }

    /// Installs a user-selected custom slicer in the active print dialog.
    pub(crate) fn add_print_slicer(&mut self, slicer: SlicerInfo) {
        if !self
            .print_slicers
            .iter()
            .any(|candidate| candidate.path == slicer.path)
        {
            self.print_slicers.push(slicer.clone());
        }
        self.selected_slicer_key = slicer.key;
        self.message.clear();
        self.message_tone = MessageTone::Info;
        if let Some(previous) = self.prepared_print.take() {
            let _ = discard_prepared_print(&previous);
        }
    }

    /// Returns the selected print destination and the explicit remember choice.
    pub(crate) fn selected_print_slicer(&self) -> Option<(SlicerInfo, bool)> {
        self.print_slicers
            .iter()
            .find(|slicer| slicer.key == self.selected_slicer_key)
            .cloned()
            .map(|slicer| (slicer, self.remember_slicer))
    }

    /// Supplies a successful staged print handoff for the launch button.
    pub(crate) fn set_prepared_print(&mut self, prepared: PreparedPrint) {
        if let Some(previous) = self.prepared_print.take() {
            let _ = discard_prepared_print(&previous);
        }
        self.prepared_print = Some(prepared);
        self.prepared_print_launched = false;
    }

    /// Marks staged files as handed to the slicer so closing the dialog keeps
    /// them available while the slicer imports the files.
    pub(crate) fn mark_print_launched(&mut self) {
        self.prepared_print_launched = true;
    }

    /// Removes staged files and clears the launch action after stale input.
    pub(crate) fn discard_prepared_print(&mut self) -> Result<(), std::io::Error> {
        self.prepared_print_launched = false;
        if let Some(prepared) = self.prepared_print.take() {
            discard_prepared_print(&prepared)?;
        }
        Ok(())
    }

    /// Seeds the authored color editor from the first selected object.
    pub(crate) fn seed_material_color(&mut self, state: &CommandState) {
        self.selected_object_count = state.selected_count();
        if let Some(index) = state.scene().selection().iter().next()
            && let Ok(object) = state.scene().object(*index)
        {
            self.color = object.color();
        }
    }

    /// Returns the current staged print handoff, if any.
    pub(crate) fn prepared_print(&self) -> Option<&PreparedPrint> {
        self.prepared_print.as_ref()
    }

    #[cfg(test)]
    pub(crate) const fn kind(&self) -> DialogKind {
        self.kind
    }

    #[cfg(test)]
    pub(crate) fn message(&self) -> &str {
        &self.message
    }

    #[cfg(test)]
    pub(crate) const fn has_error(&self) -> bool {
        matches!(self.message_tone, MessageTone::Error)
    }

    /// Seeds the torus editor from the selected object when available.
    pub(crate) fn load_torus_from_state(&mut self, state: &CommandState) {
        if let Ok(parameters) = state.selected_torus_parameters() {
            self.torus = parameters;
        }
    }

    /// Draws the dialog and returns at most one typed action per frame.
    pub(crate) fn show(&mut self, ctx: &Context) -> Option<DialogAction> {
        let mut action = None;
        let title = self.title();
        egui::Window::new(title)
            .collapsible(false)
            .resizable(false)
            .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.set_min_width(360.0);
                match self.kind {
                    DialogKind::OpenProject => self.path_dialog(ui, "Open", &mut action),
                    DialogKind::ImportModel => self.import_dialog(ui, &mut action),
                    DialogKind::SaveProject => self.path_dialog(ui, "Save", &mut action),
                    DialogKind::Export => self.path_dialog(ui, "Export", &mut action),
                    DialogKind::Settings => {
                        let max_height = (ctx.content_rect().height() - 160.0).max(280.0);
                        egui::ScrollArea::vertical()
                            .max_height(max_height)
                            .show(ui, |ui| self.settings_dialog(ui, &mut action));
                    }
                    DialogKind::Repair => self.repair_dialog(ui, &mut action),
                    DialogKind::PrintPrep => self.print_prep_dialog(ui, &mut action),
                    DialogKind::Material => self.material_dialog(ui, &mut action),
                    DialogKind::Torus => self.torus_dialog(ui, &mut action),
                    DialogKind::Emboss => self.emboss_dialog(ui, &mut action),
                    DialogKind::Hollow => self.hollow_dialog(ui, &mut action),
                }
                if !self.message.is_empty() {
                    ui.separator();
                    let color = match self.message_tone {
                        MessageTone::Info => Color32::from_rgb(0x40, 0x52, 0xB5),
                        MessageTone::Warning => Color32::from_rgb(0x9A, 0x62, 0x00),
                        MessageTone::Error => Color32::from_rgb(0xB4, 0x23, 0x18),
                    };
                    egui::ScrollArea::vertical()
                        .max_height(180.0)
                        .show(ui, |ui| ui.label(RichText::new(&self.message).color(color)));
                }
                let close_label = if self.kind == DialogKind::Settings {
                    "Close"
                } else {
                    "Cancel"
                };
                if ui.button(close_label).clicked() {
                    action = Some(DialogAction::Close);
                }
            });
        if self.about_open {
            let mut close_about = false;
            egui::Modal::new(egui::Id::unique("builder3d-settings-about")).show(ctx, |ui| {
                ui.heading("About Builder3D");
                ui.label(
                    "A reverse-engineering-informed 3D model repair and building application.",
                );
                if ui.button("Close About").clicked() {
                    close_about = true;
                }
            });
            if close_about {
                self.about_open = false;
            }
        }
        if ctx.input(|input| input.key_pressed(egui::Key::Escape)) {
            if self.about_open {
                self.about_open = false;
            } else {
                action = Some(DialogAction::Close);
            }
        }
        action
    }

    fn path_dialog(
        &mut self,
        ui: &mut egui::Ui,
        action_label: &str,
        action: &mut Option<DialogAction>,
    ) {
        ui.label(match self.kind {
            DialogKind::OpenProject => "Builder3D project (.3dp)",
            DialogKind::SaveProject => "Save project (.3dp)",
            DialogKind::Export => "Export model",
            _ => "Path",
        });
        ui.horizontal(|ui| {
            let path_changed = ui.text_edit_singleline(&mut self.path_text).changed();
            if path_changed {
                self.native_overwrite_confirmed = false;
            }
            if ui.button("Browse...").clicked() {
                let selected = native_path_picker(self.kind, &self.path_text);
                self.path_text = path_after_browse(&self.path_text, selected.as_deref());
                if selected.is_some() {
                    self.native_overwrite_confirmed =
                        selected.as_ref().is_some_and(|path| path.is_file());
                    self.message.clear();
                }
            }
        });
        ui.label(RichText::new("The path is validated before the core file operation.").weak());
        if ui.button(action_label).clicked() {
            let path = PathBuf::from(self.path_text.trim());
            *action = Some(match self.kind {
                DialogKind::OpenProject => DialogAction::Open(path),
                DialogKind::SaveProject => {
                    DialogAction::Save(path, self.native_overwrite_confirmed)
                }
                DialogKind::Export => DialogAction::Export(
                    export_path_with_default_extension(path),
                    self.native_overwrite_confirmed,
                ),
                _ => return,
            });
        }
    }

    fn import_dialog(&mut self, ui: &mut egui::Ui, action: &mut Option<DialogAction>) {
        ui.label("Path");
        ui.horizontal(|ui| {
            ui.text_edit_singleline(&mut self.path_text);
            if ui.button("Browse...").clicked() {
                let selected = rfd::FileDialog::new()
                    .set_title("Import Builder3D model")
                    .add_filter(
                        "Builder3D models",
                        &["stl", "obj", "ply", "3mf", "gltf", "glb"],
                    )
                    .add_filter("STL mesh", &["stl"])
                    .add_filter("OBJ mesh", &["obj"])
                    .add_filter("PLY mesh", &["ply"])
                    .add_filter("3D Manufacturing Format", &["3mf"])
                    .add_filter("glTF", &["gltf", "glb"])
                    .pick_file();
                self.path_text = path_after_browse(&self.path_text, selected.as_deref());
                if selected.is_some() {
                    self.message.clear();
                }
            }
        });
        ui.label(
            RichText::new(
                if import_uses_authoritative_units(Path::new(self.path_text.trim())) {
                    "This format carries its own units; the selector is ignored."
                } else {
                    "Unit applies to STL, OBJ, and PLY; 3MF, glTF, and GLB carry their own units."
                },
            )
            .weak(),
        );
        egui::ComboBox::from_label("Unit")
            .selected_text(self.import_unit.label())
            .show_ui(ui, |ui| {
                for unit in ImportUnit::all() {
                    ui.selectable_value(&mut self.import_unit, unit, unit.label());
                }
            });
        if ui.button("Import").clicked() {
            let path = self.path_text.trim();
            if path.is_empty() {
                self.message = String::from("Enter an import path.");
            } else {
                *action = Some(DialogAction::Import(PathBuf::from(path), self.import_unit));
            }
        }
    }

    fn settings_dialog(&mut self, ui: &mut egui::Ui, action: &mut Option<DialogAction>) {
        ui.label("Theme");
        ui.horizontal(|ui| {
            if ui
                .selectable_label(self.theme_mode == ThemeMode::Light, "Light")
                .clicked()
            {
                self.theme_mode = ThemeMode::Light;
                *action = Some(DialogAction::Theme(ThemeMode::Light));
            }
            if ui
                .selectable_label(self.theme_mode == ThemeMode::Dark, "Dark")
                .clicked()
            {
                self.theme_mode = ThemeMode::Dark;
                *action = Some(DialogAction::Theme(ThemeMode::Dark));
            }
        });
        ui.label(RichText::new("Theme changes apply immediately to shell surfaces.").weak());
        ui.separator();
        ui.label("Default units for ambiguous imports");
        let import_unit_combo = egui::ComboBox::from_id_salt("settings-import-unit")
            .selected_text(self.import_unit.label())
            .show_ui(ui, |ui| {
                for unit in ImportUnit::all() {
                    if ui
                        .selectable_value(&mut self.import_unit, unit, unit.label())
                        .changed()
                    {
                        *action = Some(DialogAction::DefaultImportUnit(unit));
                    }
                }
            });
        import_unit_combo.response.widget_info(|| {
            egui::WidgetInfo::labeled(egui::Role::ComboBox, true, "Default ambiguous import unit")
        });
        ui.label(RichText::new("Used when a mesh file does not declare physical units.").weak());
        ui.label("Transform display units");
        let display_unit_combo = egui::ComboBox::from_id_salt("settings-display-unit")
            .selected_text(self.display_unit.label())
            .show_ui(ui, |ui| {
                for unit in [DisplayUnit::Millimetre, DisplayUnit::Inch] {
                    if ui
                        .selectable_value(&mut self.display_unit, unit, unit.label())
                        .changed()
                    {
                        *action = Some(DialogAction::DisplayUnit(unit));
                    }
                }
            });
        display_unit_combo.response.widget_info(|| {
            egui::WidgetInfo::labeled(egui::Role::ComboBox, true, "Transform display unit")
        });
        ui.separator();
        ui.label("Preferred 3D slicer");
        match &self.preferred_slicer_path {
            Some(path) if path.is_file() => {
                ui.label(
                    self.preferred_slicer_label
                        .as_deref()
                        .unwrap_or("Selected slicer"),
                );
                ui.label(RichText::new(path.display().to_string()).small().weak());
            }
            Some(path) => {
                ui.label(
                    RichText::new("Saved slicer is unavailable")
                        .color(Color32::from_rgb(0x9A, 0x62, 0x00)),
                );
                ui.label(RichText::new(path.display().to_string()).small().weak());
            }
            None => {
                ui.label(
                    RichText::new("No slicer selected. The print workflow can search for one.")
                        .weak(),
                );
            }
        }
        ui.horizontal(|ui| {
            if ui.button("Change Slicer...").clicked() {
                *action = Some(DialogAction::BrowsePreferredSlicer);
            }
            if ui
                .add_enabled(
                    self.preferred_slicer_path.is_some(),
                    egui::Button::new("Clear Preference"),
                )
                .clicked()
            {
                *action = Some(DialogAction::ClearPreferredSlicer);
            }
        });
        ui.separator();
        ui.label("Recent files");
        if ui
            .add_enabled(
                self.recent_file_count > 0,
                egui::Button::new("Clear Recent Files List"),
            )
            .clicked()
        {
            *action = Some(DialogAction::ClearRecentFiles);
        }
        if self.recent_file_count == 0 {
            ui.label(RichText::new("The recent-files list is already empty.").weak());
        } else {
            ui.label(
                RichText::new(format!(
                    "{} recent path(s); source files are not deleted.",
                    self.recent_file_count
                ))
                .weak(),
            );
        }
        ui.separator();
        ui.label("Application");
        if ui.button("About").clicked() {
            self.about_open = true;
        }
        if ui.button("Done").clicked() {
            *action = Some(DialogAction::Close);
        }
    }

    fn repair_dialog(&mut self, ui: &mut egui::Ui, action: &mut Option<DialogAction>) {
        ui.label("Repair validates and applies a bounded best-effort mesh candidate.");
        if ui.button("Run repair").clicked() {
            *action = Some(DialogAction::Repair);
        }
    }

    fn print_prep_dialog(&mut self, ui: &mut egui::Ui, action: &mut Option<DialogAction>) {
        ui.label("Run bounded scene checks before handing geometry to a slicer.");
        ui.horizontal(|ui| {
            ui.label("Square bed");
            let bed_size_changed = ui
                .add_sized(
                    [120.0, 22.0],
                    egui::TextEdit::singleline(&mut self.print_bed_size)
                        .hint_text("Square bed size in millimetres"),
                )
                .changed();
            if bed_size_changed {
                self.message.clear();
                self.message_tone = MessageTone::Info;
            }
            if ui.button("Run print prep check").clicked() {
                match parse_distance_mm(&self.print_bed_size, DisplayUnit::Millimetre) {
                    Ok(value) if value > 0.0 => *action = Some(DialogAction::PrintPrep(value)),
                    Ok(_) => self.set_error("Bed size must be greater than zero."),
                    Err(error) => self.set_error(format!("Bed size: {error}")),
                }
            }
        });
        ui.separator();
        ui.label("Known slicer handoff");
        if self.print_slicers.is_empty() {
            ui.label(
                RichText::new("No supported slicer was discovered on this machine.")
                    .color(Color32::from_rgb(0x9A, 0x62, 0x00)),
            );
        } else {
            let mut slicer_changed = false;
            ui.add_enabled_ui(!self.prepared_print_launched, |ui| {
                let slicer_combo = egui::ComboBox::from_id_salt("print-slicer")
                    .selected_text(
                        self.print_slicers
                            .iter()
                            .find(|slicer| slicer.key == self.selected_slicer_key)
                            .map_or("Choose slicer", |slicer| slicer.label.as_str()),
                    )
                    .show_ui(ui, |ui| {
                        for slicer in &self.print_slicers {
                            slicer_changed |= ui
                                .selectable_value(
                                    &mut self.selected_slicer_key,
                                    slicer.key.clone(),
                                    format!(
                                        "{} ({})",
                                        slicer.label,
                                        if slicer.stl_fallback() { "STL" } else { "3MF" }
                                    ),
                                )
                                .changed();
                        }
                    });
                slicer_combo.response.widget_info(|| {
                    egui::WidgetInfo::labeled(egui::Role::ComboBox, true, "Print slicer")
                });
            });
            if slicer_changed {
                if let Some(prepared) = self.prepared_print.take()
                    && let Err(error) = discard_prepared_print(&prepared)
                {
                    self.set_error(format!("Could not discard previous staging: {error}"));
                } else {
                    self.message.clear();
                    self.message_tone = MessageTone::Info;
                }
            }
        }
        ui.horizontal(|ui| {
            if ui.button("Browse for slicer...").clicked() {
                *action = Some(DialogAction::BrowsePrintSlicer);
            }
            if !self.print_slicers.is_empty() {
                ui.checkbox(&mut self.remember_slicer, "Remember this slicer");
            }
        });
        if !self.print_slicers.is_empty()
            && self.prepared_print.is_none()
            && ui.button("Prepare visible objects").clicked()
        {
            if let Some((slicer, remember)) = self.selected_print_slicer() {
                *action = Some(DialogAction::PreparePrint(slicer, remember));
            } else {
                self.set_error("Choose a slicer before preparing the visible objects.");
            }
        }
        if let Some(prepared) = &self.prepared_print {
            ui.separator();
            ui.label(format!(
                "Prepared {} object(s) as {} for {}.",
                prepared.objects_exported, prepared.format, prepared.slicer.label
            ));
            ui.label(
                RichText::new(prepared.staging_dir.display().to_string())
                    .small()
                    .weak(),
            );
            if ui
                .add_enabled(
                    !self.prepared_print_launched,
                    egui::Button::new("Launch slicer"),
                )
                .clicked()
            {
                *action = Some(DialogAction::LaunchPrint);
            }
        }
    }

    fn material_dialog(&mut self, ui: &mut egui::Ui, action: &mut Option<DialogAction>) {
        ui.label(format!(
            "Selected object(s) color (RGBA, {})",
            self.selected_object_count
        ));
        ui.horizontal(|ui| {
            ui.color_edit_button_rgba_unmultiplied(&mut self.color);
            ui.label("Choose a solid color");
        });
        for (label, index) in [("R", 0_usize), ("G", 1), ("B", 2), ("A", 3)] {
            ui.add(egui::Slider::new(&mut self.color[index], 0.0..=1.0).text(label));
        }
        if ui
            .button(format!(
                "Apply color to {} selected object(s)",
                self.selected_object_count
            ))
            .clicked()
        {
            *action = Some(DialogAction::Color(self.color));
        }
        ui.separator();
        ui.label(
            "Sample an image's average color as one solid color; this does not create a texture.",
        );
        ui.horizontal(|ui| {
            if ui
                .text_edit_singleline(&mut self.color_image_path)
                .changed()
            {
                self.message.clear();
                self.message_tone = MessageTone::Info;
            }
            if ui.button("Browse image...").clicked() {
                let selected = image_path_picker(&self.color_image_path);
                self.color_image_path =
                    path_after_browse(&self.color_image_path, selected.as_deref());
                if selected.is_some() {
                    self.message.clear();
                    self.message_tone = MessageTone::Info;
                } else {
                    self.set_message("Image selection cancelled; the current path was kept.");
                }
            }
        });
        if ui.button("Sample image color").clicked() {
            let path = self.color_image_path.trim();
            if path.is_empty() {
                self.set_error("Enter an image path.");
            } else {
                *action = Some(DialogAction::ColorFromImage(PathBuf::from(path)));
            }
        }
    }

    fn torus_dialog(&mut self, ui: &mut egui::Ui, action: &mut Option<DialogAction>) {
        ui.label("Bounded torus parameters (millimetres)");
        let mut mode_changed = false;
        ui.horizontal(|ui| {
            ui.label("Mode");
            if ui
                .selectable_label(self.torus.mode == "solid", "Solid")
                .clicked()
            {
                self.torus.mode = String::from("solid");
                mode_changed = true;
            }
            if ui
                .selectable_label(self.torus.mode == "hole", "Hole")
                .clicked()
            {
                self.torus.mode = String::from("hole");
                mode_changed = true;
            }
        });
        let radius_changed = ui
            .add(
                egui::Slider::new(
                    &mut self.torus.radius_mm,
                    TORUS_RADIUS_MIN_MM..=TORUS_RADIUS_MAX_MM,
                )
                .text("Radius"),
            )
            .changed();
        let maximum_tube = (self.torus.radius_mm - 0.01).max(TORUS_TUBE_MIN_MM);
        if self.torus.tube_mm > maximum_tube {
            self.torus.tube_mm = maximum_tube;
        }
        let tube_changed = ui
            .add(
                egui::Slider::new(&mut self.torus.tube_mm, TORUS_TUBE_MIN_MM..=maximum_tube)
                    .text("Tube"),
            )
            .changed();
        let mut sides = self.torus.sides as u32;
        let mut steps = self.torus.steps as u32;
        let sides_changed = ui
            .add(egui::Slider::new(&mut sides, 3..=256).text("Sides"))
            .changed();
        let steps_changed = ui
            .add(egui::Slider::new(&mut steps, 3..=256).text("Steps"))
            .changed();
        self.torus.sides = sides as usize;
        self.torus.steps = steps as usize;
        let changed =
            radius_changed || tube_changed || sides_changed || steps_changed || mode_changed;
        if changed {
            *action = Some(DialogAction::TorusPreview(self.torus.clone()));
        }
        if ui.button("Apply torus").clicked() {
            *action = Some(DialogAction::Torus(self.torus.clone()));
        }
    }

    fn emboss_dialog(&mut self, ui: &mut egui::Ui, action: &mut Option<DialogAction>) {
        ui.label("Apply bounded text relief to the selected solid.");
        ui.label("Text");
        ui.text_edit_singleline(&mut self.emboss_text);
        egui::ComboBox::from_label("Font")
            .selected_text(self.emboss_font.as_str())
            .show_ui(ui, |ui| {
                for font in [
                    EmbossFont::Default,
                    EmbossFont::Sans,
                    EmbossFont::Serif,
                    EmbossFont::Mono,
                ] {
                    ui.selectable_value(&mut self.emboss_font, font, font.as_str());
                }
            });
        ui.add(
            egui::DragValue::new(&mut self.emboss_font_size)
                .range(1..=200)
                .prefix("Size "),
        );
        ui.add(
            egui::DragValue::new(&mut self.emboss_depth_mm)
                .range(0.000_001..=50.0)
                .speed(0.1)
                .suffix(" mm depth"),
        );
        ui.checkbox(&mut self.emboss_raised, "Raised (clear for engraved)");
        ui.add(
            egui::DragValue::new(&mut self.emboss_offset_x_mm)
                .range(-10_000.0..=10_000.0)
                .speed(0.1)
                .suffix(" mm X offset"),
        );
        ui.add(
            egui::DragValue::new(&mut self.emboss_offset_z_mm)
                .range(-10_000.0..=10_000.0)
                .speed(0.1)
                .suffix(" mm Z offset"),
        );
        if ui.button("Apply text emboss").clicked() {
            *action = Some(DialogAction::Emboss(TextEmbossOptions {
                text: self.emboss_text.clone(),
                font_size: self.emboss_font_size,
                depth_mm: self.emboss_depth_mm,
                raised: self.emboss_raised,
                font: self.emboss_font,
                offset_x_mm: self.emboss_offset_x_mm,
                offset_z_mm: self.emboss_offset_z_mm,
                limits: Default::default(),
            }));
        }
    }

    fn hollow_dialog(&mut self, ui: &mut egui::Ui, action: &mut Option<DialogAction>) {
        ui.label("Create a bounded constant-thickness shell from the selected solid.");
        ui.add(
            egui::DragValue::new(&mut self.hollow_wall_thickness_mm)
                .range(0.000_001..=50.0)
                .speed(0.1)
                .suffix(" mm wall"),
        );
        if ui.button("Apply hollow").clicked() {
            *action = Some(DialogAction::Hollow(HollowOptions {
                wall_thickness_mm: self.hollow_wall_thickness_mm,
                ..HollowOptions::default()
            }));
        }
    }

    fn title(&self) -> &'static str {
        match self.kind {
            DialogKind::OpenProject => "Open Project",
            DialogKind::ImportModel => "Import Model",
            DialogKind::SaveProject => "Save Project",
            DialogKind::Export => "Export",
            DialogKind::Settings => "Settings",
            DialogKind::Repair => "Repair",
            DialogKind::PrintPrep => "Print Prep Check",
            DialogKind::Material => "Material",
            DialogKind::Torus => "Edit Torus",
            DialogKind::Emboss => "Emboss",
            DialogKind::Hollow => "Hollow",
        }
    }
}

impl Drop for DialogState {
    fn drop(&mut self) {
        if !self.prepared_print_launched
            && let Some(prepared) = self.prepared_print.take()
        {
            let _ = discard_prepared_print(&prepared);
        }
    }
}

fn path_after_browse(current: &str, selected: Option<&Path>) -> String {
    selected.map_or_else(
        || current.to_owned(),
        |path| path.to_string_lossy().into_owned(),
    )
}

fn export_path_with_default_extension(path: PathBuf) -> PathBuf {
    if path
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| !extension.is_empty())
    {
        path
    } else {
        path.with_extension("stl")
    }
}

fn native_path_picker(kind: DialogKind, current: &str) -> Option<PathBuf> {
    let current_path = Path::new(current.trim());
    let mut dialog = rfd::FileDialog::new();
    if let Some(parent) = current_path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        dialog = dialog.set_directory(parent);
    }
    if let Some(name) = current_path.file_name().and_then(|name| name.to_str()) {
        dialog = dialog.set_file_name(name);
    }

    match kind {
        DialogKind::OpenProject => dialog
            .set_title("Open Builder3D project")
            .add_filter("Builder3D project", &["3dp"])
            .pick_file(),
        DialogKind::SaveProject => dialog
            .set_title("Save Builder3D project")
            .add_filter("Builder3D project", &["3dp"])
            .save_file(),
        DialogKind::Export => dialog
            .set_title("Export Builder3D model")
            .add_filter(
                "Builder3D models",
                &["stl", "obj", "ply", "3mf", "gltf", "glb", "wrl"],
            )
            .add_filter("3MF model", &["3mf"])
            .add_filter("glTF scene", &["gltf"])
            .add_filter("GLB scene", &["glb"])
            .add_filter("VRML scene", &["wrl"])
            .add_filter("STL mesh", &["stl"])
            .add_filter("OBJ mesh", &["obj"])
            .add_filter("PLY mesh", &["ply"])
            .save_file(),
        _ => None,
    }
}

fn image_path_picker(current: &str) -> Option<PathBuf> {
    let current_path = Path::new(current.trim());
    let mut dialog = rfd::FileDialog::new().set_title("Open Image for Average Color");
    if let Some(parent) = current_path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        dialog = dialog.set_directory(parent);
    }
    if let Some(name) = current_path.file_name().and_then(|name| name.to_str()) {
        dialog = dialog.set_file_name(name);
    }
    dialog
        .add_filter("Supported images", &["png", "jpg", "jpeg", "bmp", "tga"])
        .pick_file()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dialog_defaults_are_bounded_and_typed() {
        let state = DialogState::new(DialogKind::SaveProject, ThemeMode::Light);
        assert_eq!(state.path_text, "Builder3D.3dp");
        assert!(state.torus.radius_mm > state.torus.tube_mm);
        assert_eq!(state.theme_mode, ThemeMode::Light);

        let import = DialogState::new(DialogKind::ImportModel, ThemeMode::Light);
        assert_eq!(import.path_text, "model.stl");
    }

    #[test]
    fn export_path_keeps_explicit_format_and_defaults_extensionless_names_to_stl() {
        assert_eq!(
            export_path_with_default_extension(PathBuf::from("model")),
            PathBuf::from("model.stl")
        );
        assert_eq!(
            export_path_with_default_extension(PathBuf::from("model.")),
            PathBuf::from("model.stl")
        );
        assert_eq!(
            export_path_with_default_extension(PathBuf::from("model.3MF")),
            PathBuf::from("model.3MF")
        );
    }

    #[test]
    fn cancelling_native_import_browse_preserves_the_typed_path() {
        let current = String::from("existing-model.stl");
        assert_eq!(path_after_browse(&current, None), current);
    }

    #[test]
    fn native_file_picker_selection_updates_path_without_changing_cancellation() {
        let current = String::from("existing-project.3dp");
        assert_eq!(path_after_browse(&current, None), current);
        assert_eq!(
            path_after_browse(&current, Some(Path::new("chosen/project.3dp"))),
            "chosen/project.3dp"
        );
    }

    #[test]
    fn cancelling_image_browse_keeps_the_typed_path_for_retry() {
        let current = String::from("C:/images/sample.png");
        assert_eq!(path_after_browse(&current, None), current);
    }

    #[test]
    fn escape_closes_about_before_closing_settings() {
        let context = Context::default();
        let mut dialog = DialogState::new(DialogKind::Settings, ThemeMode::Dark);
        dialog.about_open = true;
        let input = egui::RawInput {
            events: vec![egui::Event::Key {
                key: egui::Key::Escape,
                physical_key: Some(egui::Key::Escape),
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::default(),
            }],
            ..Default::default()
        };
        let mut action = None;
        let mut output = context.run_ui(input, |ui| {
            action = dialog.show(ui.ctx());
        });
        output.textures_delta.clear();
        assert!(!dialog.about_open);
        assert_eq!(action, None);

        let second_escape = egui::RawInput {
            events: vec![egui::Event::Key {
                key: egui::Key::Escape,
                physical_key: Some(egui::Key::Escape),
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::default(),
            }],
            ..Default::default()
        };
        let mut output = context.run_ui(second_escape, |ui| {
            action = dialog.show(ui.ctx());
        });
        output.textures_delta.clear();
        assert_eq!(action, Some(DialogAction::Close));
    }

    #[test]
    fn material_dialog_is_seeded_from_the_selected_object_and_counts_selection() {
        let mut state = CommandState::default();
        assert!(
            state
                .dispatch(crate::commands::CommandId::AddPrimitive(
                    builder3d_core::PrimitiveKind::Cube
                ))
                .is_ok()
        );
        assert!(
            state
                .dispatch(crate::commands::CommandId::AddPrimitive(
                    builder3d_core::PrimitiveKind::Sphere
                ))
                .is_ok()
        );
        state.set_selection_indices([0, 1]);
        let color = [0.2, 0.3, 0.4, 0.5];
        assert!(state.set_selected_color(color).is_ok());
        let mut dialog = DialogState::new(DialogKind::Material, ThemeMode::Light);

        dialog.seed_material_color(&state);

        assert_eq!(dialog.color, color);
        assert_eq!(dialog.selected_object_count, 2);
    }
}
