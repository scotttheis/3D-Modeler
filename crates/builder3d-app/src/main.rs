// Release builds on Windows are windowed so no console opens behind the app
// (OF-28); `--console` brings one back for diagnostics.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

fn main() -> Result<(), builder3d_app::AppError> {
    builder3d_app::run()
}
