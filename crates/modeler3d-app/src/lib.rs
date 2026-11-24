#![deny(unsafe_code)]

//! Native 3D Modeler application shell.
//!
//! The crate is the desktop adapter: it presents the ribbon and shell, routes
//! commands to `modeler3d-core`, and consumes renderer-neutral data from
//! `modeler3d-viewport`. Platform paths and process capabilities stay in
//! `modeler3d-platform`.

mod artwork;
mod brush_outline;
mod chrome;
mod commands;
mod desktop_jobs;
mod diagnostics;
mod dialogs;
mod export;
mod floor_labels;
mod grab_cursor;
mod image_color;
mod launchers;
mod mcp_bridge;
mod numeric_controls;
mod operations;
mod packaging;
mod preferences;
mod print_handoff;
mod print_prep;
mod recent_preview;
mod resize_overlay;
mod review_capture;
mod rotation;
mod shell;
mod shrink_assets;
mod signals;
mod theme;
mod toast;
mod transform_gesture;
mod transforms;
mod ui_state;
mod updates;
mod view_state;

use std::{
    env,
    ffi::OsString,
    path::{Path, PathBuf},
};

use thiserror::Error;

pub use artwork::{ArtworkAsset, ArtworkError, ThemeArtwork};
pub use commands::{
    CommandDescriptor, CommandError, CommandId, CommandOutcome, CommandState, DialogKind,
    DisplayState, IconKey, ImportUnit, RibbonItem, RibbonTab, ViewOrientation, file_menu_spec,
    ribbon_spec,
};
pub use export::{ExportError, ExportReport, export_indices, export_scene, export_scene_objects};
pub use image_color::{ImageColorError, MAX_IMAGE_BYTES, MAX_IMAGE_PIXELS, average_image_color};
pub use launchers::{
    LauncherError, LauncherSpec, launcher_specs, stage_launchers, validate_launchers,
};
pub use numeric_controls::{
    format_distance, parse_angle_degrees, parse_distance_mm, parse_scale_factor,
    select_numeric_value,
};
pub use operations::{
    MAX_OPERATION_RECORDS, OPERATION_RETENTION, OperationFailure, OperationHandle, OperationPhase,
    OperationRegistry, OperationRegistryError, OperationStatus,
};
pub use packaging::{
    InstalledMeasurement, InstallerSpec, MAX_COMPRESSED_BYTES, MAX_INSTALLED_BYTES, PackageEntry,
    PackageMeasurement, PackagingError, installer_specs, measure_installed, stage_installers,
    stage_package, validate_installers,
};
pub use preferences::{
    DisplayUnit, MAX_RECENT_FILES, MAX_RIBBON_FAVORITES, Preferences, PreferencesError,
    RecentFiles, import_unit_for_path, import_uses_authoritative_units, normalize_import_unit,
};
pub use print_handoff::{
    PreparedPrint, PrintHandoffError, discover_print_slicers, launch_prepared_print, prepare_print,
    prepare_print_by_key,
};
pub use print_prep::{
    BED_SURFACE_TOLERANCE_MM, DEFAULT_BED_SIZE_MM, MAX_TYPICAL_PRINT_DIMENSION_MM,
    MIN_PRINTABLE_DIMENSION_MM, PrintPrepError, PrintPrepReport, check_bed_fit, check_empty_hidden,
    check_non_manifold, check_scale, full_check,
};
pub use shell::Modeler3dApp;
pub use signals::{Signal, SignalConnection};
pub use theme::{ThemeMode, ThemePalette};
pub use transforms::{Alignment, Axis, GroupTransform, TransformError, WorldBounds};
pub use view_state::{
    FrameTarget, ViewDisplayState, ViewState, ViewStateController, ViewStateError,
};

