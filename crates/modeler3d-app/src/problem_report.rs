//! Report a problem (owner request 2026-11-24, modelled on SolidCraft's
//! "Report an Issue"). The person says what went wrong and, if they like, how
//! it could be fixed. The app adds its version, the system and the recent
//! errors from its own log, with the home folder hidden. The person reviews
//! all of it, and it opens as a pre-filled GitHub issue in their browser,
//! where they check it once more and post it. The app itself never sends
//! anything.
//!
//! The link fills the fields of `.github/ISSUE_TEMPLATE/bug.yml` by their
//! ids, so a report from the app looks like one typed on GitHub. A link that
//! would be too long for GitHub leaves the technical details out and they go
//! on the clipboard instead, to paste into their box.

use std::fmt::Write as _;
use std::time::{SystemTime, UNIX_EPOCH};

use eframe::egui::{self, RichText};
use serde_json::{Value, json};

use crate::updates::VERSION;

/// Where reports go.
const ISSUE_URL: &str = "https://github.com/scotttheis/3D-Modeler/issues/new";
/// The issue form the link fills (`.github/ISSUE_TEMPLATE/bug.yml`).
const TEMPLATE: &str = "bug.yml";
/// GitHub turns away a request whose link and headers pass about 7,000
/// bytes; someone not signed in is sent through the sign-in page with the
/// link encoded once more. These keep room for both (as SolidCraft does).
const MAX_URL: usize = 4000;
const SIGN_IN_URL: &str = "https://github.com/login?return_to=";
/// The most each answer keeps, in characters.
pub(crate) const MAX_ANSWER: usize = 2000;
const MAX_TITLE: usize = 90;
/// Log entries listed in a report, and the room each one gets.
const REPORT_LOG_ENTRIES: usize = 15;
const MAX_ENTRY_CHARS: usize = 300;
const MAX_PANIC_CHARS: usize = 1500;

/// What the person wrote, and whether the technical details go with it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct ProblemReport {
    pub(crate) what: String,
    pub(crate) steps: String,
    pub(crate) fix: String,
    pub(crate) include_details: bool,
    /// The collected details, shown to the person before anything leaves.
    pub(crate) details: String,
    /// The error the report started from, used as its title.
    pub(crate) subject: Option<String>,
}

/// The finished report: the browser link, and the whole text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PreparedReport {
    pub(crate) title: String,
    pub(crate) url: String,
    pub(crate) text: String,
    /// False when the details were too long for the link and must be pasted.
    pub(crate) details_in_link: bool,
    /// What to paste into the Technical details box when it is not in the link.
    pub(crate) clipboard: Option<String>,
}

impl ProblemReport {
    /// A new report with freshly collected details. `crash` is an internal
    /// error from the last run, put first.
    #[must_use]
    pub(crate) fn new(crash: Option<&str>) -> Self {
        let mut report = Self {
            include_details: true,
            details: technical_details(&crate::diagnostics::recent_log_text(), now_ms()),
            ..Self::default()
        };
        if let Some(crash) = crash {
            report.what = String::from(
                "3D Modeler hit an internal error the last time it ran \
                 (it may have closed by itself).",
            );
            report.subject = crash.lines().next().map(|line| {
                format!(
                    "Internal error: {}",
                    hide_home(line, home_text().as_deref())
                )
            });
            report.details = format!(
                "Internal error from the last run:\n{}\n\n{}",
                hide_home(
                    &limit_chars(crash, MAX_PANIC_CHARS * 2),
                    home_text().as_deref()
                ),
                report.details
            );
        }
        report
    }

    /// The issue title: the first line of what went wrong.
    #[must_use]
    pub(crate) fn title(&self) -> String {
        let first = self.what.lines().find(|line| !line.trim().is_empty());
        let first = self
            .subject
            .as_deref()
            .or(first)
            .unwrap_or("Problem report")
            .trim();
        limit_chars(first, MAX_TITLE)
    }

    fn details_or_none(&self) -> &str {
        if self.include_details {
            &self.details
        } else {
            "Not included."
        }
    }

    /// The whole report as text, the same sections as the GitHub form.
    #[must_use]
    pub(crate) fn text(&self) -> String {
        let optional = |text: &str| {
            if text.trim().is_empty() {
                String::from("_No response_")
            } else {
                limit_chars(text.trim(), MAX_ANSWER)
            }
        };
        format!(
            "### What happened, and what did you expect?\n\n{}\n\n\
             ### Steps to reproduce\n\n{}\n\n\
             ### How could it be fixed?\n\n{}\n\n\
             ### 3D Modeler version and system\n\n{}\n\n\
             ### Technical details\n\n```text\n{}\n```\n",
            optional(&self.what),
            optional(&self.steps),
            optional(&self.fix),
            version_line(),
            self.details_or_none()
        )
    }

