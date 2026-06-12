use eframe::egui;
use snake_core::BoundaryMode;

use crate::game_view::{GameSession, SessionEvent};
use crate::settings::{Settings, SizePreset, Speed};

enum Screen {
    Menu,
    Game(GameSession),
}

/// Top-level eframe application: menu ↔ running game.
pub struct App {
    settings: Settings,
    screen: Screen,
    /// Seed fixed via `?seed=` query parameter (web) or `--seed` (native);
    /// `None` means every game gets a fresh time-derived seed.
    seed_override: Option<u64>,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        Self {
            settings: Settings::default(),
            screen: Screen::Menu,
            seed_override: seed_override(cc),
        }
    }

    fn next_seed(&self) -> u64 {
        self.seed_override.unwrap_or_else(entropy_seed)
    }

    fn menu_ui(ui: &mut egui::Ui, settings: &mut Settings, seed_override: Option<u64>) -> bool {
        let mut start = false;
        ui.vertical_centered(|ui| {
            ui.add_space(40.0);
            ui.heading(egui::RichText::new("HexSnake").size(48.0));
            ui.add_space(24.0);

            egui::Grid::new("menu_options")
                .num_columns(2)
                .spacing([16.0, 12.0])
                .show(ui, |ui| {
                    ui.label("Randbedingung:");
                    ui.horizontal(|ui| {
                        ui.selectable_value(&mut settings.boundary, BoundaryMode::Walls, "Wände");
                        ui.selectable_value(
                            &mut settings.boundary,
                            BoundaryMode::Periodic,
                            "Periodisch (Torus)",
                        );
                    });
                    ui.end_row();

                    ui.label("Feldgröße:");
                    egui::ComboBox::from_id_salt("size_preset")
                        .selected_text(settings.preset.label())
                        .show_ui(ui, |ui| {
                            for preset in SizePreset::ALL {
                                ui.selectable_value(&mut settings.preset, preset, preset.label());
                            }
                        });
                    ui.end_row();

                    if settings.preset == SizePreset::Custom {
                        ui.label("Breite × Höhe:");
                        ui.horizontal(|ui| {
                            ui.add(egui::DragValue::new(&mut settings.custom_width).range(6..=48));
                            ui.label("×");
                            ui.add(egui::DragValue::new(&mut settings.custom_height).range(6..=36));
                        });
                        ui.end_row();
                    }

                    ui.label("Geschwindigkeit:");
                    ui.horizontal(|ui| {
                        for speed in Speed::ALL {
                            ui.selectable_value(&mut settings.speed, speed, speed.label());
                        }
                    });
                    ui.end_row();
                });

            ui.add_space(24.0);
            let button = egui::Button::new(egui::RichText::new("Start").size(24.0))
                .min_size(egui::Vec2::new(160.0, 48.0));
            if ui.add(button).clicked() || ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                start = true;
            }
            ui.add_space(12.0);
            ui.weak("Steuerung: Q W E / A S D — sechs Richtungen auf dem Hexgitter");
            if let Some(seed) = seed_override {
                ui.weak(format!("Seed fixiert: {seed}"));
            }
        });
        start
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::Frame::central_panel(ui.style()).show(ui, |ui| {
            ui.set_min_size(ui.available_size());
            let action = match &mut self.screen {
                Screen::Menu => {
                    if Self::menu_ui(ui, &mut self.settings, self.seed_override) {
                        SessionEvent::Restart
                    } else {
                        SessionEvent::Continue
                    }
                }
                Screen::Game(session) => session.ui(ui),
            };
            match action {
                SessionEvent::Continue => {}
                SessionEvent::Restart => {
                    self.screen = Screen::Game(GameSession::new(&self.settings, self.next_seed()));
                }
                SessionEvent::BackToMenu => self.screen = Screen::Menu,
            }
        });
    }
}

/// A non-deterministic seed for normal play. Wall-clock use is fine here in
/// the app crate — only `snake-core` itself must stay clock-free.
#[cfg(not(target_arch = "wasm32"))]
fn entropy_seed() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0)
}

#[cfg(target_arch = "wasm32")]
fn entropy_seed() -> u64 {
    eframe::web_sys::js_sys::Date::now() as u64
}

/// Fixed seed for reproducible runs: `?seed=42` in the browser URL,
/// `--seed 42` (or `--seed=42`) on the command line.
#[cfg(target_arch = "wasm32")]
fn seed_override(cc: &eframe::CreationContext<'_>) -> Option<u64> {
    let query_map = &cc.integration_info.web_info.location.query_map;
    query_map
        .get("seed")
        .and_then(|values| values.first())
        .and_then(|v| v.parse().ok())
}

#[cfg(not(target_arch = "wasm32"))]
fn seed_override(_cc: &eframe::CreationContext<'_>) -> Option<u64> {
    let args: Vec<String> = std::env::args().collect();
    for (i, arg) in args.iter().enumerate() {
        if let Some(value) = arg.strip_prefix("--seed=") {
            return value.parse().ok();
        }
        if arg == "--seed" {
            return args.get(i + 1).and_then(|v| v.parse().ok());
        }
    }
    None
}
