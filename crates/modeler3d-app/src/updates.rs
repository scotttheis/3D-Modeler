//! Startup update card and background workers. Installation is handed off only
//! after the shell has finished its ordinary save/discard/cancel close flow.

use eframe::egui;
use modeler3d_platform::updates::{self, PreparedUpdate, Update};
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
    mpsc::{self, Receiver, TryRecvError},
};

pub(crate) const VERSION: &str = env!("MODELER3D_RELEASE_VERSION");

enum Event {
    Checked(Result<Option<Update>, String>),
    Downloaded(Result<PreparedUpdate, String>),
}

#[derive(Default)]
pub(crate) struct Updater {
    worker: Option<Receiver<Event>>,
    update: Option<Update>,
    prepared: Option<PreparedUpdate>,
    message: Option<String>,
    downloaded: Arc<AtomicU64>,
    total: Arc<AtomicU64>,
    visible: bool,
    downloading: bool,
    review: bool,
    installable: bool,
    close_when_ready: bool,
    icon: Option<egui::TextureHandle>,
    context: Option<egui::Context>,
}

impl Updater {
    pub(crate) fn start(&mut self, ctx: &egui::Context) {
        self.context = Some(ctx.clone());
        self.installable = updates::can_install();
        self.message = updates::take_outcome();
        self.visible = self.message.is_some();
        let repo = env!("MODELER3D_UPDATE_REPOSITORY");
        if repo.is_empty() {
            return;
        }
        let ctx = ctx.clone();
        let (tx, rx) = mpsc::channel();
        self.worker = Some(rx);
        std::thread::spawn(move || {
            let result = updates::check(repo, VERSION).map_err(|e| e.to_string());
            let _ = tx.send(Event::Checked(result));
            ctx.request_repaint();
        });
    }

