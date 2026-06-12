//! The rectangular hex playing field with switchable boundary behavior.

use crate::coords::{Direction, Offset};

/// Behavior at the edge of the field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum BoundaryMode {
    /// Hitting the edge ends the game.
    Walls,
    /// The field wraps around (torus): leaving on one side re-enters on the
    /// opposite side.
    Periodic,
}

/// A rectangular field of hexes, `width` columns × `height` rows in offset
/// coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Board {
    pub width: i32,
    pub height: i32,
    pub boundary: BoundaryMode,
}

impl Board {
    pub fn new(width: i32, height: i32, boundary: BoundaryMode) -> Self {
        assert!(width > 0 && height > 0, "board must be non-empty");
        Self {
            width,
            height,
            boundary,
        }
    }

    pub fn contains(&self, cell: Offset) -> bool {
        (0..self.width).contains(&cell.col) && (0..self.height).contains(&cell.row)
    }

    pub fn num_cells(&self) -> usize {
        (self.width as usize) * (self.height as usize)
    }

    /// All cells in deterministic row-major order.
    pub fn cells(&self) -> impl Iterator<Item = Offset> + '_ {
        (0..self.height).flat_map(|row| (0..self.width).map(move |col| Offset::new(col, row)))
    }

    /// Wrap an offset coordinate onto the field (torus).
    pub fn wrap(&self, cell: Offset) -> Offset {
        Offset::new(
            cell.col.rem_euclid(self.width),
            cell.row.rem_euclid(self.height),
        )
    }

    /// The neighboring cell in the given direction, respecting the boundary
    /// mode: `None` means "wall".
    pub fn neighbor(&self, cell: Offset, dir: Direction) -> Option<Offset> {
        debug_assert!(self.contains(cell));
        let stepped = cell.to_axial().neighbor(dir).to_offset();
        match self.boundary {
            BoundaryMode::Walls => self.contains(stepped).then_some(stepped),
            BoundaryMode::Periodic => Some(self.wrap(stepped)),
        }
    }

    /// Hex distance between two cells. With periodic boundaries this is the
    /// torus distance: the minimum over all wrap variants of `b`.
    pub fn distance(&self, a: Offset, b: Offset) -> u32 {
        match self.boundary {
            BoundaryMode::Walls => a.to_axial().distance(b.to_axial()),
            BoundaryMode::Periodic => {
                let a = a.to_axial();
                let mut best = u32::MAX;
                for dcol in [-self.width, 0, self.width] {
                    for drow in [-self.height, 0, self.height] {
                        let shifted = Offset::new(b.col + dcol, b.row + drow).to_axial();
                        best = best.min(a.distance(shifted));
                    }
                }
                best
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn board(boundary: BoundaryMode) -> Board {
        Board::new(8, 6, boundary)
    }

    #[test]
    fn walls_block_all_edges() {
        let b = board(BoundaryMode::Walls);
        // Top and bottom edge.
        for col in 0..b.width {
            assert_eq!(b.neighbor(Offset::new(col, 0), Direction::North), None);
            assert_eq!(
                b.neighbor(Offset::new(col, b.height - 1), Direction::South),
                None
            );
        }
        // Left and right edge: NW/SW resp. NE/SE must not leave the field.
        for row in 0..b.height {
            for dir in [Direction::NorthWest, Direction::SouthWest] {
                let n = b.neighbor(Offset::new(0, row), dir);
                assert!(n.is_none(), "left edge {row} {dir:?} gave {n:?}");
            }
            for dir in [Direction::NorthEast, Direction::SouthEast] {
                let n = b.neighbor(Offset::new(b.width - 1, row), dir);
                assert!(n.is_none(), "right edge {row} {dir:?} gave {n:?}");
            }
        }
    }

    #[test]
    fn periodic_wraps_all_edges() {
        let b = board(BoundaryMode::Periodic);
        for cell in b.cells().collect::<Vec<_>>() {
            for dir in Direction::ALL {
                let n = b.neighbor(cell, dir).expect("no walls on a torus");
                assert!(b.contains(n), "neighbor {n:?} of {cell:?} is off-board");
                assert_ne!(n, cell);
            }
        }
        // Explicit wrap examples on the straight axes.
        assert_eq!(
            b.neighbor(Offset::new(3, 0), Direction::North),
            Some(Offset::new(3, b.height - 1))
        );
        assert_eq!(
            b.neighbor(Offset::new(3, b.height - 1), Direction::South),
            Some(Offset::new(3, 0))
        );
    }

    #[test]
    fn periodic_neighbors_have_torus_distance_one() {
        let b = board(BoundaryMode::Periodic);
        for cell in b.cells().collect::<Vec<_>>() {
            for dir in Direction::ALL {
                let n = b.neighbor(cell, dir).unwrap();
                assert_eq!(
                    b.distance(cell, n),
                    1,
                    "{cell:?} -> {dir:?} -> {n:?} should have torus distance 1"
                );
            }
        }
    }

    #[test]
    fn torus_distance_never_exceeds_direct_distance() {
        let b = board(BoundaryMode::Periodic);
        let direct = board(BoundaryMode::Walls);
        for a in b.cells().collect::<Vec<_>>() {
            for c in b.cells().collect::<Vec<_>>() {
                assert!(b.distance(a, c) <= direct.distance(a, c));
                assert_eq!(
                    b.distance(a, c),
                    b.distance(c, a),
                    "torus distance symmetric"
                );
            }
        }
    }

    #[test]
    fn torus_distance_wraps_across_edge() {
        let b = board(BoundaryMode::Periodic);
        // Opposite corners are close on the torus.
        let d = b.distance(Offset::new(0, 0), Offset::new(b.width - 1, b.height - 1));
        assert!(d <= 2, "corner-to-corner torus distance was {d}");
    }

    #[test]
    fn walls_distance_is_plain_hex_distance() {
        let b = board(BoundaryMode::Walls);
        assert_eq!(b.distance(Offset::new(0, 0), Offset::new(0, 5)), 5);
    }
}