    /// The link to a pre-filled issue and the full text.
    #[must_use]
    pub(crate) fn prepare(&self) -> PreparedReport {
        let title = self.title();
        let answer = |text: &str| limit_chars(text.trim(), MAX_ANSWER);
        let base = format!(
            "{ISSUE_URL}?template={TEMPLATE}&title={}&version={}",
            encode(&title),
            encode(&version_line())
        );
        let mut answers = String::new();
        for (id, text) in [
            ("what", &self.what),
            ("steps", &self.steps),
            ("fix", &self.fix),
        ] {
            if !text.trim().is_empty() {
                let _ = write!(answers, "&{id}={}", encode(&answer(text)));
            }
        }
        let with_details = format!("{base}{answers}&details={}", encode(self.details_or_none()));
        let paste_here = "Too long for the link: 3D Modeler put it on your clipboard. \
                          Paste it here (Ctrl+V).";
        let without_details = format!("{base}{answers}&details={}", encode(paste_here));
        let (url, details_in_link, clipboard) = if fits(&with_details) {
            (with_details, true, None)
        } else if fits(&without_details) {
            (
                without_details,
                false,
                Some(self.details_or_none().to_owned()),
            )
        } else {
            // Even the answers are too long: the whole report goes in the box.
            let url = format!("{base}&details={}", encode(paste_here));
            (url, false, Some(self.text()))
        };
        PreparedReport {
            title,
            url,
            text: self.text(),
            details_in_link,
            clipboard,
        }
    }

    /// The MCP `report_problem` result: the same report the window makes.
    #[must_use]
    pub(crate) fn mcp_value(&self) -> Value {
        let prepared = self.prepare();
        json!({
            "title": prepared.title,
            "url": prepared.url,
            "report": prepared.text,
            "details_in_link": prepared.details_in_link,
            "message": if prepared.details_in_link {
                "Give the person this link. It opens a pre-filled GitHub issue that they review and post themselves. Nothing has been sent."
            } else {
                "Give the person this link and the report text. Part of the report was too long for the link, so they paste it into the Technical details box. Nothing has been sent."
            },
        })
    }
}

fn fits(url: &str) -> bool {
    url.len() <= MAX_URL && sign_in_length(url) + SIGN_IN_URL.len() <= MAX_URL
}

/// "1.0.5 on windows x86_64", the same on every report.
fn version_line() -> String {
    format!(
        "{VERSION} on {} {}",
        std::env::consts::OS,
        std::env::consts::ARCH
    )
}

fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default()
}

/// Version, system, graphics and the recent problems from `log` (the
/// diagnostic log's text), read at `now` (ms since 1970).
fn technical_details(log: &str, now: u128) -> String {
    let home = home_text();
    let home = home.as_deref();
    let mut graphics = None;
    let mut problems = Vec::new();
    for line in log.lines() {
        let mut fields = line.splitn(5, '\t');
        let (Some(at), Some(level), Some(_pid), Some(_thread), Some(message)) = (
            fields.next(),
            fields.next(),
            fields.next(),
            fields.next(),
            fields.next(),
        ) else {
            continue;
        };
        let at = at.parse::<u128>().unwrap_or(now);
        match level {
            "INFO" if message.starts_with("viewport initialized:") => {
                graphics = Some(message.trim_start_matches("viewport initialized:").trim());
            }
            "ERROR" | "WARN" | "PANIC" => problems.push((at, level, message)),
            _ => {}
        }
    }
    let mut out = format!("3D Modeler {}\n", version_line());
    if let Some(graphics) = graphics {
        let _ = writeln!(out, "Graphics: {}", limit_chars(graphics, MAX_ENTRY_CHARS));
    }
    let total = problems.len();
    if total == 0 {
        out.push_str("Recent errors: none recorded\n");
    } else {
        let shown = total.min(REPORT_LOG_ENTRIES);
        let _ = writeln!(out, "Recent errors (last {shown} of {total}):");
        for (at, level, message) in &problems[total - shown..] {
            let room = if *level == "PANIC" {
                MAX_PANIC_CHARS
            } else {
                MAX_ENTRY_CHARS
            };
            let _ = writeln!(
                out,
                "  {} {level}: {}",
                ago(now, *at),
                limit_chars(&hide_home(message, home), room)
            );
        }
    }
    let log = crate::diagnostics::log_file_path();
    let _ = write!(
        out,
        "Full log on the reporter's computer: {}",
        hide_home(&log.to_string_lossy(), home)
    );
    out
}

