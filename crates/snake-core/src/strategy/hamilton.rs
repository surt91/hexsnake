use super::{cell_index, doomed_move, safe_moves, Strategy, StrategyDebug};
use crate::board::Board;
use crate::coords::{Direction, Offset};
use crate::game::GameState;

/// How many cells of the upcoming cycle the debug overlay shows.
const DEBUG_PATH_LEN: usize = 24;
/// Extra ticks of clearance when checking a shortcut against the snake's
/// vacate times — absorbs growth from food eaten along the way.
const SHORTCUT_SAFETY_MARGIN: u32 = 2;

/// Cycle steps from `from` to `to` in cycle direction.
pub(crate) fn cyclic_offset(len: usize, from: usize, to: usize) -> usize {
    (to + len - from) % len
}

/// Is jumping to cycle position `target` safe? We assume the snake then
/// follows the plain cycle: the cell reached after `k` further steps is
/// entered at tick `1 + k`, which must be at or after the tick the body
/// vacates it (plus a safety margin for growth). `cycle` is the cycle in
/// order; `vacate[cell]` is the tick a body cell frees (0 = free).
fn shortcut_is_safe(
    board: &Board,
    cycle: &[Offset],
    target: usize,
    vacate: &[u32],
    snake_len: usize,
) -> bool {
    let lookahead = snake_len as u32 + SHORTCUT_SAFETY_MARGIN;
    for k in 0..=lookahead {
        let cell = cycle[(target + k as usize) % cycle.len()];
        let vacates = vacate[cell_index(board, cell)];
        if vacates != 0 && 1 + k < vacates + SHORTCUT_SAFETY_MARGIN {
            return false;
        }
    }
    true
}

/// Follow the given Hamiltonian cycle from the head, taking a provably safe
/// shortcut when one exists. Shared by [`HamiltonRider`] (static cycle) and
/// the `CycleSurgeon` (dynamically repaired cycle). `cycle` is the cell
/// sequence in cycle order; `position[cell_index]` is each cell's index in
/// that sequence. Fills `debug.path` with the upcoming cycle stretch.
pub(crate) fn ride_cycle(
    state: &GameState,
    cycle: &[Offset],
    position: &[usize],
    debug: &mut StrategyDebug,
) -> Direction {
    let board = state.board();
    let head_pos = position[cell_index(board, state.head())];
    let food_offset = cyclic_offset(
        cycle.len(),
        head_pos,
        position[cell_index(board, state.food())],
    );

    let mut vacate = vec![0u32; board.num_cells()];
    let len = state.snake_len() as u32;
    for (i, cell) in state.snake().enumerate() {
        vacate[cell_index(board, cell)] = len - i as u32;
    }

    // Shortcuts only while the board is mostly empty; afterwards the
    // plain cycle is the only provably safe route.
    let shortcuts_allowed = state.snake_len() * 2 < board.num_cells();

    let mut best: Option<(Direction, usize)> = None;
    for (dir, cell) in safe_moves(state) {
        let target = position[cell_index(board, cell)];
        let offset = cyclic_offset(cycle.len(), head_pos, target);
        // Never move "backwards" along the cycle and never beyond the
        // food (it would have to come around again).
        if offset == 0 || offset > food_offset {
            continue;
        }
        if offset > 1
            && !(shortcuts_allowed
                && shortcut_is_safe(board, cycle, target, &vacate, state.snake_len()))
        {
            continue;
        }
        if best.is_none_or(|(_, o)| offset > o) {
            best = Some((dir, offset));
        }
    }

    let chosen = best.map(|(dir, _)| dir).or_else(|| {
        // No shortcut and the plain successor was filtered out (e.g.
        // it lies beyond the food in cycle order): follow the cycle.
        let next = cycle[(head_pos + 1) % cycle.len()];
        safe_moves(state)
            .into_iter()
            .find(|(_, cell)| *cell == next)
            .map(|(dir, _)| dir)
    });

    // Debug: show the upcoming stretch of the cycle.
    debug.path.clear();
    if let Some(dir) = chosen {
        if let Some(cell) = board.neighbor(state.head(), dir) {
            let mut pos = position[cell_index(board, cell)];
            for _ in 0..DEBUG_PATH_LEN {
                debug.path.push(cycle[pos]);
                pos = (pos + 1) % cycle.len();
            }
        }
    }

    chosen.unwrap_or_else(|| {
        safe_moves(state)
            .first()
            .map(|(dir, _)| *dir)
            .unwrap_or_else(|| doomed_move(state))
    })
}

