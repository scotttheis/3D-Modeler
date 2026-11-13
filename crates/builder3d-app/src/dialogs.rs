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
use builder3d_viewport::ViewportLighting;
use eframe::egui::{self, Align2, Color32, Context, RichText};

use crate::commands::{CommandState, DialogKind, ImportUnit};
use crate::numeric_controls::parse_distance_mm;
use crate::preferences::DisplayUnit;
use crate::preferences::import_uses_authoritative_units;
use crate::print_handoff::{PreparedPrint, discard_prepared_print, discover_print_slicers};
use crate::print_prep::DEFAULT_BED_SIZE_MM;
use crate::theme::{ThemeMode, UiSize};

/// A file type offered by Save (STL, 3MF) or Export (every writer).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum FileType {
    Stl,
    ThreeMf,
    Obj,
    Ply,
    Gltf,
    Glb,
    Wrl,
    Project,
}

impl FileType {
    /// Save's quick-pick types; STL is the default (OF-23/OF-24).
    const SAVE: [Self; 2] = [Self::Stl, Self::ThreeMf];
    /// Export's drop-down, in menu order (OF-25).
    const EXPORT: [Self; 8] = [
        Self::Stl,
        Self::ThreeMf,
        Self::Obj,
        Self::Ply,
        Self::Gltf,
        Self::Glb,
        Self::Wrl,
        Self::Project,
    ];

    pub(crate) const fn extension(self) -> &'static str {
        match self {
            Self::Stl => "stl",
            Self::ThreeMf => "3mf",
            Self::Obj => "obj",
            Self::Ply => "ply",
            Self::Gltf => "gltf",
            Self::Glb => "glb",
            Self::Wrl => "wrl",
            Self::Project => "3dp",
        }
    }

    const fn short_label(self) -> &'static str {
        match self {
            Self::Stl => "STL",
            Self::ThreeMf => "3MF",
            Self::Obj => "OBJ",
            Self::Ply => "PLY",
            Self::Gltf => "glTF",
            Self::Glb => "GLB",
            Self::Wrl => "VRML",
            Self::Project => "Builder3D project",
        }
    }

    const fn label(self) -> &'static str {
        match self {
            Self::Stl => "STL mesh (.stl)",
            Self::ThreeMf => "3MF model (.3mf)",
            Self::Obj => "OBJ mesh (.obj)",
            Self::Ply => "PLY mesh (.ply)",
            Self::Gltf => "glTF scene (.gltf)",
            Self::Glb => "GLB scene (.glb)",
            Self::Wrl => "VRML scene (.wrl)",
            Self::Project => "Builder3D project (.3dp)",
        }
    }
}

/// What the native save window is asked for: its title, the one file type it
/// filters to, and the suggested path (directory plus file name).
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct SavePickerRequest {
    pub(crate) title: &'static str,
    pub(crate) file_type: FileType,
    pub(crate) suggested: PathBuf,
}

