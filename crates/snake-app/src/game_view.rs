//! A running game: fixed-tick loop, keyboard input, hex rendering.

use std::time::Duration;

use eframe::egui::{
    epaint::PathStroke, Align2, Color32, ComboBox, CornerRadius, FontId, Key, Painter, Pos2, Rect,
    Sense, Shape, Stroke, StrokeKind, Ui, Vec2,
};
use snake_core::nn::NeuralNet;
use snake_core::strategy::{
    ChaosWalker, Greedy, HamiltonRider, MonteCarlo, PathPlanner, SpaceKeeper,
};
use snake_core::{
    Board, BoundaryMode, Config, Direction, GameState, Recorder, Replay, Status, Strategy,
};

use crate::hex_layout::HexLayout;
use crate::seed::encode_seed;
use crate::settings::{Settings, Speed, StrategyChoice, HAMILTON_INCOMPATIBLE_HINT};
use crate::theme::{FoodStyle, HeadMarker, SnakeStyle, Theme, ThemeId};

const KEY_BINDINGS: [(Key, Direction); 6] = [
    (Key::W, Direction::North),
    (Key::E, Direction::NorthEast),
    (Key::D, Direction::SouthEast),
    (Key::S, Direction::South),
    (Key::A, Direction::SouthWest),
    (Key::Q, Direction::NorthWest),
];

const SQRT3: f32 = 1.732_050_8;

const OVERLAY_PATH_COLOR: Color32 = Color32::from_rgb(0xff, 0xa0, 0x20);
// Premultiplied teal at ~11% alpha (0x3a, 0xc6, 0xe6 × 28/255).
const OVERLAY_HEAT_COLOR: Color32 = Color32::from_rgba_premultiplied(6, 22, 25, 28);

/// What the session wants the app to do next.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionEvent {
    Continue,
    Restart,
    BackToMenu,
}

pub struct GameSession {
    state: GameState,
    /// Records every tick's input so the run can be submitted to the global
    /// leaderboard (seed + input list ⇒ server re-simulation).
    recorder: Recorder,
    config: Config,
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
    theme: ThemeId,
    autopilot: Option<Box<dyn Strategy>>,
    autopilot_on: bool,
    /// Debug overlay (key O): shows what the autopilot "thinks".
    overlay_on: bool,
    /// True once the autopilot steered at least one tick — such runs are
    /// excluded from the highscores.
    autopilot_used: bool,
    /// Virtual hex pad, shown once a touch event was seen.
    touch_seen: bool,
    prev_score: u32,
    prev_status: Status,
    /// Eat effect: cell where food was eaten + egui start time.
    eat_effect: Option<(snake_core::Offset, f64)>,
    /// Game-over pulse: egui start time.
    game_over_effect: Option<f64>,
}

const EAT_EFFECT_SECS: f64 = 0.45;
const GAME_OVER_EFFECT_SECS: f64 = 0.9;

fn build_autopilot(choice: StrategyChoice, seed: u64, board: &Board) -> Option<Box<dyn Strategy>> {
    match choice {
        StrategyChoice::Human => None,
        StrategyChoice::Chaos => Some(Box::new(ChaosWalker::new(seed))),
        StrategyChoice::Greedy => Some(Box::new(Greedy)),
        StrategyChoice::Planner => Some(Box::new(PathPlanner::new())),
        StrategyChoice::Space => Some(Box::new(SpaceKeeper::new())),
        // None for incompatible board sizes — the UI greys the entry out.
        StrategyChoice::Hamilton => {
            HamiltonRider::new(board).map(|rider| Box::new(rider) as Box<dyn Strategy>)
        }
        StrategyChoice::MonteCarlo => Some(Box::new(MonteCarlo::new(seed))),
        StrategyChoice::NeuralNet => Some(Box::new(NeuralNet::embedded())),
    }
}

impl GameSession {
    pub fn new(settings: &Settings, seed: u64, script: &[Direction]) -> Self {
        let (width, height) = settings.dimensions();
        let config = Config {
            width,
            height,
            boundary: settings.boundary,
            seed,
        };
        let state = GameState::new(config);
        let autopilot = build_autopilot(settings.strategy, seed, state.board());
        let autopilot_on = autopilot.is_some();
        Self {
            state,
            recorder: Recorder::new(),
            config,
            seed,
            speed: settings.speed,
            slowest_speed: settings.speed,
            paused: false,
            ui_paused: false,
            next_tick: None,
            script: script.iter().copied().collect(),
            strategy_choice: settings.strategy,
            theme: settings.theme,
            autopilot,
            autopilot_on,
            overlay_on: false,
            autopilot_used: false,
            touch_seen: false,
            prev_score: 0,
            prev_status: Status::Running,
            eat_effect: None,
            game_over_effect: None,
        }
    }