fn ago(now: u128, at: u128) -> String {
    let seconds = now.saturating_sub(at) / 1000;
    match seconds {
        0..60 => format!("{seconds} s ago"),
        60..3600 => format!("{} min ago", seconds / 60),
        3600..172_800 => format!("{} h ago", seconds / 3600),
        _ => format!("{} days ago", seconds / 86_400),
    }
}

/// The person's home folder, which a report shows as `~`.
fn home_text() -> Option<String> {
    let home = std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" })?;
    let home = home
        .to_string_lossy()
        .trim_end_matches(['/', '\\'])
        .to_owned();
    (home.chars().count() > 3).then_some(home)
}

fn hide_home(text: &str, home: Option<&str>) -> String {
    let Some(home) = home else {
        return text.to_owned();
    };
    let mut text = text.replace(home, "~");
    // Logged paths sometimes use the other slash or escape it.
    let escaped = home.replace('\\', "\\\\");
    if escaped != home {
        text = text.replace(&escaped, "~");
    }
    let forward = home.replace('\\', "/");
    if forward != home {
        text = text.replace(&forward, "~");
    }
    text
}

fn limit_chars(text: &str, limit: usize) -> String {
    if text.chars().count() <= limit {
        return text.to_owned();
    }
    let mut out: String = text.chars().take(limit.saturating_sub(1)).collect();
    out.push('…');
    out
}

/// A query value as GitHub reads it: a space is `+`; `+`, `&`, `#`, `%`,
/// `=`, quotes, line breaks and every byte outside ASCII are encoded.
fn encode(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for byte in value.bytes() {
        if byte == b' ' {
            out.push('+');
        } else if byte.is_ascii_alphanumeric() || b"-_.~!$()*,;:@/?".contains(&byte) {
            out.push(char::from(byte));
        } else {
            let _ = write!(out, "%{byte:02X}");
        }
    }
    out
}

/// The length of `url` once the sign-in page encodes it again.
fn sign_in_length(url: &str) -> usize {
    url.bytes()
        .map(|byte| {
            if byte.is_ascii_alphanumeric() || b"-_.*~".contains(&byte) {
                1
            } else {
                3
            }
        })
        .sum()
}

/// The Report a problem window.
pub(crate) struct ProblemReportWindow {
    report: ProblemReport,
    show_details: bool,
    note: Option<String>,
}

/// What the window asks of the shell after a frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum WindowResult {
    Open,
    Closed,
}

impl ProblemReportWindow {
    /// A window for a new report; `error` is the message the person was
    /// looking at, if they came from one.
    #[must_use]
    pub(crate) fn new(error: Option<&str>, crash: Option<&str>) -> Self {
        let mut report = ProblemReport::new(crash);
        if let Some(error) = error.filter(|error| !error.trim().is_empty()) {
            report.what = format!("3D Modeler showed this error:\n{}\n\n", error.trim());
            report.subject = error.lines().next().map(str::to_owned);
        }
        Self {
            report,
            show_details: false,
            note: None,
        }
    }

    #[cfg(test)]
    pub(crate) fn report(&self) -> &ProblemReport {
        &self.report
    }

