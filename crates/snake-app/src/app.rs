use eframe::egui;
use snake_core::api::{ScoreEntry, ScoreSubmission};
use snake_core::{BoundaryMode, Direction, Replay, Status};

use crate::game_view::{GameSession, SessionEvent};
use crate::highscores::{self, Entry, Highscores};
use crate::net::{Online, PendingSubmission, Reachability};
use crate::seed::{encode_seed, parse_seed, random_seed};
use crate::settings::{Settings, SizePreset, Speed, StrategyChoice, HAMILTON_INCOMPATIBLE_HINT};
use crate::stats::Stats;
use crate::theme::ThemeId;

/// Captured details of the finished run, used for global submission once the
/// player confirms a name.
struct RunInfo {
    replay: Replay,
    /// Online cache key / routing: a preset mode key, or `"daily"`.
    mode: String,
    /// Whether to submit to the daily-challenge endpoint.
    daily: bool,
    /// Eligible for the global leaderboard: a preset board (or a server-seed
    /// daily run), played without the autopilot, with a positive score.
    eligible_global: bool,
}

enum Screen {
    Menu,
    Game(Box<GameSession>),
}

/// What the menu wants to do this frame.
enum MenuAction {
    None,
    Start,
    Daily,
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
    stats: Stats,
    /// Whether the current game's end was already counted in the stats.
    stats_recorded: bool,
    /// Optional global-highscore networking (best-effort, offline-safe).
    online: Online,
    /// The finished run captured for possible global submission.
    last_run: Option<RunInfo>,
    /// Whether the running game is today's daily challenge.
    daily_run: bool,
    /// Set by the menu to launch the daily challenge on the next restart.
    start_daily_next: bool,
    /// One-shot focus grab for the name dialog. Re-requesting focus every
    /// frame would cancel the Enter-submits-the-field detection
    /// (`lost_focus` is false once focus is re-acquired).
    name_focus_grabbed: bool,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let (settings, highscores, player_name, stats, pending) = match cc.storage {
            Some(storage) => (
                eframe::get_value(storage, "settings").unwrap_or_default(),
                eframe::get_value(storage, "highscores").unwrap_or_default(),
                eframe::get_value(storage, "player_name").unwrap_or_default(),
                eframe::get_value(storage, "stats").unwrap_or_default(),
                eframe::get_value::<Vec<PendingSubmission>>(storage, "pending_scores")
                    .unwrap_or_default(),
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
            stats,
            stats_recorded: false,
            online: Online::new(pending),
            last_run: None,
            daily_run: false,
            start_daily_next: false,
            name_focus_grabbed: false,
        }
    }

    fn next_seed(&self) -> u64 {
        self.seed_override.unwrap_or_else(random_seed)
    }

    fn start_game(&mut self) {
        let seed = self.next_seed();
        self.start_with(self.settings.clone(), seed, false);
    }

    /// Start today's daily challenge: a fixed board (medium, walls) with the
    /// server's daily seed, or a date-based local fallback when offline.
    fn start_daily(&mut self) {
        let seed = self
            .online
            .challenge()
            .map(|c| c.seed)
            .unwrap_or_else(local_daily_seed);
        let settings = Settings {
            boundary: BoundaryMode::Walls,
            preset: SizePreset::Medium,
            ..self.settings.clone()
        };
        self.start_with(settings, seed, true);
    }

    fn start_with(&mut self, settings: Settings, seed: u64, daily: bool) {
        self.highscore_done = false;
        self.stats_recorded = false;
        self.last_run = None;
        self.daily_run = daily;
        self.name_focus_grabbed = false;
        self.screen = Screen::Game(Box::new(GameSession::new(
            &settings,
            seed,
            &self.input_script,
        )));
    }

    fn menu_ui(&mut self, ui: &mut egui::Ui) -> MenuAction {
        let mut action = MenuAction::None;
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
                action = MenuAction::Start;
            }
            ui.add_space(12.0);
            ui.weak("Steuerung: Q W E / A S D — sechs Richtungen auf dem Hexgitter");
            if let Some(seed) = self.seed_override {
                ui.weak(format!("Seed fixiert: {}", encode_seed(seed)));
            }