    pub fn autopilot_used(&self) -> bool {
        self.autopilot_used
    }

    /// The recorded run as a submittable replay (seed + input list).
    pub fn replay(&self) -> Replay {
        self.recorder.clone().into_replay(self.config)
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
        self.detect_effects(ui);
        self.draw(ui);
        let touch_event = self.touch_pad(ui, lock_input);
        if touch_event != SessionEvent::Continue {
            touch_event
        } else {
            event
        }
    }

    /// Virtual hex pad for touch devices: six direction buttons arranged
    /// like the hex neighbors plus a central pause button. Appears after
    /// the first touch event and is drawn on the foreground layer.
    /// On game over shows "Neustart" and "Menü" buttons instead.
    fn touch_pad(&mut self, ui: &mut Ui, lock_input: bool) -> SessionEvent {
        if ui.ctx().input(|i| i.any_touches()) {
            self.touch_seen = true;
        }
        if !self.touch_seen {
            return SessionEvent::Continue;
        }

        let screen = ui.ctx().content_rect();
        let status = self.state.status();
        let finished = matches!(status, Status::GameOver | Status::Won);

        let painter = ui.ctx().layer_painter(eframe::egui::LayerId::new(
            eframe::egui::Order::Foreground,
            eframe::egui::Id::new("touch_pad"),
        ));

        let pressed_at = ui.input(|i| {
            (!lock_input && i.pointer.any_pressed())
                .then(|| i.pointer.interact_pos())
                .flatten()
        });

        if finished && !lock_input {
            // Replace the direction pad with two large tap targets.
            let center = screen.center() + Vec2::new(0.0, 80.0);
            let button_w = 130.0;
            let button_h = 50.0;
            let gap = 16.0;

            let restart_rect = Rect::from_center_size(
                center + Vec2::new(-(button_w * 0.5 + gap * 0.5), 0.0),
                Vec2::new(button_w, button_h),
            );
            let menu_rect = Rect::from_center_size(
                center + Vec2::new(button_w * 0.5 + gap * 0.5, 0.0),
                Vec2::new(button_w, button_h),
            );

            for (rect, label) in [(&restart_rect, "Neustart"), (&menu_rect, "Menü")] {
                let pressed = pressed_at.is_some_and(|p| rect.contains(p));
                let bg_alpha = if pressed { 80u8 } else { 35u8 };
                painter.rect_filled(
                    *rect,
                    8.0,
                    Color32::from_rgba_unmultiplied(255, 255, 255, bg_alpha),
                );
                painter.rect_stroke(
                    *rect,
                    8.0,
                    Stroke::new(1.5, Color32::from_rgba_unmultiplied(255, 255, 255, 130)),
                    StrokeKind::Middle,
                );
                painter.text(
                    rect.center(),
                    Align2::CENTER_CENTER,
                    label.to_string(),
                    FontId::proportional(20.0),
                    Color32::WHITE,
                );
            }

            if let Some(pos) = pressed_at {
                if restart_rect.contains(pos) {
                    return SessionEvent::Restart;
                }
                if menu_rect.contains(pos) {
                    return SessionEvent::BackToMenu;
                }
            }
            return SessionEvent::Continue;
        }

        // Running (or paused) game: direction ring + central pause button.
        let center = screen.right_bottom() - Vec2::new(110.0, 120.0);
        let ring = 62.0;
        let button_r = 26.0;

        // Center: pause toggle.
        painter.circle(
            center,
            button_r * 0.7,
            Color32::from_rgba_unmultiplied(255, 255, 255, 18),
            Stroke::new(1.5, Color32::from_rgba_unmultiplied(255, 255, 255, 60)),
        );
        painter.text(
            center,
            Align2::CENTER_CENTER,
            "⏸",
            FontId::proportional(18.0),
            Color32::from_rgba_unmultiplied(255, 255, 255, 160),
        );
        if let Some(pos) = pressed_at {
            if pos.distance(center) <= button_r * 0.7 && status == Status::Running {
                self.paused = !self.paused;
                self.next_tick = None;
            }
        }

        for dir in Direction::ALL {
            let pos = center + dir_vector(dir, ring / SQRT3);
            painter.circle(
                pos,
                button_r,
                Color32::from_rgba_unmultiplied(255, 255, 255, 18),
                Stroke::new(1.5, Color32::from_rgba_unmultiplied(255, 255, 255, 60)),
            );
            // Arrow: small triangle pointing outward.
            let out = dir_vector(dir, 1.0).normalized();
            let perp = Vec2::new(-out.y, out.x);
            painter.add(Shape::convex_polygon(
                vec![
                    pos + out * 12.0,
                    pos - out * 6.0 + perp * 8.0,
                    pos - out * 6.0 - perp * 8.0,
                ],
                Color32::from_rgba_unmultiplied(255, 255, 255, 170),
                PathStroke::NONE,
            ));
            if let Some(p) = pressed_at {
                if p.distance(pos) <= button_r {
                    if self.autopilot_on {
                        self.autopilot_on = false;
                        self.state.clear_input_queue();
                    }
                    self.state.push_input(dir);
                }
            }
        }

        SessionEvent::Continue
    }