/// Opens a native save window. Replaceable so tests never block on one.
pub(crate) type SavePicker = fn(&SavePickerRequest) -> Option<PathBuf>;

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
    /// Select the shell size (Big doubles it).
    UiSize(UiSize),
    /// Update the mesh light, highlights, and floor contact shadows.
    ViewportLighting(ViewportLighting),
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
    ui_size: UiSize,
    file_type: FileType,
    save_picker: SavePicker,
    viewport_lighting: ViewportLighting,
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
            DialogKind::SaveProject | DialogKind::Export => String::from("Builder3D.stl"),
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
            ui_size: UiSize::Normal,
            file_type: FileType::Stl,
            save_picker: native_save_picker,
            viewport_lighting: ViewportLighting::default(),
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

    /// Applies the persisted viewport light and shadow controls to Settings.
    pub(crate) fn set_viewport_lighting(&mut self, lighting: ViewportLighting) {
        self.viewport_lighting = lighting.normalized();
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
    pub(crate) const fn theme_mode(&self) -> ThemeMode {
        self.theme_mode
    }

    /// Settings stays alongside the viewport; its About popup remains modal.
    pub(crate) fn is_modal(&self) -> bool {
        self.kind != DialogKind::Settings || self.about_open
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

    /// Draws the dialog, docking Settings into the space between shell bars.
    pub(crate) fn show_with_settings_area(
        &mut self,
        ctx: &Context,
        settings_work_area: Option<egui::Rect>,
    ) -> Option<DialogAction> {
        let mut action = None;
        let title = self.title();
        let window = egui::Window::new(title).collapsible(false).resizable(false);
        let window = if self.kind == DialogKind::Settings {
            if let Some(rect) = settings_work_area {
                window.fixed_rect(rect)
            } else {
                window.anchor(Align2::CENTER_CENTER, [0.0, 0.0])
            }
        } else {
            window.anchor(Align2::CENTER_CENTER, [0.0, 0.0])
        };
        let shown = window.show(ctx, |ui| {
            let min_width = if self.kind == DialogKind::Settings {
                settings_work_area.map_or(360.0, |rect| (rect.width() - 24.0).max(240.0))
            } else {
                360.0
            };
            ui.set_min_width(min_width);
            match self.kind {
                DialogKind::OpenProject => self.path_dialog(ui, "Open", &mut action),
                DialogKind::ImportModel => self.import_dialog(ui, &mut action),
                DialogKind::SaveProject | DialogKind::Export => self.file_type_dialog(ui),
                DialogKind::Settings => {
                    let max_height = settings_work_area.map_or_else(
                        || (ctx.content_rect().height() - 160.0).max(280.0),
                        |rect| (rect.height() - 92.0).max(120.0),
                    );
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
            match self.kind {
                DialogKind::Settings => self.settings_footer(ui, &mut action),
                DialogKind::SaveProject | DialogKind::Export => {
                    self.file_type_footer(ui, &mut action);
                }
                _ => {
                    if ui.button("Cancel").clicked() {
                        action = Some(DialogAction::Close);
                    }
                }
            }
        });
        if action.is_none()
            && let Some(shown) = shown
            && self.settings_dismissed_by_outside_press(ctx, shown.response.rect)
        {
            action = Some(DialogAction::Close);
        }
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

    /// Settings is non-modal, so a press anywhere outside its window closes it
    /// (owner request OF-17). A press inside one of its own drop-down lists
    /// (an open popup) or while About is up does not count.
    fn settings_dismissed_by_outside_press(&self, ctx: &Context, window_rect: egui::Rect) -> bool {
        if self.kind != DialogKind::Settings || self.about_open || egui::Popup::is_any_open(ctx) {
            return false;
        }
        ctx.input(|input| {
            input.pointer.any_pressed()
                && input
                    .pointer
                    .press_origin()
                    .is_some_and(|origin| !window_rect.contains(origin))
        })
    }

    /// Keeps Settings' Big/Normal choice in step with the applied size.
    pub(crate) fn sync_ui_size(&mut self, size: UiSize) {
        self.ui_size = size;
    }

    /// Seeds the native save window's folder and file name from the current
    /// document; the chosen type's extension is put on when the window opens.
    pub(crate) fn set_suggested_path(&mut self, document: Option<&Path>) {
        if let Some(stem) = document.and_then(Path::file_stem) {
            let mut suggested = document
                .and_then(Path::parent)
                .map_or_else(PathBuf::new, Path::to_path_buf);
            suggested.push(stem);
            self.path_text = suggested.to_string_lossy().into_owned();
        }
    }

    #[cfg(test)]
    pub(crate) fn set_save_picker(&mut self, picker: SavePicker) {
        self.save_picker = picker;
    }

    #[cfg(test)]
    pub(crate) fn set_file_type(&mut self, file_type: FileType) {
        self.file_type = file_type;
    }

    #[cfg(test)]
    pub(crate) const fn ui_size(&self) -> UiSize {
        self.ui_size
    }

    /// Keeps Settings' Light/Dark choice in step with a theme changed
    /// elsewhere (the top-bar toggle) while Settings is open (OF-16).
    pub(crate) fn sync_theme_mode(&mut self, mode: ThemeMode) {
        self.theme_mode = mode;
    }

    fn path_dialog(
        &mut self,
        ui: &mut egui::Ui,
        action_label: &str,
        action: &mut Option<DialogAction>,
    ) {
        ui.label(match self.kind {
            DialogKind::OpenProject => "Builder3D project (.3dp)",
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
                _ => return,
            });
        }
    }

    /// Save: quick-pick STL or 3MF (STL default). Export: a drop-down of every
    /// type. The file window opens from the Save / Export button (OF-22..25).
    fn file_type_dialog(&mut self, ui: &mut egui::Ui) {
        if self.kind == DialogKind::SaveProject {
            ui.label("Save as");
            ui.horizontal(|ui| {
                for file_type in FileType::SAVE {
                    let button = egui::Button::selectable(
                        self.file_type == file_type,
                        RichText::new(file_type.short_label()).strong(),
                    )
                    .min_size(egui::vec2(72.0, 32.0));
                    if ui.add(button).on_hover_text(file_type.label()).clicked() {
                        self.file_type = file_type;
                    }
                }
            });
        } else {
            ui.label("File type");
            let combo = egui::ComboBox::from_id_salt("export-file-type")
                .width(240.0)
                .selected_text(self.file_type.label())
                .show_ui(ui, |ui| {
                    for file_type in FileType::EXPORT {
                        ui.selectable_value(&mut self.file_type, file_type, file_type.label());
                    }
                });
            combo.response.widget_info(|| {
                egui::WidgetInfo::labeled(egui::Role::ComboBox, true, "Export file type")
            });
        }
    }

    /// The Save / Export button and Cancel on one line (OF-22, OF-25).
    fn file_type_footer(&mut self, ui: &mut egui::Ui, action: &mut Option<DialogAction>) {
        let (label, title) = if self.kind == DialogKind::SaveProject {
            ("Save", "Save Builder3D model")
        } else {
            ("Export", "Export Builder3D model")
        };
        ui.horizontal(|ui| {
            if ui.button(label).clicked() {
                let request = SavePickerRequest {
                    title,
                    file_type: self.file_type,
                    suggested: Path::new(self.path_text.trim())
                        .with_extension(self.file_type.extension()),
                };
                if let Some(selected) = (self.save_picker)(&request) {
                    *action = Some(self.file_action(selected));
                }
            }
            if ui.button("Cancel").clicked() {
                *action = Some(DialogAction::Close);
            }
        });
    }

    /// Turns a path chosen in the native window into Save / Export, adding
    /// the chosen type's extension when the window returned none. The native
    /// window has already asked about replacing an existing file.
    fn file_action(&self, selected: PathBuf) -> DialogAction {
        let path = if selected.extension().is_some() {
            selected
        } else {
            selected.with_extension(self.file_type.extension())
        };
        let confirmed = path.is_file();
        if self.kind == DialogKind::SaveProject {
            DialogAction::Save(path, confirmed)
        } else {
            DialogAction::Export(export_path_with_default_extension(path), confirmed)
        }
    }

    /// Application, About and Close on one line under Settings (OF-21).
    fn settings_footer(&mut self, ui: &mut egui::Ui, action: &mut Option<DialogAction>) {
        ui.separator();
        ui.horizontal(|ui| {
            ui.label("Application");
            if ui.button("About").clicked() {
                self.about_open = true;
            }
            if ui.button("Close").clicked() {
                *action = Some(DialogAction::Close);
            }
        });
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
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label("Theme");
                ui.horizontal(|ui| {
                    for (mode, label) in [(ThemeMode::Light, "Light"), (ThemeMode::Dark, "Dark")] {
                        if ui
                            .selectable_label(self.theme_mode == mode, label)
                            .clicked()
                        {
                            self.theme_mode = mode;
                            *action = Some(DialogAction::Theme(mode));
                        }
                    }
                });
            });
            ui.add_space(24.0);
            ui.vertical(|ui| {
                ui.label("Size");
                ui.horizontal(|ui| {
                    for (size, label) in [(UiSize::Big, "Big"), (UiSize::Normal, "Normal")] {
                        if ui.selectable_label(self.ui_size == size, label).clicked() {
                            self.ui_size = size;
                            *action = Some(DialogAction::UiSize(size));
                        }
                    }
                });
            });
        });
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
        ui.heading("Viewport lighting");
        ui.label(
            RichText::new(
                "Keep the key light on the same side of the screen as you orbit, or turn this off to keep it fixed in the scene.",
            )
            .weak(),
        );
        let mut lighting = self.viewport_lighting;
        let mut lighting_changed = false;
        if ui.button("Restore 3D Builder defaults").clicked() {
            lighting = ViewportLighting::default();
            lighting_changed = true;
        }
        lighting_changed |= ui
            .checkbox(&mut lighting.screen_aligned, "Screen-aligned light")
            .changed();
        lighting_changed |= ui
            .add(
                egui::Slider::new(&mut lighting.azimuth_degrees, -80..=80)
                    .text("Light left / right"),
            )
            .changed();
        lighting_changed |= ui
            .add(egui::Slider::new(&mut lighting.elevation_degrees, 10..=85).text("Light height"))
            .changed();
        lighting_changed |= ui
            .add(egui::Slider::new(&mut lighting.intensity_percent, 0..=150).text("Light strength"))
            .changed();
        lighting_changed |= ui
            .add(egui::Slider::new(&mut lighting.ambient_percent, 0..=80).text("Ambient fill"))
            .changed();
        lighting_changed |= ui
            .add(
                egui::Slider::new(&mut lighting.specular_percent, 0..=100)
                    .text("Highlight strength"),
            )
            .changed();
        lighting_changed |= ui
            .add(
                egui::Slider::new(&mut lighting.highlight_sharpness, 0..=100)
                    .text("Highlight sharpness"),
            )
            .changed();
        lighting_changed |= ui
            .add(
                egui::Slider::new(&mut lighting.contact_shadow_percent, 0..=200)
                    .text("Floor shadow strength"),
            )
            .changed();
        if lighting_changed {
            self.viewport_lighting = lighting.normalized();
            *action = Some(DialogAction::ViewportLighting(self.viewport_lighting));
        }
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
                    RichText::new("No slicer chosen yet. Click Slicer in the top bar to pick one.")
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
            "Choose a color for {} selected object(s)",
            self.selected_object_count
        ));
        // Basic colors, as in the Python build's color dialog.
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(4.0, 4.0);
            for swatch in MATERIAL_SWATCHES {
                let color = egui::Color32::from_rgb(swatch[0], swatch[1], swatch[2]);
                let (rect, response) =
                    ui.allocate_exact_size(egui::vec2(22.0, 22.0), egui::Sense::click());
                let chosen = self.color[..3]
                    .iter()
                    .zip(swatch)
                    .all(|(value, channel)| (value * 255.0).round() as u8 == channel);
                ui.painter().rect_filled(rect, 4.0, color);
                ui.painter().rect_stroke(
                    rect,
                    4.0,
                    egui::Stroke::new(
                        if chosen || response.hovered() {
                            2.0
                        } else {
                            1.0
                        },
                        ui.visuals().widgets.inactive.bg_stroke.color,
                    ),
                    egui::StrokeKind::Inside,
                );
                if response.clicked() {
                    let alpha = self.color[3];
                    self.color = [
                        f32::from(swatch[0]) / 255.0,
                        f32::from(swatch[1]) / 255.0,
                        f32::from(swatch[2]) / 255.0,
                        alpha,
                    ];
                }
            }
        });
        ui.add_space(4.0);
        let alpha = self.color[3];
        if let Some([r, g, b]) = hsv_picker(ui, [self.color[0], self.color[1], self.color[2]]) {
            self.color = [r, g, b, alpha];
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
            DialogKind::SaveProject => "Save",
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
        _ => None,
    }
}

