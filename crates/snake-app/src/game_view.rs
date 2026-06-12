//! A running game: fixed-tick loop, keyboard input, hex rendering.

use std::time::Duration;

use eframe::egui::{
    epaint::PathStroke, Align2, Color32, ComboBox, CornerRadius, FontId, Key, Painter, Rect, Sense,
    Shape, Stroke, StrokeKind, Ui,
};
use snake_core::strategy::{ChaosWalker, Greedy, PathPlanner};
use snake_core::{BoundaryMode, Config, Direction, GameState, Status, Strategy};

use crate::hex_layout::HexLayout;
use crate::seed::encode_seed;
use crate::settings::{Settings, Speed, StrategyChoice};

const KEY_BINDINGS: [(Key, Direction); 6] = [
    (Key::W, Direction::North),
    (Key::E, Direction::NorthEast),
    (Key::D, Direction::SouthEast),
    (Key::S, Direction::South),
    (Key::A, Direction::SouthWest),
    (Key::Q, Direction::NorthWest),
];

const BACKGROUND: Color32 = Color32::from_rgb(0x18, 0x18, 0x18);
const CELL_FILL: Color32 = Color32::from_rgb(0x24, 0x28, 0x2c);
const CELL_BORDER: Color32 = Color32::from_rgb(0x33, 0x39, 0x40);
const HEAD_COLOR: Color32 = Color32::from_rgb(0x9a, 0xe6, 0x3a);
const BODY_COLOR: Color32 = Color32::from_rgb(0x3f, 0xa3, 0x4d);
const BODY_TAIL_COLOR: Color32 = Color32::from_rgb(0x2a, 0x6e, 0x38);
const FOOD_COLOR: Color32 = Color32::from_rgb(0xe6, 0x4d, 0x3a);
const WALL_COLOR: Color32 = Color32::from_rgb(0x8a, 0x80, 0x70);
const PERIODIC_COLOR: Color32 = Color32::from_rgb(0x3a, 0xc6, 0xe6);

/// What the session wants the app to do next.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionEvent {
    Continue,
    Restart,
    BackToMenu,
}

pub struct GameSession {
    state: GameState,
    seed: u64,
    speed: Speed,
    /// Slowest base speed used at any point — highscores are filed under
    /// this speed's table, so switching speeds mid-game can't game a
    /// faster leaderboard.
    slowest_speed: Speed,
    paused: bool,
    /// Auto-pause while an in-game dropdown is open.
    ui_paused: bool,
    /// egui timestamp of the next due game tick.
    next_tick: Option<f64>,
    /// Scripted inputs (debug feature), consumed one per tick.
    script: std::collections::VecDeque<Direction>,
    strategy_choice: StrategyChoice,
    autopilot: Option<Box<dyn Strategy>>,
    autopilot_on: bool,
    /// True once the autopilot steered at least one tick — such runs are
    /// excluded from the highscores.
    autopilot_used: bool,
}

fn build_autopilot(choice: StrategyChoice, seed: u64) -> Option<Box<dyn Strategy>> {
    match choice {
        StrategyChoice::Human => None,
        StrategyChoice::Chaos => Some(Box::new(ChaosWalker::new(seed))),
        StrategyChoice::Greedy => Some(Box::new(Greedy)),
        StrategyChoice::Planner => Some(Box::new(PathPlanner::new())),
    }
}

impl GameSession {
    pub fn new(settings: &Settings, seed: u64, script: &[Direction]) -> Self {
        let (width, height) = settings.dimensions();
        let state = GameState::new(Config {
            width,
            height,
            boundary: settings.boundary,
            seed,
        });
        let autopilot = build_autopilot(settings.strategy, seed);
        let autopilot_on = autopilot.is_some();
        Self {
            state,
            seed,
            speed: settings.speed,
            slowest_speed: settings.speed,
            paused: false,
            ui_paused: false,
            next_tick: None,
            script: script.iter().copied().collect(),
            strategy_choice: settings.strategy,
            autopilot,
            autopilot_on,
            autopilot_used: false,
        }
    }

    pub fn autopilot_used(&self) -> bool {
        self.autopilot_used
    }

    /// The speed whose highscore table this run counts toward.
    pub fn slowest_speed(&self) -> Speed {
        self.slowest_speed
    }

    pub fn game_state(&self) -> &GameState {
        &self.state
    }

    /// Speed multiplier: the game gets faster as the snake grows.
    fn speed_multiplier(&self) -> f64 {
        1.03f64.powi(self.state.score() as i32).min(2.5)
    }