    fn poll(&mut self) {
        let Some(worker) = &self.worker else { return };
        match worker.try_recv() {
            Ok(Event::Checked(Ok(update))) => {
                self.visible |= update.is_some();
                self.update = update;
                self.worker = None;
            }
            Ok(Event::Checked(Err(error))) => {
                crate::diagnostics::record_warning("startup update check", &error);
                self.worker = None;
            }
            Ok(Event::Downloaded(result)) => {
                self.downloading = false;
                self.worker = None;
                match result {
                    Ok(prepared) => {
                        self.prepared = Some(prepared);
                        self.message = None;
                    }
                    Err(error) => self.message = Some(error),
                }
            }
            Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) => {
                self.worker = None;
                if self.downloading {
                    self.message = Some("The download stopped. Please try again.".into());
                    self.downloading = false;
                }
            }
        }
    }

    /// True asks the shell to close through the normal unsaved-work prompt.
    pub(crate) fn show(&mut self, ctx: &egui::Context) -> bool {
        self.context = Some(ctx.clone());
        self.poll();
        if self.prepared.is_some() && std::mem::take(&mut self.close_when_ready) {
            return true;
        }
        if !self.visible {
            if self.prepared.is_some() {
                return egui::Area::new(egui::Id::unique("update-ready"))
                    .anchor(egui::Align2::RIGHT_BOTTOM, [-18.0, -38.0])
                    .show(ctx, |ui| {
                        ui.button("Update ready — Restart to install").clicked()
                    })
                    .inner;
            }
            return false;
        }
        let (version, notes) = if let Some(update) = &self.update {
            (update.version.clone(), update.notes_url.clone())
        } else if self.review {
            ("0.2.0".into(), "https://github.com".into())
        } else {
            let mut dismiss = false;
            egui::Modal::new(egui::Id::unique("update-result")).show(ctx, |ui| {
                ui.set_width(410.0);
                ui.heading("Software update");
                if let Some(message) = &self.message {
                    ui.label(message);
                }
                dismiss = ui.button("OK").clicked();
            });
            if dismiss {
                self.visible = false;
                self.message = None;
            }
            return false;
        };
        let mut install = false;
        let mut later = false;
        if self.icon.is_none()
            && let Ok(icon) =
                crate::artwork::load_native_icon(&crate::artwork::original_asset_root())
        {
            self.icon = Some(ctx.load_texture(
                "update-app-icon",
                egui::ColorImage::from_rgba_unmultiplied(
                    [icon.width as usize, icon.height as usize],
                    &icon.rgba,
                ),
                egui::TextureOptions::LINEAR,
            ));
        }
        egui::Modal::new(egui::Id::unique("modeler3d-update")).show(ctx, |ui| {
            ui.set_width(400.0);
            ui.horizontal(|ui| {
                if let Some(icon) = &self.icon {
                    ui.image((icon.id(), egui::vec2(34.0, 34.0)));
                }
                ui.vertical(|ui| {
                    ui.label(egui::RichText::new("3D Modeler").size(16.0));
                    ui.small(format!("v{VERSION} to {version}"));
                });
                ui.label(
                    egui::RichText::new("UPDATE")
                        .small()
                        .background_color(ui.visuals().selection.bg_fill),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    later |= ui.small_button("×").clicked();
                });
            });
            ui.add_space(12.0);
            if let Some(message) = &self.message {
                ui.colored_label(ui.visuals().error_fg_color, message);
            }
            if self.downloading {
                ui.label("Downloading and verifying update…");
                let total = self.total.load(Ordering::Relaxed).max(1);
                let downloaded = self.downloaded.load(Ordering::Relaxed);
                ui.add(egui::ProgressBar::new(downloaded as f32 / total as f32).show_percentage());
                ctx.request_repaint_after(std::time::Duration::from_millis(100));
            } else if self.prepared.is_some() {
                ui.label("The update is ready. Restart to install it.");
                ui.label("You can save your work before closing.");
            } else {
                ui.label(format!(
                    "Version {version} is ready with improvements and fixes."
                ));
            }
            ui.add_space(12.0);
            ui.separator();
            ui.horizontal(|ui| {
                ui.hyperlink_to("What’s new", notes);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let label = if self.prepared.is_some() {
                        "RESTART TO UPDATE"
                    } else if self.installable {
                        "UPDATE AND RESTART"
                    } else {
                        "Download update"
                    };
                    let button = egui::Button::new(
                        egui::RichText::new(label).color(ui.visuals().selection.stroke.color),
                    )
                    .fill(ui.visuals().selection.bg_fill);
                    install = ui.add_enabled(!self.downloading, button).clicked();
                    later |= ui.button("LATER").clicked();
                });
            });
        });
        if later {
            let _ = self.respond("later", &version);
        }
        if !install {
            return false;
        }
        if !self.review && !self.installable {
            ctx.open_url(egui::OpenUrl::new_tab(
                self.update
                    .as_ref()
                    .map(|u| u.notes_url.as_str())
                    .unwrap_or("https://github.com"),
            ));
            self.visible = false;
            return false;
        }
        match self.respond("install", &version) {
            Ok(close) => close,
            Err(error) => {
                self.message = Some(error);
                false
            }
        }
    }

    fn begin_download(&mut self, ctx: &egui::Context) {
        let Some(update) = self.update.clone() else {
            return;
        };
        self.downloading = true;
        self.close_when_ready = true;
        self.message = None;
        let (tx, rx) = mpsc::channel();
        self.worker = Some(rx);
        let downloaded = self.downloaded.clone();
        let total = self.total.clone();
        downloaded.store(0, Ordering::Relaxed);
        total.store(1, Ordering::Relaxed);
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let result = updates::download(&update, |done, size| {
                downloaded.store(done, Ordering::Relaxed);
                total.store(size, Ordering::Relaxed);
            })
            .map_err(|e| e.to_string());
            let _ = tx.send(Event::Downloaded(result));
            ctx.request_repaint();
        });
    }

    pub(crate) fn hide(&mut self) {
        self.visible = false;
    }

    pub(crate) fn status(&mut self) -> serde_json::Value {
        self.poll();
        let version = self
            .update
            .as_ref()
            .map(|u| u.version.as_str())
            .or(if self.review { Some("0.2.0") } else { None });
        let phase = if self.downloading {
            "downloading"
        } else if self.prepared.is_some() {
            "ready"
        } else if version.is_some() {
            "available"
        } else if self.worker.is_some() {
            "checking"
        } else if env!("MODELER3D_UPDATE_REPOSITORY").is_empty() {
            "unconfigured"
        } else {
            "idle"
        };
        serde_json::json!({"current_version":VERSION,"available_version":version,
            "notes_url":self.update.as_ref().map(|u|u.notes_url.as_str()),"phase":phase,
            "downloaded_bytes":self.downloaded.load(Ordering::Relaxed),"total_bytes":self.total.load(Ordering::Relaxed),
            "prompt_open":self.visible,"can_install":self.installable,"message":self.message})
    }

    /// Same consent operation for the card and MCP. Repeated acceptance while
    /// downloading never starts another worker; the selected version must match.
    pub(crate) fn respond(&mut self, response: &str, version: &str) -> Result<bool, String> {
        let status = self.status();
        if status["available_version"].as_str() != Some(version) {
            return Err(
                "The offered update changed or is unavailable; read get_update_status again."
                    .into(),
            );
        }
        if response == "later" {
            self.visible = false;
            self.close_when_ready = false;
            return Ok(false);
        }
        if response != "install" {
            return Err("Response must be install or later".into());
        }
        if !self.installable {
            return Err("This copy requires a manual download from notes_url.".into());
        }
        if self.prepared.is_some() {
            return Ok(true);
        }
        self.close_when_ready = true;
        self.visible = true;
        if !self.downloading && !self.review {
            let ctx = self.context.clone().ok_or("The desktop is not ready")?;
            self.begin_download(&ctx);
        }
        Ok(false)
    }

    pub(crate) fn finish_close(&mut self) -> Result<(), String> {
        if let Some(prepared) = &self.prepared {
            prepared.install_after_exit().map_err(|e| e.to_string())?;
            self.prepared = None;
        }
        Ok(())
    }

    pub(crate) fn review() -> Self {
        Self {
            visible: true,
            review: true,
            installable: true,
            ..Self::default()
        }
    }
}

#[cfg(test)]
mod tests {
    /// OF-75: a build made on any PC carries the workspace version, so About,
    /// MCP and the update check agree and a local build is never offered the
    /// release it is ahead of. Packaging test builds may override it with
    /// `MODELER3D_RELEASE_VERSION`; the test suite never does.
    #[test]
    fn app_version_is_the_workspace_version() {
        assert_eq!(super::VERSION, env!("CARGO_PKG_VERSION"));
        assert_ne!(super::VERSION, "0.1.0");
    }
}