/// Errors returned by the native application entry point.
#[derive(Debug, Error)]
pub enum AppError {
    /// eframe could not create or run the native window.
    #[error("native shell failed: {0}")]
    Native(#[from] eframe::Error),
    /// A command smoke check failed.
    #[error(transparent)]
    Command(#[from] CommandError),
    /// The original shell artwork failed its package validation.
    #[error(transparent)]
    Artwork(#[from] ArtworkError),
    /// A native launcher contract failed validation.
    #[error(transparent)]
    Launcher(#[from] LauncherError),
    /// Native package staging failed.
    #[error(transparent)]
    Packaging(#[from] PackagingError),
    /// The application entry point could not inspect its own executable.
    #[error("application I/O failed: {0}")]
    Io(#[from] std::io::Error),
}

/// Runs the native 3D Modeler shell, or the bounded command smoke check.
///
/// On Windows the release executable is windowed, so no console opens behind
/// the app (OF-28). Command-line modes and review captures attach to the
/// terminal that started them; `--console` also opens a console when there is
/// none and mirrors the diagnostic log to it.
pub fn run() -> Result<(), AppError> {
    diagnostics::initialize();
    diagnostics::install_panic_hook();
    let console_requested = has_argument("--console");
    if console_requested || wants_terminal_output() {
        let attached = modeler3d_platform::attach_console(console_requested);
        if console_requested && attached {
            diagnostics::echo_to_console();
        }
    }
    let result = run_inner();
    if let Err(error) = &result {
        diagnostics::record_error("application startup or shutdown", error);
    }
    result
}

fn run_inner() -> Result<(), AppError> {
    if has_argument("--require-update-feed") {
        modeler3d_platform::updates::validate_repository(env!("MODELER3D_UPDATE_REPOSITORY"))
            .map_err(|error| std::io::Error::other(error.to_string()))?;
        println!(
            "Update feed: https://github.com/{}",
            env!("MODELER3D_UPDATE_REPOSITORY")
        );
        return Ok(());
    }
    if has_argument("--version") {
        println!("3D Modeler {}", updates::VERSION);
        return Ok(());
    }
    if let Some(directory) = argument_value("--shrink-assets") {
        let report = shrink_assets::shrink_assets(
            Path::new(&directory),
            shrink_assets::RELEASE_ICON_PIXELS,
        )?;
        println!(
            "shrunk {} icons to at most {} px: {} -> {} bytes",
            report.shrunk,
            shrink_assets::RELEASE_ICON_PIXELS,
            report.bytes_before,
            report.bytes_after
        );
        return Ok(());
    }
    ThemeArtwork::new(ThemeMode::Light).validate()?;
    ThemeArtwork::new(ThemeMode::Dark).validate()?;
    let artwork_root = artwork::original_asset_root();
    artwork::validate_original_artwork(&artwork_root)?;
    validate_launchers()?;
    validate_installers()?;
    if let Some(destination) = argument_value("--package") {
        let executable = env::current_exe()?;
        let measurement = stage_package(&executable, Path::new(&destination))?;
        println!(
            "packaged {} files: installed={} bytes, archive={} bytes at {}",
            measurement.installed.file_count,
            measurement.installed.installed_bytes,
            measurement.compressed_bytes,
            measurement.archive_path.display()
        );
        return Ok(());
    }
    if let Some(destination) = argument_value("--stage-launchers") {
        let paths = stage_launchers(Path::new(&destination))?;
        println!("staged {} native launchers in {}", paths.len(), destination);
        return Ok(());
    }
    if has_argument("--smoke") {
        run_smoke_check()?;
        return Ok(());
    }
    let native_options = eframe::NativeOptions {
        // Keep object clipboard shortcuts independent of the OS text clipboard.
        // The app forwards focused text-edit shortcuts through raw_input_hook.
        clipboard_shortcuts: false,
        viewport: eframe::egui::ViewportBuilder::default()
            .with_icon(artwork::load_native_icon(&artwork_root)?)
            .with_inner_size([1_280.0, 840.0])
            .with_min_inner_size([980.0, 640.0]),
        wgpu_options: wgpu_options(),
        ..Default::default()
    };
    let launch_file = launch_file(env::args_os().skip(1));
    let watchdog = diagnostics::UiWatchdog::start();
    let native_result = eframe::run_native(
        "3D Modeler",
        native_options,
        Box::new(move |creation_context| {
            let mut app = Modeler3dApp::new(creation_context);
            if let Some(path) = launch_file {
                app.open_launch_file(&path);
            }
            Ok(Box::new(app))
        }),
    );
    drop(watchdog);
    native_result?;
    Ok(())
}

/// eframe's graphics setup, asking the adapter for its full texture size
/// instead of eframe's fixed 8192 pixels. The window surface and the 3D view
/// are textures, so this is the largest window the app can draw; the window
/// is capped to it as well (OF-71, see `Modeler3dApp::new`).
fn wgpu_options() -> eframe::egui_wgpu::WgpuConfiguration {
    let mut options = eframe::egui_wgpu::WgpuConfiguration::default();
    if let eframe::egui_wgpu::WgpuSetup::CreateNew(create_new) = &mut options.wgpu_setup {
        let eframe_descriptor = std::sync::Arc::clone(&create_new.device_descriptor);
        create_new.device_descriptor = std::sync::Arc::new(move |adapter| {
            let mut descriptor = eframe_descriptor(adapter);
            descriptor.required_limits.max_texture_dimension_2d =
                adapter.limits().max_texture_dimension_2d;
            descriptor
        });
    }
    options
}

fn run_smoke_check() -> Result<(), AppError> {
    let mut state = CommandState::default();
    state.dispatch(CommandId::AddPrimitive(modeler3d_core::PrimitiveKind::Cube))?;
    state.dispatch(CommandId::Copy)?;
    state.dispatch(CommandId::Paste)?;
    println!(
        "3D Modeler shell smoke: objects={} selected={} tabs={}",
        state.scene().objects().len(),
        state.selected_count(),
        RibbonTab::all().len()
    );
    Ok(())
}

fn has_argument(name: &str) -> bool {
    env::args().skip(1).any(|argument| argument == name)
}

/// Modes that print results for whoever started the program.
fn wants_terminal_output() -> bool {
    ["--smoke", "--package", "--stage-launchers"]
        .iter()
        .any(|flag| has_argument(flag))
        || env::var_os("MODELER3D_REVIEW_CAPTURE").is_some()
}

/// The file to open at startup: the first argument that is not a `--` flag
/// or the value of one. Windows passes the double-clicked or "Open with"
/// file this way (`"Modeler3D.exe" "%1"`); a relative path is resolved
/// against the working directory.
fn launch_file(arguments: impl IntoIterator<Item = OsString>) -> Option<PathBuf> {
    const FLAGS_WITH_VALUES: [&str; 3] = ["--package", "--stage-launchers", "--shrink-assets"];
    let mut arguments = arguments.into_iter();
    while let Some(argument) = arguments.next() {
        let text = argument.to_string_lossy();
        if FLAGS_WITH_VALUES.contains(&text.as_ref()) {
            arguments.next();
            continue;
        }
        if text.starts_with("--") || text.is_empty() {
            continue;
        }
        let path = PathBuf::from(argument);
        return Some(std::path::absolute(&path).unwrap_or(path));
    }
    None
}

fn argument_value(name: &str) -> Option<String> {
    let mut arguments = env::args().skip(1);
    while let Some(argument) = arguments.next() {
        if argument == name {
            return arguments.next();
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn arguments(values: &[&str]) -> Vec<OsString> {
        values.iter().map(OsString::from).collect()
    }

    #[test]
    fn launch_file_is_the_first_plain_argument() {
        let file = env::temp_dir().join("part one.3dp");
        let found = launch_file([OsString::from("--console"), file.clone().into_os_string()]);
        assert_eq!(found, Some(file));
    }

    #[test]
    fn launch_file_skips_flag_values_and_flags() {
        assert_eq!(launch_file(arguments(&["--package", r"C:\out"])), None);
        assert_eq!(launch_file(arguments(&["--console", "--smoke"])), None);
        assert_eq!(launch_file(arguments(&[])), None);
    }

    #[test]
    fn launch_file_resolves_a_relative_path() {
        let found = launch_file(arguments(&["part.stl"])).unwrap_or_default();
        assert!(found.is_absolute());
        assert!(found.ends_with("part.stl"));
    }
}