    /// `lock_input` suppresses all game key handling, e.g. while the
    /// highscore name dialog is open.
    pub fn ui(&mut self, ui: &mut Ui, lock_input: bool) -> SessionEvent {
        // The HUD comes first: it owns the in-game dropdowns, and an open
        // dropdown auto-pauses the game for this frame.
        self.hud(ui);
        let event = if lock_input {
            SessionEvent::Continue
        } else {
            self.handle_input(ui)
        };
        self.advance(ui);
        self.draw(ui);
        event
    }

    fn handle_input(&mut self, ui: &mut Ui) -> SessionEvent {
        let running = self.state.status() == Status::Running;
        ui.input(|input| {
            for (key, dir) in KEY_BINDINGS {
                if input.key_pressed(key) {
                    // A manual steering input always hands control back to
                    // the human.
                    if self.autopilot_on {
                        self.autopilot_on = false;
                        self.state.clear_input_queue();
                    }
                    self.state.push_input(dir);
                }
            }
            if self.autopilot.is_some() && input.key_pressed(Key::T) {
                self.autopilot_on = !self.autopilot_on;
            }

            if running && (input.key_pressed(Key::Space) || input.key_pressed(Key::P)) {
                self.paused = !self.paused;
                self.next_tick = None; // don't "catch up" ticks after a pause
            }
            if input.key_pressed(Key::Escape) {
                return SessionEvent::BackToMenu;
            }
            if !running && (input.key_pressed(Key::Enter) || input.key_pressed(Key::Space)) {
                return SessionEvent::Restart;
            }
            SessionEvent::Continue
        })
    }

    /// Run game ticks on a fixed interval, decoupled from the frame rate.
    fn advance(&mut self, ui: &Ui) {
        if self.ui_paused {
            self.next_tick = None; // no catch-up after the dropdown closes
            return;
        }
        if self.paused || self.state.status() != Status::Running {
            return;
        }
        let interval = 1.0 / (self.speed.ticks_per_second() * self.speed_multiplier());
        let now = ui.input(|i| i.time);
        let next_tick = *self.next_tick.get_or_insert(now + interval);
        if now >= next_tick {
            let input = if self.autopilot_on {
                if let Some(autopilot) = &mut self.autopilot {
                    // Stale buffered inputs would override the strategy.
                    self.state.clear_input_queue();
                    self.autopilot_used = true;
                    Some(autopilot.next_move(&self.state))
                } else {
                    None
                }
            } else {
                self.script.pop_front()
            };
            self.state.tick(input);
            // Schedule relative to the due time to avoid drift, but never
            // accumulate a backlog (e.g. after the tab was hidden).
            self.next_tick = Some((next_tick + interval).max(now));
        }
        let wait = (self.next_tick.unwrap() - now).max(0.0);
        ui.ctx()
            .request_repaint_after(Duration::from_secs_f64(wait));
    }

    fn draw(&self, ui: &mut Ui) {
        let (response, painter) =
            ui.allocate_painter(ui.available_size(), Sense::focusable_noninteractive());
        let rect = response.rect;
        painter.rect_filled(rect, 0.0, BACKGROUND);

        let board = self.state.board();
        let layout = HexLayout::fit(rect.shrink(12.0), board);

        for cell in board.cells() {
            painter.add(Shape::convex_polygon(
                layout.corners(cell, 1.0),
                CELL_FILL,
                PathStroke::new(1.0, CELL_BORDER),
            ));
        }

        self.draw_boundary(&painter, layout.bounds(board));

        // Food: a filled circle, clearly distinct from the hex snake.
        painter.circle_filled(
            layout.center(self.state.food()),
            layout.size * 0.45,
            FOOD_COLOR,
        );

        // Snake, tail first so the head ends up on top.
        let len = self.state.snake_len().max(2);
        for (i, cell) in self.state.snake().enumerate().rev() {
            let (color, scale) = if i == 0 {
                (HEAD_COLOR, 0.95)
            } else {
                let t = i as f32 / (len - 1) as f32;
                (BODY_COLOR.lerp_to_gamma(BODY_TAIL_COLOR, t), 0.85)
            };
            painter.add(Shape::convex_polygon(
                layout.corners(cell, scale),
                color,
                PathStroke::NONE,
            ));
        }

        match self.state.status() {
            Status::Running if self.paused => {
                self.overlay(&painter, rect, "Pause", "Space/P: weiter — Esc: Menü");
            }
            Status::GameOver => {
                let subtitle =
                    format!("Score {} — Enter: Neustart — Esc: Menü", self.state.score());
                self.overlay(&painter, rect, "Game Over", &subtitle);
            }
            Status::Won => {
                self.overlay(
                    &painter,
                    rect,
                    "Gewonnen!",
                    "Das Brett ist voll — Enter: Neustart",
                );
            }
            _ => {}
        }
    }