/// The native save window for Save and Export, filtered to one file type.
fn native_save_picker(request: &SavePickerRequest) -> Option<PathBuf> {
    let mut dialog = rfd::FileDialog::new()
        .set_title(request.title)
        .add_filter(request.file_type.label(), &[request.file_type.extension()]);
    if let Some(parent) = request
        .suggested
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        dialog = dialog.set_directory(parent);
    }
    if let Some(name) = request.suggested.file_name().and_then(|name| name.to_str()) {
        dialog = dialog.set_file_name(name);
    }
    dialog.save_file()
}

/// A compact saturation/value square over a hue bar, with a preview chip.
/// Returns the new sRGB color (0..1 channels) when the user changed it.
fn hsv_picker(ui: &mut egui::Ui, rgb: [f32; 3]) -> Option<[f32; 3]> {
    let id = ui.scope_id().with("builder3d-hsv-picker");
    // Keep the hue while saturation or value is zero, where RGB loses it.
    let stored = ui.data(|data| data.get_temp::<[f32; 3]>(id));
    let mut hsv = rgb_to_hsv(rgb);
    if let Some(previous) = stored
        && hsv_to_rgb(previous)
            .iter()
            .zip(rgb)
            .all(|(a, b)| (a - b).abs() < 1.5 / 255.0)
    {
        hsv = previous;
    }
    let mut changed = false;
    let size = egui::vec2(220.0, 140.0);
    let (square, response) = ui.allocate_exact_size(size, egui::Sense::click_and_drag());
    let hue_rgb = hsv_to_rgb([hsv[0], 1.0, 1.0]);
    let to_color = |c: [f32; 3]| {
        egui::Color32::from_rgb(
            (c[0] * 255.0).round() as u8,
            (c[1] * 255.0).round() as u8,
            (c[2] * 255.0).round() as u8,
        )
    };
    // White to hue across, then a black overlay down: the classic picker.
    let mut mesh = egui::Mesh::default();
    mesh.colored_vertex(square.left_top(), egui::Color32::WHITE);
    mesh.colored_vertex(square.right_top(), to_color(hue_rgb));
    mesh.colored_vertex(square.right_bottom(), to_color(hue_rgb));
    mesh.colored_vertex(square.left_bottom(), egui::Color32::WHITE);
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(0, 2, 3);
    mesh.colored_vertex(square.left_top(), egui::Color32::TRANSPARENT);
    mesh.colored_vertex(square.right_top(), egui::Color32::TRANSPARENT);
    mesh.colored_vertex(square.right_bottom(), egui::Color32::BLACK);
    mesh.colored_vertex(square.left_bottom(), egui::Color32::BLACK);
    mesh.add_triangle(4, 5, 6);
    mesh.add_triangle(4, 6, 7);
    ui.painter().add(egui::Shape::mesh(mesh));
    if let Some(pointer) = response.interact_pointer_pos() {
        hsv[1] = ((pointer.x - square.left()) / square.width()).clamp(0.0, 1.0);
        hsv[2] = 1.0 - ((pointer.y - square.top()) / square.height()).clamp(0.0, 1.0);
        changed = true;
    }
    let marker = egui::pos2(
        square.left() + hsv[1] * square.width(),
        square.top() + (1.0 - hsv[2]) * square.height(),
    );
    ui.painter()
        .circle_stroke(marker, 6.0, egui::Stroke::new(2.0, egui::Color32::WHITE));
    ui.painter().circle_stroke(
        marker,
        7.0,
        egui::Stroke::new(1.0, egui::Color32::from_black_alpha(140)),
    );
    let (bar, bar_response) =
        ui.allocate_exact_size(egui::vec2(size.x, 14.0), egui::Sense::click_and_drag());
    let mut hue_mesh = egui::Mesh::default();
    for step in 0..=6 {
        let t = step as f32 / 6.0;
        let color = to_color(hsv_to_rgb([t, 1.0, 1.0]));
        let x = bar.left() + t * bar.width();
        hue_mesh.colored_vertex(egui::pos2(x, bar.top()), color);
        hue_mesh.colored_vertex(egui::pos2(x, bar.bottom()), color);
        if step > 0 {
            let base = (step as u32 - 1) * 2;
            hue_mesh.add_triangle(base, base + 1, base + 2);
            hue_mesh.add_triangle(base + 1, base + 3, base + 2);
        }
    }
    ui.painter().add(egui::Shape::mesh(hue_mesh));
    if let Some(pointer) = bar_response.interact_pointer_pos() {
        hsv[0] = ((pointer.x - bar.left()) / bar.width()).clamp(0.0, 0.9999);
        changed = true;
    }
    let hue_x = bar.left() + hsv[0] * bar.width();
    ui.painter().rect_stroke(
        egui::Rect::from_center_size(egui::pos2(hue_x, bar.center().y), egui::vec2(6.0, 18.0)),
        2.0,
        egui::Stroke::new(2.0, egui::Color32::WHITE),
        egui::StrokeKind::Middle,
    );
    let result = hsv_to_rgb(hsv);
    ui.horizontal(|ui| {
        let (chip, _) = ui.allocate_exact_size(egui::vec2(40.0, 20.0), egui::Sense::hover());
        ui.painter().rect_filled(chip, 4.0, to_color(result));
        ui.label(format!(
            "#{:02X}{:02X}{:02X}",
            (result[0] * 255.0).round() as u8,
            (result[1] * 255.0).round() as u8,
            (result[2] * 255.0).round() as u8
        ));
    });
    ui.data_mut(|data| data.insert_temp(id, hsv));
    changed.then_some(result)
}