    pub(crate) fn show(&mut self, ctx: &egui::Context) -> WindowResult {
        let mut result = WindowResult::Open;
        let mut send = false;
        let mut copy = false;
        egui::Modal::new(egui::Id::unique("modeler3d-report-problem")).show(ctx, |ui| {
            ui.set_width(460.0);
            ui.heading("Report a problem");
            ui.label(
                RichText::new(
                    "This opens a ready-made report on GitHub (a free account is needed). \
                     You can check and change it there before posting. \
                     3D Modeler sends nothing by itself.",
                )
                .small()
                .weak(),
            );
            ui.add_space(8.0);
            answer(ui, "What went wrong?", &mut self.report.what, 4);
            answer(
                ui,
                "What were you doing when it happened? (optional)",
                &mut self.report.steps,
                3,
            );
            answer(
                ui,
                "Any idea how it could be fixed? (optional)",
                &mut self.report.fix,
                3,
            );
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.checkbox(
                    &mut self.report.include_details,
                    "Include technical details",
                )
                .on_hover_text("Version, system, graphics and recent errors. No models or files.");
                let label = if self.show_details { "Hide" } else { "Show" };
                if ui.small_button(label).clicked() {
                    self.show_details = !self.show_details;
                }
            });
            if self.show_details {
                egui::ScrollArea::vertical()
                    .max_height(160.0)
                    .show(ui, |ui| {
                        ui.add(
                            egui::Label::new(
                                RichText::new(&self.report.details).small().monospace(),
                            )
                            .selectable(true),
                        );
                    });
            }
            if let Some(note) = &self.note {
                ui.add_space(4.0);
                ui.label(RichText::new(note).small());
            }
            ui.add_space(8.0);
            ui.separator();
            ui.horizontal(|ui| {
                let ready = !self.report.what.trim().is_empty();
                send = ui
                    .add_enabled(ready, egui::Button::new("Open on GitHub"))
                    .on_disabled_hover_text("Say what went wrong first.")
                    .clicked();
                copy = ui
                    .add_enabled(ready, egui::Button::new("Copy report"))
                    .on_hover_text("Copy the report to paste somewhere else.")
                    .clicked();
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Close").clicked() {
                        result = WindowResult::Closed;
                    }
                });
            });
        });
        if ctx.input(|input| input.key_pressed(egui::Key::Escape)) {
            result = WindowResult::Closed;
        }
        if send {
            let prepared = self.report.prepare();
            if let Some(clipboard) = &prepared.clipboard {
                ctx.copy_text(clipboard.clone());
                self.note = Some(String::from(
                    "Opened in your browser. Part of the report was too long for the link, so \
                     it is on your clipboard: paste it into the Technical details box.",
                ));
            } else {
                self.note = Some(String::from(
                    "Opened in your browser. Check the report there and press Create.",
                ));
            }
            crate::diagnostics::record_info("problem report", "opened on GitHub");
            ctx.open_url(egui::OpenUrl::new_tab(prepared.url));
        }
        if copy {
            ctx.copy_text(self.report.text());
            self.note = Some(String::from("The report is on your clipboard."));
        }
        result
    }
}

