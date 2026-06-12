//! Pre-game options chosen in the menu.

use serde::{Deserialize, Serialize};
use snake_core::BoundaryMode;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SizePreset {
    Small,
    Medium,
    Large,
    Custom,
}

impl SizePreset {
    pub const ALL: [SizePreset; 4] = [
        SizePreset::Small,
        SizePreset::Medium,
        SizePreset::Large,
        SizePreset::Custom,
    ];

    pub fn label(self) -> &'static str {
        match self {
            SizePreset::Small => "Klein (16×12)",
            SizePreset::Medium => "Mittel (24×18)",
            SizePreset::Large => "Groß (32×24)",
            SizePreset::Custom => "Frei wählbar",
        }
    }
}

// Declaration order is the speed order: Slow < Normal < Fast.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Speed {
    Slow,
    Normal,
    Fast,
}

impl Speed {
    pub const ALL: [Speed; 3] = [Speed::Slow, Speed::Normal, Speed::Fast];

    pub fn label(self) -> &'static str {
        match self {
            Speed::Slow => "Gemütlich",
            Speed::Normal => "Normal",
            Speed::Fast => "Schnell",
        }
    }

    pub fn ticks_per_second(self) -> f64 {
        match self {
            Speed::Slow => 4.0,
            Speed::Normal => 6.0,
            Speed::Fast => 9.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum StrategyChoice {
    #[default]
    Human,
    Chaos,
    Greedy,
    Planner,
}

impl StrategyChoice {
    pub const ALL: [StrategyChoice; 4] = [
        StrategyChoice::Human,
        StrategyChoice::Chaos,
        StrategyChoice::Greedy,
        StrategyChoice::Planner,
    ];

    pub fn label(self) -> &'static str {
        match self {
            StrategyChoice::Human => "Aus (selbst spielen)",
            StrategyChoice::Chaos => "Chaos-Walker",
            StrategyChoice::Greedy => "Greedy",
            StrategyChoice::Planner => "Pfadplaner",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Settings {
    pub boundary: BoundaryMode,
    pub preset: SizePreset,
    pub custom_width: i32,
    pub custom_height: i32,
    pub speed: Speed,
    /// Autopilot strategy; `Human` disables it. `serde(default)` keeps old
    /// stored settings loadable.
    #[serde(default)]
    pub strategy: StrategyChoice,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            boundary: BoundaryMode::Walls,
            preset: SizePreset::Medium,
            custom_width: 20,
            custom_height: 15,
            speed: Speed::Normal,
            strategy: StrategyChoice::default(),
        }
    }
}

impl Settings {
    pub fn dimensions(&self) -> (i32, i32) {
        match self.preset {
            SizePreset::Small => (16, 12),
            SizePreset::Medium => (24, 18),
            SizePreset::Large => (32, 24),
            SizePreset::Custom => (self.custom_width, self.custom_height),
        }
    }
}
