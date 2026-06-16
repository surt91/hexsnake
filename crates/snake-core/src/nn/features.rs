//! Sensor feature vector: six rays relative to the current heading.

use crate::coords::Direction;
use crate::game::GameState;

/// Per ray: inverse distance to the blocking cell (wall or body), inverse
/// distance to the own body on the ray, and a signed food-approach score
/// (positive = closer, magnitude scales with 1/food_dist). Two globals:
/// normalized snake length and normalized food distance.
pub const FEATURE_COUNT: usize = 6 * 3 + 2;

impl Direction {
    /// This direction rotated clockwise by `steps` × 60°.
    pub fn rotated_cw(self, steps: u8) -> Direction {
        let idx = Direction::ALL
            .iter()
            .position(|d| *d == self)
            .expect("direction is in ALL");
        Direction::ALL[(idx + steps as usize) % 6]
    }
}

/// Compute the feature vector. Rays are ordered relative to the current
/// heading (index 0 = straight ahead) so the representation is
/// rotation-invariant. Values lie in [-1, 1].
pub fn features(state: &GameState) -> Vec<f32> {
    let board = state.board();
    let head = state.head();
    let food = state.food();
    let max_steps = (board.width + board.height) as u32;
    let food_dist = board.distance(head, food);

    let mut out = Vec::with_capacity(FEATURE_COUNT);
    for step in 0..6u8 {
        let dir = state.direction().rotated_cw(step);

        let mut blocking = None; // wall or body
        let mut body = None;
        let mut cell = head;
        for dist in 1..=max_steps {
            match board.neighbor(cell, dir) {
                None => {
                    blocking = blocking.or(Some(dist));
                    break;
                }
                Some(next) => {
                    if state.occupies(next) && body.is_none() {
                        body = Some(dist);
                        blocking = blocking.or(Some(dist));
                    }
                    cell = next;
                }
            }
            if blocking.is_some() {
                break;
            }
        }

        // Signed food-approach score: positive when moving toward food,
        // negative when moving away. Magnitude = 1/food_dist so the
        // signal strengthens as food gets close. Zero when no neighbor.
        let food_approach = board.neighbor(head, dir).map_or(0.0, |n| {
            let d = board.distance(n, food);
            (food_dist as f32 - d as f32) / food_dist as f32
        });

        out.push(blocking.map_or(0.0, |d| 1.0 / d as f32));
        out.push(body.map_or(0.0, |d| 1.0 / d as f32));
        out.push(food_approach);
    }
    // Global: snake fill ratio and normalized food distance.
    out.push(state.snake_len() as f32 / board.num_cells() as f32);
    out.push(food_dist as f32 / max_steps as f32);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::BoundaryMode;
    use crate::game::Config;

    #[test]
    fn rotation_helper_cycles() {
        assert_eq!(Direction::North.rotated_cw(0), Direction::North);
        assert_eq!(Direction::North.rotated_cw(1), Direction::NorthEast);
        assert_eq!(Direction::North.rotated_cw(3), Direction::South);
        assert_eq!(Direction::NorthWest.rotated_cw(1), Direction::North);
        for d in Direction::ALL {
            assert_eq!(d.rotated_cw(6), d);
            assert_eq!(d.rotated_cw(3), d.opposite());
        }
    }

    #[test]
    fn feature_vector_shape_and_range() {
        for boundary in [BoundaryMode::Walls, BoundaryMode::Periodic] {
            for seed in 0..5 {
                let state = GameState::new(Config {
                    boundary,
                    seed,
                    ..Config::default()
                });
                let f = features(&state);
                assert_eq!(f.len(), FEATURE_COUNT);
                assert!(
                    f.iter().all(|v| (-1.0..=1.0).contains(v)),
                    "features out of range: {f:?}"
                );
            }
        }
    }

    #[test]
    fn fresh_state_sees_own_body_behind() {
        // Heading north with the body extending south: the backward ray
        // (rotation 3) must report the body at distance 1.
        let state = GameState::new(Config::default());
        let f = features(&state);
        let backward_body = f[3 * 3 + 1];
        assert_eq!(backward_body, 1.0, "body directly behind the head");
        // Straight ahead there is no body.
        assert_eq!(f[1], 0.0);
    }

    #[test]
    fn torus_rays_wrap_onto_the_own_body() {
        let walls = GameState::new(Config::default());
        let torus = GameState::new(Config {
            boundary: BoundaryMode::Periodic,
            ..Config::default()
        });
        // Forward ray (north): on walls it hits the wall and the body
        // feature stays 0; on the torus the ray wraps and the only
        // obstacle it can find *is* the body — blocking and body distance
        // must coincide there.
        let fw = features(&walls);
        assert!(fw[0] > 0.0, "wall blocks the forward ray");
        assert_eq!(fw[1], 0.0, "no body ahead on walls");
        let ft = features(&torus);
        assert!(ft[0] > 0.0, "wrapped ray finds the body");
        assert_eq!(ft[0], ft[1], "on the torus the obstacle is the body");
        assert!(ft[0] < fw[0], "torus obstacle is farther than the wall");
    }
}