fn rgb_to_hsv([r, g, b]: [f32; 3]) -> [f32; 3] {
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let delta = max - min;
    let hue = if delta <= f32::EPSILON {
        0.0
    } else if max == r {
        ((g - b) / delta).rem_euclid(6.0) / 6.0
    } else if max == g {
        ((b - r) / delta + 2.0) / 6.0
    } else {
        ((r - g) / delta + 4.0) / 6.0
    };
    let saturation = if max <= f32::EPSILON {
        0.0
    } else {
        delta / max
    };
    [hue, saturation, max]
}

fn hsv_to_rgb([h, s, v]: [f32; 3]) -> [f32; 3] {
    let sector = (h.rem_euclid(1.0) * 6.0).min(5.9999);
    let index = sector.floor();
    let fraction = sector - index;
    let (p, q, t) = (
        v * (1.0 - s),
        v * (1.0 - s * fraction),
        v * (1.0 - s * (1.0 - fraction)),
    );
    match index as u8 {
        0 => [v, t, p],
        1 => [q, v, p],
        2 => [p, v, t],
        3 => [p, q, v],
        4 => [t, p, v],
        _ => [v, p, q],
    }
}

/// The Python color dialog's basic colors, as RGB.
const MATERIAL_SWATCHES: [[u8; 3]; 16] = [
    [255, 255, 255],
    [192, 192, 192],
    [128, 128, 128],
    [32, 32, 32],
    [220, 40, 40],
    [240, 130, 30],
    [250, 210, 40],
    [120, 200, 60],
    [40, 160, 80],
    [40, 180, 200],
    [40, 110, 220],
    [90, 70, 200],
    [160, 60, 190],
    [230, 90, 160],
    [150, 95, 50],
    [235, 205, 160],
];

