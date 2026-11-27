//! A window in front of `victory.exe`.
//!
//! The command-line side of this port has four ways in (three `.cmd` files and
//! `scripts\play.ps1`) and about forty switches. This is the other half of the
//! preset idea: one window that asks for the two things a person actually
//! knows - where the dump is, and whether they want it to look right or run
//! fast - and then runs the same `victory run --profile` those files run.
//!
//! It does nothing the binary cannot do: every button is a command line, and
//! every line of output is the binary's own. That is deliberate. A launcher
//! that quietly does something else is what the presets exist to end.

use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use eframe::egui;

const PROFILES: [&str; 6] = [
    "balanced",
    "speed",
    "quality",
    "lowvram",
    "pareto-residency",
    "pareto-residency-amd",
];

const PROFILE_HINT: [&str; 6] = [
    "1080p, two thirds scale, 60 Hz tick - start here",
    "936p, no frame cap, every port the card allows",
    "1440p native with ray tracing, about a third slower",
    "720p Series S profile, for 4 GB cards",
    "what Play-VIctory.cmd has always set",
    "the Radeon set: recorded lists, all four ports",
];

const VENDORS: [&str; 3] = ["other", "nvidia", "amd"];

/// Lines from the child process, and how it ended.
#[derive(Default)]
struct Output {
    lines: Vec<String>,
    exited: Option<i32>,
}

struct App {
    game_dir: Option<PathBuf>,
    dir_text: String,
    profile: usize,
    vendor: usize,
    runner: Option<PathBuf>,
    output: Arc<Mutex<Output>>,
    child: Option<Child>,
    running: Arc<AtomicBool>,
    note: Option<String>,
}

impl Default for App {
    fn default() -> Self {
        let found = find_game_dir();
        let runner = find_runner();
        let note = match (&found, &runner) {
            (None, _) => Some("No game folder yet: drop it on the window, or paste it.".to_owned()),
            (_, None) => Some(
                "victory.exe is not beside this window: build it (cargo build --release -p victory) \
                 or set VICTORY_BIN."
                    .to_owned(),
            ),
            (Some(_), Some(_)) => None,
        };
        Self {
            dir_text: found
                .as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or_default(),
            game_dir: found,
            profile: 0,
            vendor: 0,
            runner,
            output: Arc::new(Mutex::new(Output::default())),
            child: None,
            running: Arc::new(AtomicBool::new(false)),
            note,
        }
    }
}

/// The folder the last run used, beside the window, or one level up: the same
/// order the PowerShell launcher uses, minus the search across the drives.
fn find_game_dir() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("VICTORY_GAME_DIR") {
        let p = PathBuf::from(p.trim().trim_matches('"'));
        if is_game_dir(&p) {
            return Some(p);
        }
    }
    let exe = std::env::current_exe().ok()?;
    let dir = exe.parent()?;
    for base in [dir, dir.parent()?, dir.parent()?.parent()?] {
        if is_game_dir(base) {
            return Some(base.to_path_buf());
        }
        let Ok(entries) = std::fs::read_dir(base) else {
            continue;
        };
        for sibling in entries.flatten() {
            let p = sibling.path();
            if p.is_dir() && is_game_dir(&p) {
                return Some(p);
            }
        }
    }
    None
}

fn is_game_dir(path: &Path) -> bool {
    path.join("game_scarlett_master.exe").is_file()
}

/// `victory.exe` next to this window, or the release build beside the debug one.
fn find_runner() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("VICTORY_BIN") {
        let p = PathBuf::from(p.trim().trim_matches('"'));
        if p.is_file() {
            return Some(p);
        }
    }
    let exe = std::env::current_exe().ok()?;
    let dir = exe.parent()?;
    let candidates = [
        dir.join("victory.exe"),
        dir.join("../release/victory.exe"),
        dir.join("../../release/victory.exe"),
    ];
    candidates.into_iter().find(|p| p.is_file())
}

