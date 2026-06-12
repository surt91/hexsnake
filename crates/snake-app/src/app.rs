use eframe::egui;
use snake_core::{BoundaryMode, Direction, Status};

use crate::game_view::{GameSession, SessionEvent};
use crate::highscores::{self, Entry, Highscores};
use crate::seed::{encode_seed, parse_seed, random_seed};
use crate::settings::{Settings, SizePreset, Speed, StrategyChoice, HAMILTON_INCOMPATIBLE_HINT};
use crate::theme::ThemeId;

enum Screen {
    Menu,
    Game(Box<GameSession>),
}

/// Top-level eframe application: menu ↔ running game.
pub struct App {
    settings: Settings,
    screen: Screen,
    /// Seed fixed via `?seed=` query parameter (web) or `--seed` (native);
    /// `None` means every game gets a fresh time-derived seed.
    seed_override: Option<u64>,
    /// Scripted inputs from `?inputs=`/`--inputs` (debug feature): key
    /// characters w/e/d/s/a/q, applied one per tick in every new game.
    input_script: Vec<Direction>,
    highscores: Highscores,
    player_name: String,
    /// Whether the highscore of the current game-over screen was already
    /// recorded (or did not qualify).
    highscore_done: bool,
    /// One-shot focus grab for the name dialog. Re-requesting focus every
    /// frame would cancel the Enter-submits-the-field detection
    /// (`lost_focus` is false once focus is re-acquired).
    name_focus_grabbed: bool,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let (settings, highscores, player_name) = match cc.storage {
            Some(storage) => (
                eframe::get_value(storage, "settings").unwrap_or_default(),
                eframe::get_value(storage, "highscores").unwrap_or_default(),
                eframe::get_value(storage, "player_name").unwrap_or_default(),
            ),
            None => Default::default(),
        };
        Self {
            settings,
            screen: Screen::Menu,
            seed_override: seed_override(cc),
            input_script: parse_input_script(&input_script_arg(cc).unwrap_or_default()),
            highscores,
            player_name,
            highscore_done: false,
            name_focus_grabbed: false,
        }
    }

    fn next_seed(&self) -> u64 {
        self.seed_override.unwrap_or_else(random_seed)
    }

    fn start_game(&mut self) {
        self.highscore_done = false;
        self.name_focus_grabbed = false;
        self.screen = Screen::Game(Box::new(GameSession::new(
            &self.settings,
            self.next_seed(),
            &self.input_script,
        )));
    }

    fn menu_ui(&mut self, ui: &mut egui::Ui) -> bool {
        let mut start = false;
        let settings = &mut self.settings;
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

                    ui.label("Autopilot:");
                    let (width, height) = settings.dimensions();
                    egui::ComboBox::from_id_salt("strategy")
                        .selected_text(settings.strategy.label())
                        .show_ui(ui, |ui| {
                            for choice in StrategyChoice::ALL {
                                let enabled = choice.compatible_with(width, height);
                                let label = egui::Button::selectable(
                                    settings.strategy == choice,
                                    choice.label(),
                                );
                                let response = ui
                                    .add_enabled(enabled, label)
                                    .on_disabled_hover_text(HAMILTON_INCOMPATIBLE_HINT);
                                if response.clicked() {
                                    settings.strategy = choice;
                                }
                            }
                        });
                    // A size change can strand an incompatible selection;
                    // fall back to manual play.
                    if !settings.strategy.compatible_with(width, height) {
                        settings.strategy = StrategyChoice::Human;
                    }
                    ui.end_row();

                    ui.label("Theme:");
                    egui::ComboBox::from_id_salt("theme")
                        .selected_text(settings.theme.label())
                        .show_ui(ui, |ui| {
                            for theme in ThemeId::ALL {
                                ui.selectable_value(&mut settings.theme, theme, theme.label());
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
            if let Some(seed) = self.seed_override {
                ui.weak(format!("Seed fixiert: {}", encode_seed(seed)));
            }

            let key = highscores::mode_key(settings);
            let entries = self.highscores.table(&key);
            if !entries.is_empty() {
                ui.add_space(20.0);
                ui.heading("Highscores");
                ui.weak(format!("Modus: {key}"));
                ui.add_space(4.0);
                highscore_table(ui, entries);
            }
        });
        start
    }

    fn name_dialog(&mut self, ctx: &egui::Context, key: &str, score: u32) {
        egui::Window::new("Neuer Highscore!")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, -140.0])
            .show(ctx, |ui| {
                ui.label(format!("Score {score} — trag deinen Namen ein:"));
                let response = ui.text_edit_singleline(&mut self.player_name);
                if !self.name_focus_grabbed {
                    response.request_focus();
                    self.name_focus_grabbed = true;
                }
                let submitted =
                    response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                ui.horizontal(|ui| {
                    if ui.button("Speichern").clicked() || submitted {
                        let name = match self.player_name.trim() {
                            "" => "Anonym".to_owned(),
                            trimmed => trimmed.to_owned(),
                        };
                        self.highscores.insert(
                            key,
                            Entry {
                                name,
                                score,
                                date: highscores::today_iso(),
                            },
                        );
                        self.highscore_done = true;
                    }
                    if ui.button("Verwerfen").clicked() {
                        self.highscore_done = true;
                    }
                });
            });
    }

    fn highscore_window(&self, ctx: &egui::Context, key: &str) {
        egui::Window::new("Highscores")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 150.0])
            .show(ctx, |ui| {
                let entries = self.highscores.table(key);
                if entries.is_empty() {
                    ui.label("Noch keine Einträge in diesem Modus.");
                } else {
                    highscore_table(ui, entries);
                }
            });
    }
}