fn answer(ui: &mut egui::Ui, label: &str, text: &mut String, rows: usize) {
    ui.label(label);
    ui.add(
        egui::TextEdit::multiline(text)
            .desired_rows(rows)
            .desired_width(f32::INFINITY)
            .char_limit(MAX_ANSWER),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn query(url: &str) -> Vec<(String, String)> {
        let query = url.split_once('?').map_or("", |(_, query)| query);
        query
            .split('&')
            .filter_map(|pair| pair.split_once('='))
            .map(|(key, value)| (key.to_owned(), decode(value)))
            .collect()
    }

    fn decode(value: &str) -> String {
        let bytes = value.as_bytes();
        let mut out = Vec::new();
        let mut index = 0;
        while index < bytes.len() {
            match bytes[index] {
                b'+' => out.push(b' '),
                b'%' => {
                    let hex = std::str::from_utf8(&bytes[index + 1..index + 3]).unwrap_or("00");
                    out.push(u8::from_str_radix(hex, 16).unwrap_or(0));
                    index += 2;
                }
                byte => out.push(byte),
            }
            index += 1;
        }
        String::from_utf8_lossy(&out).into_owned()
    }

    fn report(what: &str, details: &str) -> ProblemReport {
        ProblemReport {
            what: what.to_owned(),
            steps: String::from("1. Add a cube\n2. Hollow it"),
            fix: String::from("Check the wall thickness first & say so."),
            include_details: true,
            details: details.to_owned(),
            subject: None,
        }
    }

    #[test]
    fn a_report_link_fills_the_github_form_fields_exactly() {
        let report = report(
            "Hollow froze\nthen nothing",
            "3D Modeler 1.0\nRecent errors: none",
        );
        let prepared = report.prepare();
        assert!(prepared.details_in_link);
        assert!(prepared.url.starts_with(ISSUE_URL));
        let fields = query(&prepared.url);
        let get = |key: &str| {
            fields
                .iter()
                .find(|(name, _)| name == key)
                .map(|(_, value)| value.clone())
        };
        assert_eq!(get("template").as_deref(), Some("bug.yml"));
        assert_eq!(get("title").as_deref(), Some("Hollow froze"));
        assert_eq!(get("what").as_deref(), Some("Hollow froze\nthen nothing"));
        assert_eq!(get("steps").as_deref(), Some("1. Add a cube\n2. Hollow it"));
        assert_eq!(
            get("fix").as_deref(),
            Some("Check the wall thickness first & say so.")
        );
        assert_eq!(
            get("details").as_deref(),
            Some("3D Modeler 1.0\nRecent errors: none")
        );
        assert!(get("version").is_some_and(|version| version.starts_with(VERSION)));
    }

    #[test]
    fn a_long_report_keeps_the_answers_and_moves_the_details_to_the_clipboard() {
        let details = "x".repeat(10_000);
        let prepared = report("Broken", &details).prepare();
        assert!(!prepared.details_in_link);
        assert!(fits(&prepared.url));
        assert!(prepared.url.contains("what=Broken"));
        assert!(prepared.text.contains(&details));
        assert_eq!(prepared.clipboard.as_deref(), Some(details.as_str()));

        let huge = "ü".repeat(MAX_ANSWER);
        let prepared = report(&huge, &details).prepare();
        assert!(fits(&prepared.url));
        assert!(!prepared.details_in_link);
        assert_eq!(prepared.clipboard, Some(prepared.text.clone()));
    }

    #[test]
    fn details_can_be_left_out() {
        let mut report = report("Broken", "secret-ish details");
        report.include_details = false;
        let prepared = report.prepare();
        assert!(!prepared.url.contains("secret"));
        assert!(!prepared.text.contains("secret"));
    }

    #[test]
    fn technical_details_list_recent_problems_and_the_graphics_adapter() {
        let now = 10_000_000_u128;
        let log = format!(
            "{}\tINFO\tpid=1\tthread=main\tviewport initialized: adapter=Quadro / Dx12 / DiscreteGpu\n\
             {}\tERROR\tpid=1\tthread=main\tcommand failed: Hollow requires a selected object\n\
             {}\tINFO\tpid=1\tthread=main\tdesktop job started: Hollow\n\
             garbage line\n\
             {}\tPANIC\tpid=1\tthread=main\tunhandled panic: boom at src/x.rs:1",
            now - 5_000,
            now - 120_000,
            now - 1_000,
            now - 500
        );
        let details = technical_details(&log, now);
        assert!(details.contains("Graphics: adapter=Quadro / Dx12 / DiscreteGpu"));
        assert!(details.contains("Recent errors (last 2 of 2):"));
        assert!(
            details.contains("2 min ago ERROR: command failed: Hollow requires a selected object")
        );
        assert!(details.contains("0 s ago PANIC: unhandled panic: boom"));
        assert!(!details.contains("desktop job started"));
        assert!(technical_details("", now).contains("Recent errors: none recorded"));
    }

    #[test]
    fn reports_hide_the_home_folder_and_bound_each_entry() {
        assert_eq!(
            hide_home(
                r"C:\Users\pat\Models\a.stl and C:/Users/pat/b",
                Some(r"C:\Users\pat")
            ),
            r"~\Models\a.stl and ~/b"
        );
        let long = "y".repeat(5000);
        assert_eq!(limit_chars(&long, 300).chars().count(), 300);
        let now = 1_000_000;
        let many = (0..40)
            .map(|index| format!("{}\tWARN\tpid=1\tthread=t\twarning {index}", now - 10))
            .collect::<Vec<_>>()
            .join("\n");
        let details = technical_details(&many, now);
        assert!(details.contains("last 15 of 40"));
        assert!(details.contains("warning 39"));
        assert!(!details.contains("warning 24\n"));
    }

    #[test]
    fn the_title_is_the_first_line_and_has_a_fallback() {
        assert_eq!(
            report("\n  Export lost colours \nmore", "").title(),
            "Export lost colours"
        );
        assert_eq!(report("", "").title(), "Problem report");
        assert_eq!(
            report(&"a".repeat(500), "").title().chars().count(),
            MAX_TITLE
        );
    }

    #[test]
    fn an_error_message_and_a_crash_prefill_the_window() {
        let window = ProblemReportWindow::new(Some("Repair failed: no mesh"), None);
        assert!(window.report().what.contains("Repair failed: no mesh"));
        assert_eq!(window.report().title(), "Repair failed: no mesh");
        assert!(window.report().include_details);
        let window = ProblemReportWindow::new(None, Some("boom at src/x.rs:1"));
        assert!(window.report().what.contains("internal error"));
        assert!(
            window
                .report()
                .details
                .starts_with("Internal error from the last run:\nboom")
        );
    }

    #[test]
    fn the_mcp_result_carries_the_same_link_and_text() {
        let report = report("Broken", "details");
        let value = report.mcp_value();
        let prepared = report.prepare();
        assert_eq!(value["url"], json!(prepared.url));
        assert_eq!(value["report"], json!(prepared.text));
        assert_eq!(value["details_in_link"], json!(true));
    }
}