            // Daily challenge: one shared seed per day. Online it comes from
            // the server (and is globally rankable); offline a date-based
            // local seed keeps it playable, ranked on the local daily table.
            ui.add_space(16.0);
            ui.separator();
            ui.heading("Tägliche Challenge");
            if self.online.enabled() && self.online.challenge().is_none() {
                self.online.fetch_challenge(ui.ctx());
            } else if self.online.enabled() {
                self.online.fetch_daily_board(ui.ctx());
            }
            let daily_seed = self
                .online
                .challenge()
                .map(|c| c.seed)
                .unwrap_or_else(local_daily_seed);
            ui.weak(format!(
                "Heute (24×18, Wände) — Seed {}",
                encode_seed(daily_seed)
            ));
            if ui.button("Tägliche Challenge spielen").clicked() {
                action = MenuAction::Daily;
            }
            let daily_key = format!("daily-{}", highscores::today_iso());
            let local_daily = self.highscores.table(&daily_key);
            if !local_daily.is_empty() {
                ui.add_space(4.0);
                ui.weak("Lokal:");
                highscore_table(ui, local_daily);
            }
            if let Some(board) = self.online.board("daily") {
                if !board.entries.is_empty() {
                    ui.add_space(4.0);
                    ui.weak("Global:");
                    global_table(ui, &board.entries);
                }
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

            // Global leaderboard (preset modes only): fetched lazily, shown
            // next to the local table. Absent server ⇒ this stays hidden.
            if self.online.enabled() && settings.preset != SizePreset::Custom {
                self.online.fetch_board(ui.ctx(), &key);
                if let Some(board) = self.online.board(&key) {
                    if !board.entries.is_empty() {
                        ui.add_space(16.0);
                        ui.heading("Global");
                        global_table(ui, &board.entries);
                    }
                }
                let pending = self.online.pending().len();
                if pending > 0 {
                    ui.add_space(4.0);
                    ui.weak(format!(
                        "{pending} Lauf(e) warten auf Übertragung zum Server"
                    ));
                }
                if self.online.reachability() == Reachability::Offline {
                    ui.weak("Server nicht erreichbar — es zählen nur die lokalen Tabellen.");
                }
            }

            if self.stats.games > 0 {
                ui.add_space(16.0);
                ui.heading("Statistik");
                ui.weak("Nur selbst gespielte Partien");
                ui.label(format!(
                    "Partien: {} — Ø Endlänge: {:.1} — Bester Score: {} — Längste Schlange: {}",
                    self.stats.games,
                    self.stats.avg_length(),
                    self.stats.best_score,
                    self.stats.best_length
                ));
            }
        });
        action
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
                                name: name.clone(),
                                score,
                                date: highscores::today_iso(),
                            },
                        );
                        // Mirror the run to the global leaderboard if it
                        // qualifies (preset, non-autopilot). Offline runs are
                        // queued and retried later — purely best-effort.
                        let global = self
                            .last_run
                            .as_ref()
                            .filter(|r| r.eligible_global)
                            .map(|r| (r.mode.clone(), r.daily, r.replay.clone()));
                        if let Some((mode, daily, replay)) = global {
                            self.online.submit(
                                ctx,
                                mode,
                                daily,
                                ScoreSubmission {
                                    name,
                                    claimed_score: score,
                                    replay,
                                    signature: None,
                                },
                            );
                        }
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

fn global_table(ui: &mut egui::Ui, entries: &[ScoreEntry]) {
    egui::Grid::new("global_table")
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

            // Drain background network results and retry queued submissions.
            self.online.pump();
            let now = ui.input(|i| i.time);
            self.online.retry_pending(ui.ctx(), now);

            // Game-over bookkeeping happens outside the borrow of `screen`.
            let mut game_over_score = None;
            let action = match &mut self.screen {
                Screen::Menu => match self.menu_ui(ui) {
                    MenuAction::None => SessionEvent::Continue,
                    MenuAction::Start => {
                        self.start_daily_next = false;
                        SessionEvent::Restart
                    }
                    MenuAction::Daily => {
                        self.start_daily_next = true;
                        SessionEvent::Restart
                    }
                },
                Screen::Game(session) => {
                    let state = session.game_state();
                    let finished = matches!(state.status(), Status::GameOver | Status::Won);
                    let score = state.score();
                    if finished && !self.stats_recorded {
                        // Stats count self-played games only, like the
                        // highscores.
                        if !session.autopilot_used() {
                            self.stats.record(score, state.snake_len() as u32);
                        }
                        self.stats_recorded = true;
                    }
                    // The daily challenge has its own table keyed by date;
                    // normal runs are filed under the slowest speed used.
                    let key = if self.daily_run {
                        format!("daily-{}", highscores::today_iso())
                    } else {
                        highscores::mode_key_with_speed(&self.settings, session.slowest_speed())
                    };
                    // Autopilot runs are excluded from the highscores.
                    let dialog_open = finished
                        && !self.highscore_done
                        && !session.autopilot_used()
                        && self.highscores.qualifies(&key, score);
                    // Capture the run once, for an eventual global submission.
                    if finished && self.last_run.is_none() {
                        let replay = session.replay();
                        // Daily runs are only globally rankable when they used
                        // the server's seed (offline fallback seeds differ and
                        // would be rejected on re-simulation).
                        let eligible = !session.autopilot_used()
                            && score > 0
                            && if self.daily_run {
                                self.online.challenge().map(|c| c.seed) == Some(replay.seed)
                            } else {
                                self.settings.preset != SizePreset::Custom
                            };
                        self.last_run = Some(RunInfo {
                            mode: if self.daily_run {
                                "daily".to_owned()
                            } else {
                                key.clone()
                            },
                            daily: self.daily_run,
                            replay,
                            eligible_global: eligible,
                        });
                    }
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
                SessionEvent::Restart => {
                    if std::mem::take(&mut self.start_daily_next) {
                        self.start_daily();
                    } else {
                        self.start_game();
                    }
                }
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
        eframe::set_value(storage, "stats", &self.stats);
        let pending = self.online.pending().to_vec();
        eframe::set_value(storage, "pending_scores", &pending);
    }
}

/// Date-based daily-challenge seed used when no server is reachable. Mirrors
/// the server's mixing (without the secret), so it is stable per day; runs
/// using it stay on the local daily table only.
fn local_daily_seed() -> u64 {
    let mut z = (today_days() as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^= z >> 31;
    z & 0xFFFF_FFFF
}

#[cfg(not(target_arch = "wasm32"))]
fn today_days() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| (d.as_secs() / 86_400) as i64)
        .unwrap_or(0)
}

#[cfg(target_arch = "wasm32")]
fn today_days() -> i64 {
    (eframe::web_sys::js_sys::Date::now() / 86_400_000.0) as i64
}

pub(crate) fn parse_input_script(script: &str) -> Vec<Direction> {
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
