//! Pixel-space layout for a rectangular field of flat-top hexagons.

use eframe::egui::{Pos2, Rect, Vec2};
use snake_core::{Board, Offset};

const SQRT3: f32 = 1.732_050_8;

/// Maps offset cell coordinates to screen positions. Flat-top hexes,
/// "odd-q" layout: odd columns are shifted down by half a hex.
pub struct HexLayout {
    /// Screen position of the center of cell (0, 0).
    origin: Pos2,
    /// Hex circumradius (center to corner) in pixels.
    pub size: f32,
}

impl HexLayout {
    /// Fit the board into `rect`, centered, preserving hex aspect ratio.
    pub fn fit(rect: Rect, board: &Board) -> Self {
        let cols = board.width as f32;
        let rows = board.height as f32;
        // Total grid extent in units of the hex size.
        let grid_w = 1.5 * cols + 0.5;
        let grid_h = SQRT3 * (rows + 0.5);
        let size = (rect.width() / grid_w).min(rect.height() / grid_h);
        let top_left = rect.center() - 0.5 * size * Vec2::new(grid_w, grid_h);
        let origin = top_left + size * Vec2::new(1.0, SQRT3 / 2.0);
        Self { origin, size }
    }

    pub fn center(&self, cell: Offset) -> Pos2 {
        let parity = (cell.col & 1) as f32;
        self.origin
            + self.size
                * Vec2::new(
                    1.5 * cell.col as f32,
                    SQRT3 * (cell.row as f32 + 0.5 * parity),
                )
    }

    /// The six corners of a cell, optionally scaled toward the center
    /// (`scale` < 1.0 gives an inset hex, used to render snake segments).
    pub fn corners(&self, cell: Offset, scale: f32) -> Vec<Pos2> {
        let c = self.center(cell);
        (0..6)
            .map(|i| {
                let angle = std::f32::consts::TAU * i as f32 / 6.0;
                c + scale * self.size * Vec2::new(angle.cos(), angle.sin())
            })
            .collect()
    }

    /// Bounding rectangle of the whole grid in screen space.
    pub fn bounds(&self, board: &Board) -> Rect {
        let top_left = self.origin - self.size * Vec2::new(1.0, SQRT3 / 2.0);
        let size = self.size
            * Vec2::new(
                1.5 * board.width as f32 + 0.5,
                SQRT3 * (board.height as f32 + 0.5),
            );
        Rect::from_min_size(top_left, size)
    }
}
