//! Skin system: a theme is pure data (palette + drawing style flags), the
//! renderer in `game_view` interprets it. No game-logic access, everything
//! drawn procedurally — no sprite assets.

use eframe::egui::Color32;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ThemeId {
    #[default]
    Classic,
    Honeycomb,
    RetroLcd,
    Neon,
    Natural,
    Colorblind,
}

impl ThemeId {
    pub const ALL: [ThemeId; 6] = [
        ThemeId::Classic,
        ThemeId::Honeycomb,
        ThemeId::RetroLcd,
        ThemeId::Neon,
        ThemeId::Natural,
        ThemeId::Colorblind,
    ];

    pub fn label(self) -> &'static str {
        match self {
            ThemeId::Classic => "Klassisch",
            ThemeId::Honeycomb => "Honigwabe",
            ThemeId::RetroLcd => "Retro-LCD",
            ThemeId::Neon => "Neon",
            ThemeId::Natural => "Naturalistisch",
            ThemeId::Colorblind => "Farbenblind-sicher",
        }
    }

    pub fn theme(self) -> &'static Theme {
        match self {
            ThemeId::Classic => &CLASSIC,
            ThemeId::Honeycomb => &HONEYCOMB,
            ThemeId::RetroLcd => &RETRO_LCD,
            ThemeId::Neon => &NEON,
            ThemeId::Natural => &NATURAL,
            ThemeId::Colorblind => &COLORBLIND,
        }
    }
}

/// How the snake body is rendered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnakeStyle {
    /// Filled inset hexes per occupied cell (classic look).
    Hexes,
    /// Connected band through the cell centers — makes the winding path
    /// visible edge-to-edge ("Schlängeln sichtbar").
    Band { taper: bool, glow: bool },
    /// Chain of circles, like a caterpillar.
    Caterpillar,
}

/// How the food is rendered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FoodStyle {
    /// Plain filled circle.
    Circle,
    /// Drop shape (circle with a tip), e.g. honey.
    Drop,
    /// Monochrome ring (LCD look).
    Ring,
    /// Glowing orb (layered translucent circles).
    GlowOrb,
    /// Apple: circle with stem and leaf.
    Apple,
    /// Upward triangle — differs from the snake *by shape*, for the
    /// colorblind-safe theme.
    Triangle,
}

/// Extra marker on the head so it is identifiable beyond color.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeadMarker {
    None,
    /// White ring (shape cue, colorblind-safe).
    Ring,
    /// Eyes that look toward the food.
    Eyes,
}

pub struct Theme {
    pub background: Color32,
    pub cell_fill: Color32,
    pub cell_border: Color32,
    pub cell_border_width: f32,
    pub wall: Color32,
    pub periodic: Color32,
    pub head: Color32,
    pub body: Color32,
    pub tail: Color32,
    pub food: Color32,
    /// Effects (eat ring, game-over pulse) and small accents.
    pub accent: Color32,
    pub snake_style: SnakeStyle,
    pub food_style: FoodStyle,
    pub head_marker: HeadMarker,
}

pub static CLASSIC: Theme = Theme {
    background: Color32::from_rgb(0x18, 0x18, 0x18),
    cell_fill: Color32::from_rgb(0x24, 0x28, 0x2c),
    cell_border: Color32::from_rgb(0x33, 0x39, 0x40),
    cell_border_width: 1.0,
    wall: Color32::from_rgb(0x8a, 0x80, 0x70),
    periodic: Color32::from_rgb(0x3a, 0xc6, 0xe6),
    head: Color32::from_rgb(0x9a, 0xe6, 0x3a),
    body: Color32::from_rgb(0x3f, 0xa3, 0x4d),
    tail: Color32::from_rgb(0x2a, 0x6e, 0x38),
    food: Color32::from_rgb(0xe6, 0x4d, 0x3a),
    accent: Color32::from_rgb(0xff, 0xd7, 0x4d),
    snake_style: SnakeStyle::Hexes,
    food_style: FoodStyle::Circle,
    head_marker: HeadMarker::None,
};

pub static HONEYCOMB: Theme = Theme {
    background: Color32::from_rgb(0x2b, 0x1d, 0x0a),
    cell_fill: Color32::from_rgb(0xe8, 0xb4, 0x3a),
    cell_border: Color32::from_rgb(0x9c, 0x6f, 0x1c),
    cell_border_width: 2.5,
    wall: Color32::from_rgb(0x5c, 0x3d, 0x12),
    periodic: Color32::from_rgb(0xff, 0xe2, 0x8a),
    head: Color32::from_rgb(0x4a, 0x5d, 0x23),
    body: Color32::from_rgb(0x6b, 0x8e, 0x23),
    tail: Color32::from_rgb(0x8a, 0xa6, 0x3f),
    food: Color32::from_rgb(0xff, 0xb3, 0x00),
    accent: Color32::from_rgb(0xff, 0xe2, 0x8a),
    snake_style: SnakeStyle::Caterpillar,
    food_style: FoodStyle::Drop,
    head_marker: HeadMarker::Eyes,
};

