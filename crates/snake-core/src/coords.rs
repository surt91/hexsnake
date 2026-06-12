//! Hex coordinates for flat-top hexagons.
//!
//! Internally we use axial coordinates `(q, r)` for neighborhood and
//! distance math, and offset coordinates `(col, row)` ("odd-q" vertical
//! layout) for the rectangular playing field, where periodic boundaries
//! are a simple modulo. See Red Blob Games, "Hexagonal Grids".

/// Axial hex coordinate (flat-top). `q` grows to the east, `r` to the south.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Axial {
    pub q: i32,
    pub r: i32,
}

/// Offset coordinate on the rectangular field ("odd-q" vertical layout).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Offset {
    pub col: i32,
    pub row: i32,
}

impl Axial {
    pub const fn new(q: i32, r: i32) -> Self {
        Self { q, r }
    }

    /// Convert to offset coordinates (odd-q vertical layout).
    pub fn to_offset(self) -> Offset {
        // `q - (q & 1)` is always even, so truncating division is exact.
        let col = self.q;
        let row = self.r + (self.q - (self.q & 1)) / 2;
        Offset { col, row }
    }

    /// The neighboring hex in the given direction (unbounded grid).
    pub fn neighbor(self, dir: Direction) -> Axial {
        let (dq, dr) = dir.axial_delta();
        Axial::new(self.q + dq, self.r + dr)
    }

    /// Hex distance on the unbounded grid.
    pub fn distance(self, other: Axial) -> u32 {
        let dq = self.q - other.q;
        let dr = self.r - other.r;
        (dq.unsigned_abs() + dr.unsigned_abs() + (dq + dr).unsigned_abs()) / 2
    }
}

impl Offset {
    pub const fn new(col: i32, row: i32) -> Self {
        Self { col, row }
    }

    /// Convert to axial coordinates (odd-q vertical layout).
    pub fn to_axial(self) -> Axial {
        let q = self.col;
        let r = self.row - (self.col - (self.col & 1)) / 2;
        Axial { q, r }
    }
}

/// The six movement directions on a flat-top hex grid,
/// mapped to the keys W, E, D, S, A, Q.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Direction {
    North,
    NorthEast,
    SouthEast,
    South,
    SouthWest,
    NorthWest,
}

impl Direction {
    pub const ALL: [Direction; 6] = [
        Direction::North,
        Direction::NorthEast,
        Direction::SouthEast,
        Direction::South,
        Direction::SouthWest,
        Direction::NorthWest,
    ];

    /// The opposite direction (180° turn, forbidden while moving).
    pub fn opposite(self) -> Direction {
        match self {
            Direction::North => Direction::South,
            Direction::NorthEast => Direction::SouthWest,
            Direction::SouthEast => Direction::NorthWest,
            Direction::South => Direction::North,
            Direction::SouthWest => Direction::NorthEast,
            Direction::NorthWest => Direction::SouthEast,
        }
    }

    /// Axial coordinate delta `(dq, dr)` for one step in this direction.
    /// Flat-top layout: `q` grows east, `r` grows south.
    pub fn axial_delta(self) -> (i32, i32) {
        match self {
            Direction::North => (0, -1),
            Direction::NorthEast => (1, -1),
            Direction::SouthEast => (1, 0),
            Direction::South => (0, 1),
            Direction::SouthWest => (-1, 1),
            Direction::NorthWest => (-1, 0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn axial_offset_roundtrip() {
        for q in -5..=5 {
            for r in -5..=5 {
                let a = Axial::new(q, r);
                assert_eq!(a.to_offset().to_axial(), a, "axial roundtrip for {a:?}");
            }
        }
        for col in -5..=5 {
            for row in -5..=5 {
                let o = Offset::new(col, row);
                assert_eq!(o.to_axial().to_offset(), o, "offset roundtrip for {o:?}");
            }
        }
    }

    #[test]
    fn neighbor_and_opposite_cancel() {
        for q in -2..=2 {
            for r in -2..=2 {
                let a = Axial::new(q, r);
                for dir in Direction::ALL {
                    assert_eq!(a.neighbor(dir).neighbor(dir.opposite()), a);
                }
            }
        }
    }

    #[test]
    fn neighbors_have_distance_one() {
        let a = Axial::new(3, -2);
        for dir in Direction::ALL {
            assert_eq!(a.distance(a.neighbor(dir)), 1);
        }
        assert_eq!(a.distance(a), 0);
    }

    #[test]
    fn opposite_is_involution() {
        for dir in Direction::ALL {
            assert_ne!(dir.opposite(), dir);
            assert_eq!(dir.opposite().opposite(), dir);
        }
    }

    #[test]
    fn north_decreases_row_in_offset() {
        // In offset coordinates, a North step decreases the row by exactly 1
        // regardless of column parity.
        for col in 0..4 {
            let o = Offset::new(col, 3);
            let n = o.to_axial().neighbor(Direction::North).to_offset();
            assert_eq!(n, Offset::new(col, 2));
        }
    }

    #[test]
    fn distance_is_symmetric() {
        let a = Axial::new(-3, 5);
        let b = Axial::new(4, -1);
        assert_eq!(a.distance(b), b.distance(a));
    }
}