/// Build a serpentine Hamiltonian cycle over the offset rectangle, or
/// `None` if the dimensions are incompatible.
///
/// Construction: column 0 is the northbound return lane; the remaining
/// columns are traversed row by row in boustrophedon order. This works on
/// the hex grid because a constant-`row` west–east walk exists (alternating
/// SE/NE steps in "odd-q" layout) and N/S steps stay in their column.
/// Requires an even number of rows.
pub fn serpentine_cycle(width: i32, height: i32) -> Option<Vec<Offset>> {
    if width < 2 || height < 2 || height % 2 != 0 {
        return None;
    }
    let mut cycle = Vec::with_capacity((width * height) as usize);
    cycle.push(Offset::new(0, 0));
    for row in 0..height {
        if row % 2 == 0 {
            for col in 1..width {
                cycle.push(Offset::new(col, row));
            }
        } else {
            for col in (1..width).rev() {
                cycle.push(Offset::new(col, row));
            }
        }
    }
    for row in (1..height).rev() {
        cycle.push(Offset::new(0, row));
    }
    Some(cycle)
}

/// Hamiltonian-cycle follower with shortcuts: in principle it can never
/// die; shortcuts along the cycle order keep it from taking forever.
pub struct HamiltonRider {
    cycle: Vec<Offset>,
    /// Cell index → position in the cycle.
    position: Vec<usize>,
    debug: StrategyDebug,
}

impl HamiltonRider {
    /// Whether the serpentine construction exists for these dimensions.
    pub fn compatible(width: i32, height: i32) -> bool {
        serpentine_cycle(width, height).is_some()
    }

    /// `None` if the board dimensions are incompatible (odd height).
    pub fn new(board: &Board) -> Option<Self> {
        let cycle = serpentine_cycle(board.width, board.height)?;
        let mut position = vec![0; board.num_cells()];
        for (i, cell) in cycle.iter().enumerate() {
            position[cell_index(board, *cell)] = i;
        }
        Some(Self {
            cycle,
            position,
            debug: StrategyDebug::default(),
        })
    }
}

impl Strategy for HamiltonRider {
    fn next_move(&mut self, state: &GameState) -> Direction {
        ride_cycle(state, &self.cycle, &self.position, &mut self.debug)
    }

    fn debug(&self) -> Option<&StrategyDebug> {
        Some(&self.debug)
    }
}

/// Assert that `order` is a valid Hamiltonian cycle over `board`: every cell
/// exactly once, consecutive cells (and last→first) hex-adjacent. Shared by
/// the `HamiltonRider` and `CycleSurgeon` tests (arbiter for the surgeon's
/// per-tick invariant).
#[cfg(test)]
pub(crate) fn assert_valid_cycle_order(board: &Board, order: &[Offset]) {
    assert_eq!(order.len(), board.num_cells(), "visits every cell once");
    let mut seen = vec![false; board.num_cells()];
    for cell in order {
        assert!(board.contains(*cell), "cell {cell:?} off board");
        let idx = cell_index(board, *cell);
        assert!(!seen[idx], "cell {cell:?} visited twice");
        seen[idx] = true;
    }
    for i in 0..order.len() {
        let a = order[i];
        let b = order[(i + 1) % order.len()];
        let adjacent = Direction::ALL
            .into_iter()
            .any(|d| board.neighbor(a, d) == Some(b));
        assert!(adjacent, "{a:?} -> {b:?} not adjacent (step {i})");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::BoundaryMode;
    use crate::game::{Config, Status};

    /// Every generated cycle must visit each cell exactly once, with all
    /// consecutive cells (and last→first) being hex neighbors.
    fn assert_valid_cycle(width: i32, height: i32) {
        let board = Board::new(width, height, BoundaryMode::Walls);
        let cycle = serpentine_cycle(width, height)
            .unwrap_or_else(|| panic!("no cycle for {width}x{height}"));
        assert_valid_cycle_order(&board, &cycle);
    }

    #[test]
    fn all_three_presets_are_hamilton_compatible() {
        assert_valid_cycle(16, 12);
        assert_valid_cycle(24, 18);
        assert_valid_cycle(32, 24);
    }

    #[test]
    fn small_and_odd_width_boards_work_too() {
        assert_valid_cycle(2, 2);
        assert_valid_cycle(7, 6);
        assert_valid_cycle(9, 4);
    }

    #[test]
    fn odd_height_is_incompatible() {
        assert!(serpentine_cycle(16, 11).is_none());
        assert!(serpentine_cycle(5, 5).is_none());
        assert!(!HamiltonRider::compatible(16, 11));
    }

    #[test]
    fn rider_survives_and_scores_in_both_modes() {
        for boundary in [BoundaryMode::Walls, BoundaryMode::Periodic] {
            let config = Config {
                width: 16,
                height: 12,
                boundary,
                seed: 5,
            };
            let mut state = GameState::new(config);
            let mut rider = HamiltonRider::new(state.board()).unwrap();
            for _ in 0..8_000 {
                if state.status() != Status::Running {
                    break;
                }
                let dir = rider.next_move(&state);
                state.tick(Some(dir));
            }
            assert_ne!(
                state.status(),
                Status::GameOver,
                "hamilton rider must not die ({boundary:?}), score {}",
                state.score()
            );
            assert!(
                state.score() >= 20,
                "rider should eat steadily ({boundary:?}), got {}",
                state.score()
            );
        }
    }
}