pub(crate) fn image_path_picker(current: &str) -> Option<PathBuf> {
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
        assert_eq!(state.path_text, "Builder3D.stl");
        assert_eq!(state.file_type, FileType::Stl);
        assert!(state.torus.radius_mm > state.torus.tube_mm);
        assert_eq!(state.theme_mode, ThemeMode::Light);

        let import = DialogState::new(DialogKind::ImportModel, ThemeMode::Light);
        assert_eq!(import.path_text, "model.stl");
    }

    #[test]
    fn settings_stays_non_modal_until_about_is_opened() {
        let mut settings = DialogState::new(DialogKind::Settings, ThemeMode::Dark);
        assert!(!settings.is_modal());
        settings.about_open = true;
        assert!(settings.is_modal());
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

    fn press_at(pos: egui::Pos2) -> Vec<egui::RawInput> {
        let screen = Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1280.0, 840.0),
        ));
        let pointer_button = |pressed| egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::default(),
        };
        vec![
            egui::RawInput {
                screen_rect: screen,
                events: vec![egui::Event::PointerMoved(pos)],
                ..Default::default()
            },
            egui::RawInput {
                screen_rect: screen,
                events: vec![pointer_button(true)],
                ..Default::default()
            },
            egui::RawInput {
                screen_rect: screen,
                events: vec![pointer_button(false)],
                ..Default::default()
            },
        ]
    }

    fn settings_actions_for_press(pos: egui::Pos2) -> Vec<DialogAction> {
        let context = Context::default();
        let mut dialog = DialogState::new(DialogKind::Settings, ThemeMode::Light);
        let area = egui::Rect::from_min_max(egui::pos2(0.0, 130.0), egui::pos2(420.0, 800.0));
        let mut actions = Vec::new();
        for input in std::iter::once(egui::RawInput::default()).chain(press_at(pos)) {
            let mut output = context.run_ui(input, |ui| {
                actions.extend(dialog.show_with_settings_area(ui.ctx(), Some(area)));
            });
            output.textures_delta.clear();
        }
        actions
    }

    #[test]
    fn pressing_outside_settings_closes_it_and_inside_does_not() {
        assert_eq!(
            settings_actions_for_press(egui::pos2(900.0, 400.0)),
            vec![DialogAction::Close]
        );
        assert!(settings_actions_for_press(egui::pos2(200.0, 400.0)).is_empty());
    }

    #[test]
    fn settings_has_one_close_button() {
        let source = include_str!("dialogs.rs");
        let settings_body = source
            .split("fn settings_dialog(")
            .nth(1)
            .and_then(|rest| rest.split("\n    fn ").next())
            .unwrap_or_default();
        assert!(!settings_body.is_empty());
        assert!(!settings_body.contains("\"Done\""));
    }

    /// Renders `dialog` for a few frames, pressing at `press` (if any), and
    /// returns its actions plus every labelled widget's rect from the last
    /// frame (read from egui's accessibility tree).
    fn run_dialog(
        dialog: &mut DialogState,
        press: Option<egui::Pos2>,
    ) -> (Vec<DialogAction>, Vec<(String, egui::Rect)>) {
        let context = Context::default();
        context.enable_accesskit();
        let area = egui::Rect::from_min_max(egui::pos2(0.0, 130.0), egui::pos2(420.0, 800.0));
        let screen = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1280.0, 840.0),
            )),
            ..Default::default()
        };
        let mut inputs = vec![screen.clone(), screen];
        if let Some(pos) = press {
            inputs.extend(press_at(pos));
        }
        let mut actions = Vec::new();
        let mut widgets = Vec::new();
        for input in inputs {
            let mut output = context.run_ui(input, |ui| {
                actions.extend(dialog.show_with_settings_area(ui.ctx(), Some(area)));
            });
            output.textures_delta.clear();
            if let Some(update) = output.platform_output.accesskit_update {
                widgets = update
                    .nodes
                    .iter()
                    .filter_map(|(_, node)| {
                        // The window title shares text with its buttons
                        // ("Save"); keep titles out of the lookup.
                        if node.role() == egui::accesskit::Role::Window {
                            return None;
                        }
                        let label = node.label()?.to_owned();
                        let bounds = node.bounds()?;
                        Some((
                            label,
                            egui::Rect::from_min_max(
                                egui::pos2(bounds.x0 as f32, bounds.y0 as f32),
                                egui::pos2(bounds.x1 as f32, bounds.y1 as f32),
                            ),
                        ))
                    })
                    .collect();
            }
        }
        (actions, widgets)
    }

    fn widget(widgets: &[(String, egui::Rect)], label: &str) -> egui::Rect {
        widgets
            .iter()
            .find(|(text, _)| text == label)
            .map(|(_, rect)| *rect)
            .unwrap_or_else(|| panic!("no widget labelled {label:?} in {widgets:?}"))
    }

    fn has_widget(widgets: &[(String, egui::Rect)], label: &str) -> bool {
        widgets.iter().any(|(text, _)| text == label)
    }

    fn same_line(a: egui::Rect, b: egui::Rect) -> bool {
        (a.center().y - b.center().y).abs() < 2.0
    }

    thread_local! {
        static PICKED: std::cell::RefCell<Vec<SavePickerRequest>> =
            const { std::cell::RefCell::new(Vec::new()) };
    }

    /// A save window stub that records the request and "chooses" the
    /// suggested path without its extension, as some native windows return.
    fn stub_picker(request: &SavePickerRequest) -> Option<PathBuf> {
        PICKED.with(|picked| picked.borrow_mut().push(request.clone()));
        Some(request.suggested.with_extension(""))
    }

    fn take_picked() -> Vec<SavePickerRequest> {
        PICKED.with(|picked| std::mem::take(&mut *picked.borrow_mut()))
    }

    #[test]
    fn save_dialog_offers_stl_and_3mf_with_save_and_cancel_on_one_line() {
        let mut dialog = DialogState::new(DialogKind::SaveProject, ThemeMode::Light);
        let (actions, widgets) = run_dialog(&mut dialog, None);
        assert!(actions.is_empty());
        assert!(has_widget(&widgets, "STL"));
        assert!(has_widget(&widgets, "3MF"));
        assert!(same_line(
            widget(&widgets, "Save"),
            widget(&widgets, "Cancel")
        ));
        assert!(!has_widget(&widgets, "Browse..."));
        assert_eq!(dialog.file_type, FileType::Stl);
    }

    #[test]
    fn save_click_opens_the_file_window_for_stl_by_default() {
        take_picked();
        let mut dialog = DialogState::new(DialogKind::SaveProject, ThemeMode::Light);
        dialog.set_save_picker(stub_picker);
        dialog.set_suggested_path(Some(Path::new("models/part.3dp")));
        let (_, widgets) = run_dialog(&mut dialog, None);
        let save = widget(&widgets, "Save").center();
        let (actions, _) = run_dialog(&mut dialog, Some(save));
        let picked = take_picked();
        assert_eq!(picked.len(), 1);
        assert_eq!(picked[0].file_type, FileType::Stl);
        assert_eq!(picked[0].suggested, Path::new("models").join("part.stl"));
        assert_eq!(
            actions,
            vec![DialogAction::Save(
                Path::new("models").join("part.stl"),
                false
            )]
        );
    }

    #[test]
    fn save_as_3mf_is_one_click() {
        take_picked();
        let mut dialog = DialogState::new(DialogKind::SaveProject, ThemeMode::Light);
        dialog.set_save_picker(stub_picker);
        let (_, widgets) = run_dialog(&mut dialog, None);
        let three_mf = widget(&widgets, "3MF").center();
        let (actions, _) = run_dialog(&mut dialog, Some(three_mf));
        assert!(actions.is_empty());
        assert_eq!(dialog.file_type, FileType::ThreeMf);
        let (_, widgets) = run_dialog(&mut dialog, None);
        let (actions, _) = run_dialog(&mut dialog, Some(widget(&widgets, "Save").center()));
        assert_eq!(take_picked()[0].file_type, FileType::ThreeMf);
        assert_eq!(
            actions,
            vec![DialogAction::Save(PathBuf::from("Builder3D.3mf"), false)]
        );
    }

    #[test]
    fn export_has_a_type_list_and_export_beside_cancel_opening_the_file_window() {
        take_picked();
        let mut dialog = DialogState::new(DialogKind::Export, ThemeMode::Light);
        dialog.set_save_picker(stub_picker);
        let (_, widgets) = run_dialog(&mut dialog, None);
        assert!(has_widget(&widgets, "Export file type"));
        assert!(!has_widget(&widgets, "Browse..."));
        let export = widget(&widgets, "Export");
        assert!(same_line(export, widget(&widgets, "Cancel")));
        assert_eq!(FileType::EXPORT.len(), 8);
        dialog.set_file_type(FileType::Project);
        let (actions, _) = run_dialog(&mut dialog, Some(export.center()));
        assert_eq!(take_picked()[0].file_type, FileType::Project);
        assert_eq!(
            actions,
            vec![DialogAction::Export(PathBuf::from("Builder3D.3dp"), false)]
        );
    }

    #[test]
    fn cancelling_the_file_window_keeps_the_dialog_open() {
        let mut dialog = DialogState::new(DialogKind::Export, ThemeMode::Light);
        dialog.set_save_picker(|_| None);
        let (_, widgets) = run_dialog(&mut dialog, None);
        let (actions, _) = run_dialog(&mut dialog, Some(widget(&widgets, "Export").center()));
        assert!(actions.is_empty());
    }

    #[test]
    fn settings_size_sits_beside_theme_and_big_doubles() {
        let mut dialog = DialogState::new(DialogKind::Settings, ThemeMode::Light);
        let (_, widgets) = run_dialog(&mut dialog, None);
        let light = widget(&widgets, "Light");
        let big = widget(&widgets, "Big");
        assert!(same_line(light, big));
        assert!(big.left() > widget(&widgets, "Dark").right());
        assert!(has_widget(&widgets, "Normal"));
        assert!(
            !widgets
                .iter()
                .any(|(text, _)| text.contains("Theme changes apply"))
        );
        let (actions, _) = run_dialog(&mut dialog, Some(big.center()));
        assert_eq!(actions, vec![DialogAction::UiSize(UiSize::Big)]);
        assert_eq!(dialog.ui_size(), UiSize::Big);
        dialog.sync_ui_size(UiSize::Normal);
        assert_eq!(dialog.ui_size(), UiSize::Normal);
    }

    #[test]
    fn settings_application_about_and_close_share_one_line() {
        let mut dialog = DialogState::new(DialogKind::Settings, ThemeMode::Light);
        let (_, widgets) = run_dialog(&mut dialog, None);
        // "Application" is plain text (no accessible label); it sits left of
        // About, so About starts well right of the panel edge.
        let about = widget(&widgets, "About");
        let close = widget(&widgets, "Close");
        assert!(same_line(about, close));
        assert!(about.left() > 60.0);
        assert!(about.bottom() > widget(&widgets, "Clear Recent Files List").bottom());
        let (actions, _) = run_dialog(&mut dialog, Some(close.center()));
        assert_eq!(actions, vec![DialogAction::Close]);
    }

    #[test]
    fn settings_follows_a_theme_changed_elsewhere() {
        let mut dialog = DialogState::new(DialogKind::Settings, ThemeMode::Light);
        dialog.sync_theme_mode(ThemeMode::Dark);
        assert_eq!(dialog.theme_mode, ThemeMode::Dark);
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
            action = dialog.show_with_settings_area(ui.ctx(), None);
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
            action = dialog.show_with_settings_area(ui.ctx(), None);
        });
        output.textures_delta.clear();
        assert_eq!(action, Some(DialogAction::Close));
    }

    #[test]
    fn hsv_conversion_round_trips_the_swatches() {
        for swatch in MATERIAL_SWATCHES {
            let rgb = swatch.map(|channel| f32::from(channel) / 255.0);
            let back = hsv_to_rgb(rgb_to_hsv(rgb));
            for (a, b) in rgb.iter().zip(back) {
                assert!((a - b).abs() < 1.0e-4, "{swatch:?}");
            }
        }
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