pub static RETRO_LCD: Theme = Theme {
    background: Color32::from_rgb(0x9b, 0xa6, 0x5e),
    cell_fill: Color32::from_rgb(0x96, 0xa1, 0x59),
    cell_border: Color32::from_rgb(0x8a, 0x95, 0x50),
    cell_border_width: 1.0,
    wall: Color32::from_rgb(0x2e, 0x36, 0x14),
    periodic: Color32::from_rgb(0x4a, 0x55, 0x26),
    head: Color32::from_rgb(0x20, 0x26, 0x0e),
    body: Color32::from_rgb(0x2e, 0x36, 0x14),
    tail: Color32::from_rgb(0x2e, 0x36, 0x14),
    food: Color32::from_rgb(0x20, 0x26, 0x0e),
    accent: Color32::from_rgb(0x2e, 0x36, 0x14),
    snake_style: SnakeStyle::Hexes,
    food_style: FoodStyle::Ring,
    head_marker: HeadMarker::None,
};

pub static NEON: Theme = Theme {
    background: Color32::from_rgb(0x07, 0x07, 0x12),
    cell_fill: Color32::from_rgb(0x0d, 0x0d, 0x1c),
    cell_border: Color32::from_rgb(0x1c, 0x16, 0x33),
    cell_border_width: 1.0,
    wall: Color32::from_rgb(0x6a, 0x2d, 0x91),
    periodic: Color32::from_rgb(0xff, 0x3d, 0xcf),
    head: Color32::from_rgb(0xe8, 0xff, 0xff),
    body: Color32::from_rgb(0x18, 0xe6, 0xe6),
    tail: Color32::from_rgb(0x0c, 0x8a, 0xa6),
    food: Color32::from_rgb(0xff, 0x3d, 0xcf),
    accent: Color32::from_rgb(0xff, 0x3d, 0xcf),
    snake_style: SnakeStyle::Band {
        taper: false,
        glow: true,
    },
    food_style: FoodStyle::GlowOrb,
    head_marker: HeadMarker::None,
};

pub static NATURAL: Theme = Theme {
    background: Color32::from_rgb(0x20, 0x2a, 0x18),
    cell_fill: Color32::from_rgb(0x2c, 0x3a, 0x20),
    cell_border: Color32::from_rgb(0x24, 0x30, 0x1a),
    cell_border_width: 1.0,
    wall: Color32::from_rgb(0x6e, 0x5a, 0x3a),
    periodic: Color32::from_rgb(0x9a, 0xc4, 0x6e),
    head: Color32::from_rgb(0x7a, 0xa6, 0x3c),
    body: Color32::from_rgb(0x5d, 0x8a, 0x2e),
    tail: Color32::from_rgb(0x3e, 0x61, 0x20),
    food: Color32::from_rgb(0xd6, 0x3b, 0x2a),
    accent: Color32::from_rgb(0xe8, 0xd4, 0x6a),
    snake_style: SnakeStyle::Band {
        taper: true,
        glow: false,
    },
    food_style: FoodStyle::Apple,
    head_marker: HeadMarker::Eyes,
};

/// Okabe–Ito palette; snake, food and (later) special food also differ by
/// **shape**, not only color.
pub static COLORBLIND: Theme = Theme {
    background: Color32::from_rgb(0x16, 0x16, 0x16),
    cell_fill: Color32::from_rgb(0x26, 0x26, 0x26),
    cell_border: Color32::from_rgb(0x3a, 0x3a, 0x3a),
    cell_border_width: 1.0,
    wall: Color32::from_rgb(0x99, 0x99, 0x99),
    periodic: Color32::from_rgb(0x56, 0xb4, 0xe9),
    head: Color32::from_rgb(0x56, 0xb4, 0xe9),
    body: Color32::from_rgb(0x00, 0x72, 0xb2),
    tail: Color32::from_rgb(0x00, 0x4f, 0x7a),
    food: Color32::from_rgb(0xe6, 0x9f, 0x00),
    accent: Color32::from_rgb(0xf0, 0xe4, 0x42),
    snake_style: SnakeStyle::Hexes,
    food_style: FoodStyle::Triangle,
    head_marker: HeadMarker::Ring,
};