impl App {
    /// Start `victory` with these arguments and put its output in the log pane.
    fn launch(&mut self, ctx: &egui::Context, args: Vec<String>, label: &str) {
        let Some(runner) = self.runner.clone() else {
            self.note = Some("victory.exe not found".to_owned());
            return;
        };
        {
            let mut out = self.output.lock().expect("log lock");
            out.lines.clear();
            out.exited = None;
            out.lines.push(format!("> victory {label}"));
        }
        let mut command = Command::new(&runner);
        command
            .args(&args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(Stdio::null());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            command.creation_flags(CREATE_NO_WINDOW);
        }
        let mut child = match command.spawn() {
            Ok(child) => child,
            Err(e) => {
                self.note = Some(format!("could not start {}: {e}", runner.display()));
                return;
            }
        };

        let output = Arc::clone(&self.output);
        for (stream, tail) in [
            (child.stdout.take().map(|s| Box::new(s) as Box<dyn std::io::Read + Send>), "out"),
            (child.stderr.take().map(|s| Box::new(s) as Box<dyn std::io::Read + Send>), "err"),
        ] {
            let Some(stream) = stream else { continue };
            let output = Arc::clone(&output);
            let ctx = ctx.clone();
            std::thread::spawn(move || {
                for line in BufReader::new(stream).lines().map_while(Result::ok) {
                    let mut out = output.lock().expect("log lock");
                    if out.lines.len() > 800 {
                        out.lines.drain(..200);
                    }
                    // stdout is where the port narrates; stderr carries the
                    // warnings and the errors, so it is marked.
                    if tail == "err" {
                        out.lines.push(format!("! {line}"));
                    } else {
                        out.lines.push(line);
                    }
                    drop(out);
                    ctx.request_repaint();
                }
            });
        }

        self.running.store(true, Ordering::Release);
        self.child = Some(child);
        self.note = None;
        ctx.request_repaint();
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // The child, waited for without blocking the window.
        if let Some(child) = self.child.as_mut()
            && let Ok(Some(status)) = child.try_wait()
        {
            let code = status.code().unwrap_or(-1);
            let mut out = self.output.lock().expect("log lock");
            out.exited = Some(code);
            out.lines.push(format!("[exit {code}]"));
            drop(out);
            self.child = None;
            self.running.store(false, Ordering::Release);
        }

        // A folder dropped on the window is the one thing a first run needs.
        let dropped: Vec<PathBuf> = ctx.input(|i| {
            i.raw
                .dropped_files
                .iter()
                .filter_map(|f| f.path.clone())
                .collect()
        });
        for path in dropped {
            if path.is_dir() && is_game_dir(&path) {
                self.dir_text = path.display().to_string();
                self.game_dir = Some(path);
                self.note = None;
            } else if path.is_file()
                && path.file_name().is_some_and(|n| n == "game_scarlett_master.exe")
                && let Some(parent) = path.parent()
            {
                self.dir_text = parent.display().to_string();
                self.game_dir = Some(parent.to_path_buf());
                self.note = None;
            }
        }

        egui::TopBottomPanel::top("head").show(ctx, |ui| {
            ui.add_space(6.0);
            ui.heading("VIctory");
            ui.label(
                egui::RichText::new(
                    "An Xbox Series title on this PC. Pick the folder once; the preset stays.",
                )
                .weak(),
            );
            ui.add_space(6.0);
        });

        egui::TopBottomPanel::bottom("foot").show(ctx, |ui| {
            ui.add_space(4.0);
            let busy = self.running.load(Ordering::Acquire);
            ui.horizontal(|ui| {
                let start = ui.add_enabled(
                    !busy && self.game_dir.is_some() && self.runner.is_some(),
                    egui::Button::new(egui::RichText::new("Start").size(15.0)),
                );
                if start.clicked()
                    && let Some(dir) = self.game_dir.clone()
                {
                    let exe = dir.join("game_scarlett_master.exe");
                    self.launch(
                        ctx,
                        vec![
                            "run".to_owned(),
                            exe.display().to_string(),
                            "--profile".to_owned(),
                            PROFILES[self.profile].to_owned(),
                            "--vendor".to_owned(),
                            VENDORS[self.vendor].to_owned(),
                            "--execute".to_owned(),
                        ],
                        "run",
                    );
                }
                if ui
                    .add_enabled(!busy && self.game_dir.is_some(), egui::Button::new("Prewarm shaders"))
                    .on_hover_text(
                        "victory shaders <folder>: translate every shader into the cache first, \
                         instead of during the first hours of play",
                    )
                    .clicked()
                    && let Some(dir) = self.game_dir.clone()
                {
                    self.launch(
                        ctx,
                        vec!["shaders".to_owned(), dir.display().to_string()],
                        "shaders",
                    );
                }
                if ui
                    .add_enabled(!busy, egui::Button::new("Which card?"))
                    .on_hover_text("victory gpu-info: the adapter a run would use, in DXGI's performance order")
                    .clicked()
                {
                    self.launch(ctx, vec!["gpu-info".to_owned()], "gpu-info");
                }
                if ui
                    .add_enabled(!busy, egui::Button::new("Config"))
                    .on_hover_text("victory config: where the cache and the data go")
                    .clicked()
                {
                    self.launch(ctx, vec!["config".to_owned()], "config");
                }
                if busy {
                    ui.spinner();
                    ui.label("running");
                }
                if let Some(exit) = self.output.lock().expect("log lock").exited {
                    ui.label(egui::RichText::new(format!("exit {exit}")).weak());
                }
            });
            ui.add_space(4.0);
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Game folder");
                let width = (ui.available_width() - 90.0).max(200.0);
                ui.add_sized([width, 22.0], egui::TextEdit::singleline(&mut self.dir_text));
                if ui.button("Use").clicked() {
                    let p = PathBuf::from(self.dir_text.trim().trim_matches('"'));
                    if is_game_dir(&p) {
                        self.game_dir = Some(p);
                        self.note = None;
                    } else {
                        self.note = Some(format!(
                            "no game_scarlett_master.exe in {}",
                            p.display()
                        ));
                    }
                }
            });
            ui.add_space(2.0);
            ui.label(egui::RichText::new("or drop the folder on this window").weak());

            if let Some(note) = self.note.clone() {
                ui.add_space(4.0);
                ui.colored_label(egui::Color32::from_rgb(0xC0, 0x50, 0x40), note);
            }

            ui.add_space(10.0);
            ui.separator();
            ui.label(egui::RichText::new("Preset").strong());
            for (index, name) in PROFILES.iter().enumerate() {
                ui.horizontal(|ui| {
                    if ui
                        .radio_value(&mut self.profile, index, *name)
                        .changed()
                    {
                        self.note = None;
                    }
                    ui.label(egui::RichText::new(PROFILE_HINT[index]).weak());
                });
            }

            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.label("Card");
                for (index, vendor) in VENDORS.iter().enumerate() {
                    ui.radio_value(&mut self.vendor, index, *vendor);
                }
                ui.label(
                    egui::RichText::new("balanced and speed only: amd turns recorded lists on")
                        .weak(),
                );
            });

            ui.add_space(10.0);
            ui.separator();
            ui.label(egui::RichText::new("Output").strong());
            egui::ScrollArea::vertical()
                .stick_to_bottom(true)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    let lines = self.output.lock().expect("log lock").lines.clone();
                    for line in lines {
                        let color = if line.starts_with('!') {
                            egui::Color32::from_rgb(0xC0, 0x50, 0x40)
                        } else if line.starts_with("[exit") {
                            egui::Color32::from_rgb(0x60, 0x90, 0x50)
                        } else {
                            ui.visuals().text_color()
                        };
                        ui.label(egui::RichText::new(line).monospace().color(color));
                    }
                });
        });
    }
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([900.0, 640.0])
            .with_min_inner_size([680.0, 460.0])
            .with_title("VIctory"),
        ..Default::default()
    };
    eframe::run_native(
        "VIctory",
        options,
        Box::new(|_cc| Ok(Box::new(App::default()))),
    )
}