fn highscore_table(ui: &mut egui::Ui, entries: &[Entry]) {
    egui::Grid::new("highscore_table")
        .num_columns(4)
        .spacing([24.0, 4.0])
        .striped(true)
        .show(ui, |ui| {
            for (rank, entry) in entries.iter().enumerate() {
                ui.label(format!("{}.", rank + 1));
                ui.label(&entry.name);
                ui.label(entry.score.to_string());
                ui.weak(&entry.date);
                ui.end_row();
            }
        });
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::Frame::central_panel(ui.style()).show(ui, |ui| {
            ui.set_min_size(ui.available_size());

            // Game-over bookkeeping happens outside the borrow of `screen`.
            let mut game_over_score = None;
            let action = match &mut self.screen {
                Screen::Menu => {
                    if self.menu_ui(ui) {
                        SessionEvent::Restart
                    } else {
                        SessionEvent::Continue
                    }
                }
                Screen::Game(session) => {
                    let state = session.game_state();
                    let finished = matches!(state.status(), Status::GameOver | Status::Won);
                    let score = state.score();
                    // Runs are filed under the slowest speed used mid-game.
                    let key =
                        highscores::mode_key_with_speed(&self.settings, session.slowest_speed());
                    // Autopilot runs are excluded from the highscores.
                    let dialog_open = finished
                        && !self.highscore_done
                        && !session.autopilot_used()
                        && self.highscores.qualifies(&key, score);
                    if finished {
                        game_over_score = Some((score, dialog_open, key));
                    }
                    session.ui(ui, dialog_open)
                }
            };

            if let Some((score, dialog_open, key)) = game_over_score {
                if dialog_open {
                    self.name_dialog(ui.ctx(), &key, score);
                } else {
                    self.highscore_done = true;
                }
                self.highscore_window(ui.ctx(), &key);
            }

            match action {
                SessionEvent::Continue => {}
                SessionEvent::Restart => self.start_game(),
                SessionEvent::BackToMenu => self.screen = Screen::Menu,
            }
        });
    }

    /// Save early and often: on the web target, persistence only happens
    /// on this interval — a closed tab in between loses the data.
    fn auto_save_interval(&self) -> std::time::Duration {
        std::time::Duration::from_secs(5)
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, "settings", &self.settings);
        eframe::set_value(storage, "highscores", &self.highscores);
        eframe::set_value(storage, "player_name", &self.player_name);
    }
}

fn parse_input_script(script: &str) -> Vec<Direction> {
    script
        .chars()
        .filter_map(|c| match c.to_ascii_lowercase() {
            'w' => Some(Direction::North),
            'e' => Some(Direction::NorthEast),
            'd' => Some(Direction::SouthEast),
            's' => Some(Direction::South),
            'a' => Some(Direction::SouthWest),
            'q' => Some(Direction::NorthWest),
            _ => None,
        })
        .collect()
}

#[cfg(target_arch = "wasm32")]
fn input_script_arg(cc: &eframe::CreationContext<'_>) -> Option<String> {
    let query_map = &cc.integration_info.web_info.location.query_map;
    query_map
        .get("inputs")
        .and_then(|values| values.first())
        .cloned()
}

#[cfg(not(target_arch = "wasm32"))]
fn input_script_arg(_cc: &eframe::CreationContext<'_>) -> Option<String> {
    let args: Vec<String> = std::env::args().collect();
    for (i, arg) in args.iter().enumerate() {
        if let Some(value) = arg.strip_prefix("--inputs=") {
            return Some(value.to_owned());
        }
        if arg == "--inputs" {
            return args.get(i + 1).cloned();
        }
    }
    None
}

/// Fixed seed for reproducible runs: `?seed=42` or `?seed=AAAAAq` in the
/// browser URL, `--seed 42` (or `--seed=…`) on the command line. Decimal
/// and compact base64 forms are both accepted.
#[cfg(target_arch = "wasm32")]
fn seed_override(cc: &eframe::CreationContext<'_>) -> Option<u64> {
    let query_map = &cc.integration_info.web_info.location.query_map;
    query_map
        .get("seed")
        .and_then(|values| values.first())
        .and_then(|v| parse_seed(v))
}

#[cfg(not(target_arch = "wasm32"))]
fn seed_override(_cc: &eframe::CreationContext<'_>) -> Option<u64> {
    let args: Vec<String> = std::env::args().collect();
    for (i, arg) in args.iter().enumerate() {
        if let Some(value) = arg.strip_prefix("--seed=") {
            return parse_seed(value);
        }
        if arg == "--seed" {
            return args.get(i + 1).and_then(|v| parse_seed(v));
        }
    }
    None
}