    /// Trigger the (theme-independent) eat and game-over effects on state
    /// transitions.
    fn detect_effects(&mut self, ui: &Ui) {
        let now = ui.input(|i| i.time);
        if self.state.score() > self.prev_score {
            // The food was eaten on the cell the head now occupies.
            self.eat_effect = Some((self.state.head(), now));
        }
        self.prev_score = self.state.score();

        let status = self.state.status();
        if status != Status::Running && self.prev_status == Status::Running {
            self.game_over_effect = Some(now);
        }
        self.prev_status = status;
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
            if input.key_pressed(Key::O) {
                self.overlay_on = !self.overlay_on;
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
            self.recorder.record(input);
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
        let theme = self.theme.theme();
        // Cap the board to the visible viewport and draw into it. A HUD row
        // wider than a narrow (mobile) screen inflates `available_size`,
        // which would otherwise size the painter larger than the screen and
        // push the board (and the pause/game-over overlay) partly off the
        // sides. Clamping to `content_rect` keeps everything centred and on
        // screen.
        let screen = ui.ctx().content_rect();
        let avail = ui.available_size();
        let desired = Vec2::new(avail.x.min(screen.width()), avail.y.min(screen.height()));
        let (response, painter) = ui.allocate_painter(desired, Sense::focusable_noninteractive());
        let rect = response.rect.intersect(screen);
        painter.rect_filled(rect, 0.0, theme.background);

        let board = self.state.board();
        // Inset enough that the wall frame (drawn ~10px outside the grid
        // bounds) also stays within the viewport.
        let layout = HexLayout::fit(rect.shrink(16.0), board);

        for cell in board.cells() {
            painter.add(Shape::convex_polygon(
                layout.corners(cell, 1.0),
                theme.cell_fill,
                PathStroke::new(theme.cell_border_width, theme.cell_border),
            ));
        }

        self.draw_boundary(&painter, theme, layout.bounds(board));
        self.draw_food(&painter, theme, &layout);
        self.draw_snake(&painter, theme, &layout);

        self.draw_effects(ui, &painter, theme, &layout);

        if self.overlay_on {
            self.draw_strategy_overlay(&painter, &layout);
        }

        match self.state.status() {
            Status::Running if self.paused => {
                let subtitle = if self.touch_seen {
                    "Tippen auf ⏸ zum Fortsetzen"
                } else {
                    "Space/P: weiter — Esc: Menü"
                };
                self.overlay(&painter, rect, "Pause", subtitle);
            }
            Status::GameOver => {
                let subtitle = if self.touch_seen {
                    format!("Score {}", self.state.score())
                } else {
                    format!("Score {} — Enter: Neustart — Esc: Menü", self.state.score())
                };
                self.overlay(&painter, rect, "Game Over", &subtitle);
            }
            Status::Won => {
                let subtitle = if self.touch_seen {
                    "Das Brett ist voll!"
                } else {
                    "Das Brett ist voll — Enter: Neustart"
                };
                self.overlay(&painter, rect, "Gewonnen!", subtitle);
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
            let board = *self.state.board();
            let strategy_combo = ComboBox::from_id_salt("hud_strategy")
                .selected_text(self.strategy_choice.label())
                .show_ui(ui, |ui| {
                    let mut choice = self.strategy_choice;
                    for option in StrategyChoice::ALL {
                        let enabled = option.compatible_with(board.width, board.height);
                        let label =
                            eframe::egui::Button::selectable(choice == option, option.label());
                        let response = ui
                            .add_enabled(enabled, label)
                            .on_disabled_hover_text(HAMILTON_INCOMPATIBLE_HINT);
                        if response.clicked() {
                            choice = option;
                        }
                    }
                    if choice != self.strategy_choice {
                        self.strategy_choice = choice;
                        self.autopilot = build_autopilot(choice, self.seed, &board);
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
                        ui.weak("QWEASD: Richtung — Space/P: Pause — O: Overlay — Esc: Menü");
                    },
                );
            }
        });
        self.ui_paused = dropdown_open;
    }

    /// Draw what the autopilot "thinks": flood-fill heatmap, planned path
    /// and per-direction scores (key O).
    fn draw_strategy_overlay(&self, painter: &Painter, layout: &HexLayout) {
        let Some(debug) = self.autopilot.as_ref().and_then(|a| a.debug()) else {
            return;
        };
        let board = self.state.board();

        for cell in &debug.heatmap {
            painter.add(Shape::convex_polygon(
                layout.corners(*cell, 0.92),
                OVERLAY_HEAT_COLOR,
                PathStroke::NONE,
            ));
        }

        // Planned path as a line through the cell centers. On the torus a
        // wrap shows up as a long jump on screen — break the line there.
        let mut points = vec![layout.center(self.state.head())];
        points.extend(debug.path.iter().map(|c| layout.center(*c)));
        let max_segment = layout.size * 2.2;
        for pair in points.windows(2) {
            if (pair[1] - pair[0]).length() <= max_segment {
                painter.line_segment([pair[0], pair[1]], Stroke::new(3.0, OVERLAY_PATH_COLOR));
            }
        }
        if let Some(last) = debug.path.last() {
            painter.circle_filled(layout.center(*last), layout.size * 0.2, OVERLAY_PATH_COLOR);
        }

        for (dir, score) in &debug.move_scores {
            if let Some(cell) = board.neighbor(self.state.head(), *dir) {
                painter.text(
                    layout.center(cell),
                    Align2::CENTER_CENTER,
                    format!("{score:.0}"),
                    FontId::proportional((layout.size * 0.6).clamp(9.0, 16.0)),
                    Color32::WHITE,
                );
            }
        }
    }

    /// Visualize the boundary mode: solid "wall" frame vs. a dashed, open
    /// border for the periodic (torus) field.
    fn draw_boundary(&self, painter: &Painter, theme: &Theme, bounds: Rect) {
        let bounds = bounds.expand(5.0);
        match self.state.board().boundary {
            BoundaryMode::Walls => {
                painter.rect_stroke(
                    bounds,
                    CornerRadius::ZERO,
                    Stroke::new(5.0, theme.wall),
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
                        Stroke::new(2.0, theme.periodic),
                        8.0,
                        6.0,
                    ));
                }
            }
        }
    }

    /// Expanding, fading rings on eating and on game over.
    fn draw_effects(&self, ui: &Ui, painter: &Painter, theme: &Theme, layout: &HexLayout) {
        let now = ui.input(|i| i.time);
        let s = layout.size;
        let mut animating = false;

        if let Some((cell, start)) = self.eat_effect {
            let t = ((now - start) / EAT_EFFECT_SECS) as f32;
            if (0.0..1.0).contains(&t) {
                let fade = 1.0 - t;
                painter.circle_stroke(
                    layout.center(cell),
                    s * (0.5 + 1.4 * t),
                    Stroke::new(
                        (s * 0.14 * fade).max(1.0),
                        theme.accent.gamma_multiply(fade),
                    ),
                );
                animating = true;
            }
        }

        if let Some(start) = self.game_over_effect {
            let t = ((now - start) / GAME_OVER_EFFECT_SECS) as f32;
            if (0.0..1.0).contains(&t) {
                let head = layout.center(self.state.head());
                for (delay, color) in [(0.0, theme.food), (0.25, theme.accent)] {
                    let tt = ((t - delay) / (1.0 - delay)).clamp(0.0, 1.0);
                    if tt > 0.0 && tt < 1.0 {
                        let fade = 1.0 - tt;
                        painter.circle_stroke(
                            head,
                            s * (0.6 + 3.5 * tt),
                            Stroke::new((s * 0.18 * fade).max(1.0), color.gamma_multiply(fade)),
                        );
                    }
                }
                animating = true;
            }
        }

        if animating {
            ui.ctx().request_repaint();
        }
    }

    fn draw_food(&self, painter: &Painter, theme: &Theme, layout: &HexLayout) {
        let c = layout.center(self.state.food());
        let s = layout.size;
        match theme.food_style {
            FoodStyle::Circle => {
                painter.circle_filled(c, s * 0.45, theme.food);
            }
            FoodStyle::Drop => {
                painter.circle_filled(c + Vec2::new(0.0, 0.12 * s), s * 0.38, theme.food);
                painter.add(Shape::convex_polygon(
                    vec![
                        c + Vec2::new(-0.26 * s, -0.12 * s),
                        c + Vec2::new(0.0, -0.72 * s),
                        c + Vec2::new(0.26 * s, -0.12 * s),
                    ],
                    theme.food,
                    PathStroke::NONE,
                ));
                painter.circle_filled(
                    c + Vec2::new(-0.12 * s, 0.05 * s),
                    s * 0.09,
                    Color32::from_rgba_unmultiplied(255, 255, 255, 140),
                );
            }
            FoodStyle::Ring => {
                painter.circle_stroke(c, s * 0.38, Stroke::new(s * 0.16, theme.food));
            }
            FoodStyle::GlowOrb => {
                painter.circle_filled(c, s * 0.85, theme.food.gamma_multiply(0.15));
                painter.circle_filled(c, s * 0.55, theme.food.gamma_multiply(0.4));
                painter.circle_filled(c, s * 0.32, theme.food);
            }
            FoodStyle::Apple => {
                painter.circle_filled(c + Vec2::new(0.0, 0.08 * s), s * 0.42, theme.food);
                painter.line_segment(
                    [
                        c + Vec2::new(0.0, -0.3 * s),
                        c + Vec2::new(0.12 * s, -0.62 * s),
                    ],
                    Stroke::new(s * 0.08, Color32::from_rgb(0x6e, 0x4a, 0x2a)),
                );
                painter.circle_filled(
                    c + Vec2::new(0.26 * s, -0.52 * s),
                    s * 0.11,
                    Color32::from_rgb(0x5d, 0x8a, 0x2e),
                );
            }
            FoodStyle::Triangle => {
                painter.add(Shape::convex_polygon(
                    vec![
                        c + Vec2::new(0.0, -0.52 * s),
                        c + Vec2::new(0.46 * s, 0.3 * s),
                        c + Vec2::new(-0.46 * s, 0.3 * s),
                    ],
                    theme.food,
                    PathStroke::NONE,
                ));
            }
        }
    }

    fn draw_snake(&self, painter: &Painter, theme: &Theme, layout: &HexLayout) {
        let board = self.state.board();
        let cells: Vec<_> = self.state.snake().collect();
        let centers: Vec<Pos2> = cells.iter().map(|c| layout.center(*c)).collect();
        let n = cells.len();
        let s = layout.size;
        let blend = |i: usize| -> Color32 {
            let t = i as f32 / (n.max(2) - 1) as f32;
            theme.body.lerp_to_gamma(theme.tail, t)
        };

        match theme.snake_style {
            SnakeStyle::Hexes => {
                for i in (0..n).rev() {
                    let (color, scale) = if i == 0 {
                        (theme.head, 0.95)
                    } else {
                        (blend(i), 0.85)
                    };
                    painter.add(Shape::convex_polygon(
                        layout.corners(cells[i], scale),
                        color,
                        PathStroke::NONE,
                    ));
                }
            }
            SnakeStyle::Band { taper, glow } => {
                let width_at = |i: usize| -> f32 {
                    let head_w = 0.66 * s;
                    let tail_w = if taper { 0.2 * s } else { 0.66 * s };
                    head_w + (tail_w - head_w) * (i as f32 / (n.max(2) - 1) as f32)
                };
                let passes: &[(f32, f32)] = if glow {
                    &[(2.4, 0.12), (1.6, 0.3), (1.0, 1.0)]
                } else {
                    &[(1.0, 1.0)]
                };
                for &(mult, alpha) in passes {
                    for i in (0..n).rev() {
                        let color = if i == 0 { theme.head } else { blend(i) };
                        let color = color.gamma_multiply(alpha);
                        painter.circle_filled(centers[i], width_at(i) * 0.5 * mult, color);
                        if i + 1 < n {
                            let w = (width_at(i) + width_at(i + 1)) * 0.5 * mult;
                            let stroke = Stroke::new(w, blend(i).gamma_multiply(alpha));
                            self.band_segment(painter, board, layout, &cells, &centers, i, stroke);
                        }
                    }
                }
            }
            SnakeStyle::Caterpillar => {
                for i in (0..n).rev() {
                    // Connector at the shared edge keeps the chain visually
                    // linked, also across a torus wrap.
                    if i + 1 < n {
                        if let Some(dir) = direction_between(board, cells[i], cells[i + 1]) {
                            let mid = centers[i] + dir_vector(dir, s) * 0.5;
                            painter.circle_filled(mid, s * 0.28, blend(i));
                        }
                    }
                    let (color, radius) = if i == 0 {
                        (theme.head, 0.5 * s)
                    } else {
                        (blend(i), if i % 2 == 0 { 0.42 * s } else { 0.38 * s })
                    };
                    painter.circle_filled(centers[i], radius, color);
                }
            }
        }

        self.draw_head_marker(painter, theme, layout, centers[0]);
    }

    /// One body segment of the band. A torus wrap shows as two stubs that
    /// exit/enter through the shared edge instead of a line across the
    /// whole board.
    #[allow(clippy::too_many_arguments)] // internal helper, plain data
    fn band_segment(
        &self,
        painter: &Painter,
        board: &snake_core::Board,
        layout: &HexLayout,
        cells: &[snake_core::Offset],
        centers: &[Pos2],
        i: usize,
        stroke: Stroke,
    ) {
        let Some(dir) = direction_between(board, cells[i], cells[i + 1]) else {
            return;
        };
        let step = dir_vector(dir, layout.size);
        let direct = centers[i + 1] - centers[i];
        if direct.length() <= 2.2 * layout.size {
            painter.line_segment([centers[i], centers[i + 1]], stroke);
        } else {
            painter.line_segment([centers[i], centers[i] + step * 0.5], stroke);
            painter.line_segment([centers[i + 1], centers[i + 1] - step * 0.5], stroke);
        }
    }

    fn draw_head_marker(&self, painter: &Painter, theme: &Theme, layout: &HexLayout, head: Pos2) {
        let s = layout.size;
        match theme.head_marker {
            HeadMarker::None => {}
            HeadMarker::Ring => {
                painter.circle_stroke(head, s * 0.55, Stroke::new(s * 0.1, Color32::WHITE));
            }
            HeadMarker::Eyes => {
                let food = layout.center(self.state.food());
                let gaze = (food - head).normalized();
                let gaze = if gaze.length_sq() < 0.5 {
                    dir_vector(self.state.direction(), 1.0).normalized()
                } else {
                    gaze
                };
                let perp = Vec2::new(-gaze.y, gaze.x);
                for side in [-1.0, 1.0] {
                    let eye = head + gaze * 0.2 * s + perp * 0.22 * s * side;
                    painter.circle_filled(eye, s * 0.14, Color32::WHITE);
                    painter.circle_filled(eye + gaze * 0.05 * s, s * 0.07, Color32::BLACK);
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

/// Screen-space lattice vector for one hex step (parity-independent).
fn dir_vector(dir: Direction, size: f32) -> Vec2 {
    let (x, y) = match dir {
        Direction::North => (0.0, -SQRT3),
        Direction::NorthEast => (1.5, -SQRT3 / 2.0),
        Direction::SouthEast => (1.5, SQRT3 / 2.0),
        Direction::South => (0.0, SQRT3),
        Direction::SouthWest => (-1.5, SQRT3 / 2.0),
        Direction::NorthWest => (-1.5, -SQRT3 / 2.0),
    };
    Vec2::new(x, y) * size
}

/// The direction leading from `a` to `b`, if they are board neighbors
/// (torus-aware).
fn direction_between(
    board: &snake_core::Board,
    a: snake_core::Offset,
    b: snake_core::Offset,
) -> Option<Direction> {
    Direction::ALL
        .into_iter()
        .find(|d| board.neighbor(a, *d) == Some(b))
}
