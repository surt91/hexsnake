//! Snapshot tests via egui_kittest: render deterministic game states
//! (fixed seed + scripted inputs + simulated clock) and compare against
//! reference images. Update references with `UPDATE_SNAPSHOTS=1`.

use egui_kittest::Harness;

use crate::app::parse_input_script;
use crate::game_view::GameSession;
use crate::settings::{Settings, SizePreset};
use crate::theme::ThemeId;

/// Eats twice, then heads for the north wall (greedy script for seed 42 on
/// the 24×18 default board) — gives every theme a snake with a few bends.
const SCRIPT: &str = "assaaaaaaaaqqqwwww";
const SEED: u64 = 42;

fn session(theme: ThemeId) -> GameSession {
    let settings = Settings {
        preset: SizePreset::Medium,
        theme,
        ..Default::default()
    };
    GameSession::new(&settings, SEED, &parse_input_script(SCRIPT))
}

#[test]
fn theme_snapshots() {
    // One merged result set: all themes are rendered before the test
    // reports, and UPDATE_SNAPSHOTS handles them consistently.
    let mut results = egui_kittest::SnapshotResults::new();
    for theme in ThemeId::ALL {
        let mut game = session(theme);
        let mut harness = Harness::builder()
            .with_size(eframe::egui::Vec2::new(900.0, 700.0))
            .build_ui(move |ui| {
                game.ui(ui, false);
            });
        // The simulated kittest clock advances deterministically per step;
        // enough steps for the script's ticks to play out.
        for _ in 0..12 {
            harness.step();
        }
        harness.snapshot(format!("theme_{}", format!("{theme:?}").to_lowercase()));
        results.extend(harness.take_snapshot_results());
    }
}