    fn hud(&mut self, ui: &mut Ui) {
        let mut dropdown_open = false;
        ui.horizontal(|ui| {
            ui.label(format!("Score: {}", self.state.score()));
            ui.separator();
            ui.label(format!("Länge: {}", self.state.snake_len()));
            ui.separator();
            let boundary = match self.state.board().boundary {
                BoundaryMode::Walls => "Wände",
                BoundaryMode::Periodic => "Periodisch",
            };
            ui.label(boundary);
            ui.separator();
            ui.label(format!("Seed: {}", encode_seed(self.seed)));
            ui.separator();

            ui.label("Tempo:");
            let speed_combo = ComboBox::from_id_salt("hud_speed")
                .selected_text(self.speed.label())
                .show_ui(ui, |ui| {
                    let mut speed = self.speed;
                    for option in Speed::ALL {
                        ui.selectable_value(&mut speed, option, option.label());
                    }
                    if speed != self.speed {
                        self.speed = speed;
                        self.slowest_speed = self.slowest_speed.min(speed);
                        self.next_tick = None;
                    }
                });
            dropdown_open |= speed_combo.inner.is_some();
            ui.label(format!("×{:.2}", self.speed_multiplier()));
            ui.separator();

            ui.label("Autopilot:");
            let strategy_combo = ComboBox::from_id_salt("hud_strategy")
                .selected_text(self.strategy_choice.label())
                .show_ui(ui, |ui| {
                    let mut choice = self.strategy_choice;
                    for option in StrategyChoice::ALL {
                        ui.selectable_value(&mut choice, option, option.label());
                    }
                    if choice != self.strategy_choice {
                        self.strategy_choice = choice;
                        self.autopilot = build_autopilot(choice, self.seed);
                        self.autopilot_on = self.autopilot.is_some();
                        self.state.clear_input_queue();
                    }
                });
            dropdown_open |= strategy_combo.inner.is_some();
            if self.autopilot.is_some() {
                let status = if self.autopilot_on { "an" } else { "aus" };
                ui.label(format!("{status} [T]"));
            }

            // The key hints are decoration: drop them before they would
            // collide with the dropdowns on narrow windows.
            if ui.available_width() > 380.0 {
                ui.with_layout(
                    eframe::egui::Layout::right_to_left(eframe::egui::Align::Center),
                    |ui| {
                        ui.weak("QWEASD: Richtung — Space/P: Pause — Esc: Menü");
                    },
                );
            }
        });
        self.ui_paused = dropdown_open;
    }

    /// Visualize the boundary mode: solid "wall" frame vs. a dashed, open
    /// border for the periodic (torus) field.
    fn draw_boundary(&self, painter: &Painter, bounds: Rect) {
        let bounds = bounds.expand(5.0);
        match self.state.board().boundary {
            BoundaryMode::Walls => {
                painter.rect_stroke(
                    bounds,
                    CornerRadius::ZERO,
                    Stroke::new(5.0, WALL_COLOR),
                    StrokeKind::Outside,
                );
            }
            BoundaryMode::Periodic => {
                let corners = [
                    bounds.left_top(),
                    bounds.right_top(),
                    bounds.right_bottom(),
                    bounds.left_bottom(),
                    bounds.left_top(),
                ];
                for edge in corners.windows(2) {
                    painter.add(Shape::dashed_line(
                        edge,
                        Stroke::new(2.0, PERIODIC_COLOR),
                        8.0,
                        6.0,
                    ));
                }
            }
        }
    }

    fn overlay(&self, painter: &Painter, rect: Rect, title: &str, subtitle: &str) {
        painter.rect_filled(rect, 0.0, Color32::from_black_alpha(160));
        painter.text(
            rect.center() - eframe::egui::Vec2::new(0.0, 20.0),
            Align2::CENTER_CENTER,
            title,
            FontId::proportional(48.0),
            Color32::WHITE,
        );
        painter.text(
            rect.center() + eframe::egui::Vec2::new(0.0, 24.0),
            Align2::CENTER_CENTER,
            subtitle,
            FontId::proportional(18.0),
            Color32::from_gray(220),
        );
    }
}
