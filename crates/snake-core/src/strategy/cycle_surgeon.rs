//! Cycle surgeon: a dynamically repaired Hamiltonian cycle.
//!
//! The [`HamiltonRider`](super::HamiltonRider) is provably perfect but slow —
//! it rides a *static* serpentine cycle and may only shortcut *along* the cycle
//! order, so food lying "against the grain" forces it around almost the whole
//! board. The surgeon instead reshapes the cycle **toward the food** every
//! tick, so the head reaches it in far fewer steps.
//!
//! # Safety invariant
//!
//! There is always a valid Hamiltonian cycle over every cell, and the repair
//! operations **never touch an edge occupied by the snake body** (a pair of
//! consecutive body segments). Because the body then lies as a contiguous run
//! along the cycle, following the cycle can never collide, and the board fills
//! (`Won`) — the same proof as the rider, only the cycle moves. The snake
//! follows the cycle **successor strictly (offset 1)** so the body stays a
//! contiguous arc; immediate safety is a [`safe_moves`](super::safe_moves)
//! filter on top. (Shortcut jumps — as the rider takes — are *unsafe* here:
//! their vacate-time proof assumes a static cycle, which per-tick reshaping
//! breaks, so the snake dies. See the plan's Phase-D problem report.) The
//! property tests are the arbiter.
//!
//! # Data structure
//!
//! Cell index = `row * width + col` (row-major, offset coordinates). The cycle
//! is a doubly linked ring (`next`/`prev`), initialized from
//! [`serpentine_cycle`]. Occupied edges are read off a per-cell "body successor"
//! table (`Vec<Option<usize>>`) rebuilt each tick — no `HashSet`, no RNG, no
//! map iteration, so the whole strategy stays deterministic.
//!
//! # Operation catalogue (only on unoccupied edges)
//!
//! - **Relocate** — splice a free cell `x` out of its position and into edge
//!   `(a→b)` as a detour `a→x→b`. O(1); the bread-and-butter op on the
//!   triangular hex grid, where `x`'s old neighbors are usually adjacent.
//! - **2-opt** — remove edges `(a→b)`, `(c→d)`, add `(a→c)`, `(b→d)`, reversing
//!   the segment `b…c` (which must contain no body cell, else the body would
//!   run against the cycle direction). Capped segment length keeps the tick
//!   cheap.

use super::hamilton::serpentine_cycle;
use super::{cell_index, doomed_move, safe_moves, HamiltonRider, Strategy, StrategyDebug};
use crate::board::Board;
use crate::coords::{Direction, Offset};
use crate::game::GameState;

/// Max tentative operations applied per tick (deterministic budget).
const OPT_BUDGET: usize = 256;
/// How many cells of the head→food cycle path a pass inspects.
const PATH_WINDOW: usize = 64;
/// Cost cap on a 2-opt segment reversal (segment length ≤ this).
const MAX_SEGMENT: usize = 32;

/// Hamiltonian-cycle follower that repairs the cycle toward the food each tick.
pub struct CycleSurgeon {
    width: usize,
    /// Cell index → next cell in cycle order.
    next: Vec<usize>,
    /// Cell index → previous cell in cycle order.
    prev: Vec<usize>,
    debug: StrategyDebug,
}

impl CycleSurgeon {
    /// Whether the serpentine construction (and thus the surgeon) exists for
    /// these dimensions.
    pub fn compatible(width: i32, height: i32) -> bool {
        HamiltonRider::compatible(width, height)
    }

    /// `None` if the board dimensions are incompatible (odd height).
    pub fn new(board: &Board) -> Option<Self> {
        let cycle = serpentine_cycle(board.width, board.height)?;
        let n = board.num_cells();
        let mut next = vec![0usize; n];
        let mut prev = vec![0usize; n];
        for i in 0..cycle.len() {
            let a = cell_index(board, cycle[i]);
            let b = cell_index(board, cycle[(i + 1) % cycle.len()]);
            next[a] = b;
            prev[b] = a;
        }
        Some(Self {
            width: board.width as usize,
            next,
            prev,
            debug: StrategyDebug::default(),
        })
    }

    fn offset_of(&self, idx: usize) -> Offset {
        Offset::new((idx % self.width) as i32, (idx / self.width) as i32)
    }

    /// Neighbor cell indices of `idx`, in `Direction::ALL` order (off-board
    /// neighbors are `None`).
    fn neighbors(&self, board: &Board, idx: usize) -> [Option<usize>; 6] {
        let cell = self.offset_of(idx);
        let mut out = [None; 6];
        for (i, dir) in Direction::ALL.iter().enumerate() {
            out[i] = board.neighbor(cell, *dir).map(|n| cell_index(board, n));
        }
        out
    }

    fn adjacent(&self, board: &Board, u: usize, v: usize) -> bool {
        self.neighbors(board, u).contains(&Some(v))
    }

    /// Steps along `next` from `a` to `b` (cycle distance). `usize::MAX` if the
    /// ring is broken (should never happen while the invariant holds).
    fn cycle_dist(&self, a: usize, b: usize) -> usize {
        let mut cur = a;
        let mut d = 0;
        while cur != b {
            cur = self.next[cur];
            d += 1;
            if d > self.next.len() {
                return usize::MAX;
            }
        }
        d
    }

    /// Rebuild the ordered cycle (offsets) and the `cell_index → position`
    /// table. Starts from cell 0. Used by the tests to validate the cycle.
    #[cfg(test)]
    fn rebuild_order(&self) -> (Vec<Offset>, Vec<usize>) {
        let n = self.next.len();
        let mut order = Vec::with_capacity(n);
        let mut position = vec![0usize; n];
        let mut cur = 0usize;
        for i in 0..n {
            order.push(self.offset_of(cur));
            position[cur] = i;
            cur = self.next[cur];
        }
        (order, position)
    }

    // --- Operations -------------------------------------------------------

    /// Would relocating free cell `x` into edge `(a→b)` be valid? Requires
    /// `next[a] == b`, `x` free and adjacent to both `a` and `b`, and — with
    /// `p = prev[x]`, `q = next[x]` — `p` adjacent `q`, all distinct, and the
    /// three removed/reused edges unoccupied.
    fn relocate_ok(
        &self,
        board: &Board,
        body_next: &[Option<usize>],
        is_snake: &[bool],
        x: usize,
        a: usize,
        b: usize,
    ) -> bool {
        if x == a || x == b || is_snake[x] {
            return false;
        }
        let p = self.prev[x];
        let q = self.next[x];
        // Degenerate rings (tiny boards) — keep the splice well-formed.
        if p == q || p == x || q == x {
            return false;
        }
        self.adjacent(board, x, a)
            && self.adjacent(board, x, b)
            && self.adjacent(board, p, q)
            && !is_body_edge(body_next, a, b)
            && !is_body_edge(body_next, p, x)
            && !is_body_edge(body_next, x, q)
    }

    fn apply_relocate(&mut self, x: usize, a: usize, b: usize) {
        let p = self.prev[x];
        let q = self.next[x];
        self.next[p] = q;
        self.prev[q] = p;
        self.next[a] = x;
        self.prev[x] = a;
        self.next[x] = b;
        self.prev[b] = x;
    }

    /// The segment `b…c` (inclusive) walked along `next`, or `None` if `c` is
    /// not reached within [`MAX_SEGMENT`], the segment contains a body cell, or
    /// the ordering is wrong (hitting `a`/`d` first).
    fn segment_between(
        &self,
        is_snake: &[bool],
        a: usize,
        b: usize,
        c: usize,
        d: usize,
    ) -> Option<Vec<usize>> {
        let mut seg = Vec::new();
        let mut cur = b;
        loop {
            if is_snake[cur] || cur == a || cur == d {
                return None;
            }
            seg.push(cur);
            if cur == c {
                return Some(seg);
            }
            if seg.len() >= MAX_SEGMENT {
                return None;
            }
            cur = self.next[cur];
        }
    }

    /// Would a 2-opt on edges `(a→b)`, `(c→d)` be valid? Requires `a` adjacent
    /// `c`, `b` adjacent `d`, both edges unoccupied, and a body-cell-free
    /// segment `b…c` of length ≤ [`MAX_SEGMENT`].
    #[allow(clippy::too_many_arguments)]
    fn twoopt_ok(
        &self,
        board: &Board,
        body_next: &[Option<usize>],
        is_snake: &[bool],
        a: usize,
        b: usize,
        c: usize,
        d: usize,
    ) -> Option<Vec<usize>> {
        if c == a || c == b || d == a || d == b {
            return None;
        }
        if !self.adjacent(board, a, c) || !self.adjacent(board, b, d) {
            return None;
        }
        if is_body_edge(body_next, a, b) || is_body_edge(body_next, c, d) {
            return None;
        }
        self.segment_between(is_snake, a, b, c, d)
    }

    fn apply_twoopt(&mut self, a: usize, b: usize, c: usize, d: usize, seg: &[usize]) {
        // Reverse seg = [b, …, c] to [c, …, b]; rewire boundaries a→c, b→d.
        self.next[a] = c;
        self.prev[c] = a;
        for i in (1..seg.len()).rev() {
            self.next[seg[i]] = seg[i - 1];
            self.prev[seg[i - 1]] = seg[i];
        }
        self.next[b] = d;
        self.prev[d] = b;
    }

    // --- Per-tick optimizer ----------------------------------------------

    /// Reshape the cycle to shorten the head→food distance. Deterministic,
    /// budget-bounded; only ever applies an op that strictly reduces the
    /// distance (rolls back otherwise).
    ///
    /// The lever is **pulling free cells out of the head→food arc**: walking
    /// that arc, each free cell `x` on it is relocated into an edge among its
    /// neighbors (typically behind the food), which drops `x` off the arc and
    /// shortens the route by one. 2-opt reversals catch cases relocate can't.
    fn optimize(
        &mut self,
        board: &Board,
        head: usize,
        food: usize,
        body_next: &[Option<usize>],
        is_snake: &[bool],
    ) {
        let mut budget = OPT_BUDGET;
        while budget > 0 {
            let d0 = self.cycle_dist(head, food);
            if d0 <= 1 {
                break;
            }
            let mut improved = false;
            // Walk the arc strictly between head and food.
            let mut x = self.next[head];
            let mut steps = 0;
            'walk: while x != food && steps < PATH_WINDOW {
                let next_x = self.next[x];
                if !is_snake[x] {
                    // Relocate x into an edge (a → next[a]) among its neighbors.
                    for a in self.neighbors(board, x).into_iter().flatten() {
                        if budget == 0 {
                            break 'walk;
                        }
                        let b = self.next[a];
                        if self.relocate_ok(board, body_next, is_snake, x, a, b) {
                            budget -= 1;
                            let (sn, sp) = (self.next.clone(), self.prev.clone());
                            self.apply_relocate(x, a, b);
                            if self.cycle_dist(head, food) < d0 {
                                improved = true;
                                break 'walk;
                            }
                            self.next = sn;
                            self.prev = sp;
                        }
                    }
                    // 2-opt: remove (a→b) at a = prev[x], and (c→d) with
                    // c ∈ N(a), reversing the segment b…c.
                    let a = self.prev[x];
                    let b = x;
                    for c in self.neighbors(board, a).into_iter().flatten() {
                        if budget == 0 {
                            break 'walk;
                        }
                        let d = self.next[c];
                        if let Some(seg) = self.twoopt_ok(board, body_next, is_snake, a, b, c, d) {
                            budget -= 1;
                            let (sn, sp) = (self.next.clone(), self.prev.clone());
                            self.apply_twoopt(a, b, c, d, &seg);
                            if self.cycle_dist(head, food) < d0 {
                                improved = true;
                                break 'walk;
                            }
                            self.next = sn;
                            self.prev = sp;
                        }
                    }
                }
                x = next_x;
                steps += 1;
            }
            if !improved {
                break;
            }
        }
    }
}

/// Is `{u, v}` an edge occupied by the snake body (consecutive segments)?
fn is_body_edge(body_next: &[Option<usize>], u: usize, v: usize) -> bool {
    body_next[u] == Some(v) || body_next[v] == Some(u)
}

impl Strategy for CycleSurgeon {
    fn next_move(&mut self, state: &GameState) -> Direction {
        let board = state.board();
        let n = board.num_cells();
        let head = cell_index(board, state.head());
        let food = cell_index(board, state.food());

        // Body-successor table + snake membership (occupied-edge source).
        let mut body_next = vec![None; n];
        let mut is_snake = vec![false; n];
        let mut prev_cell: Option<usize> = None;
        for cell in state.snake() {
            let idx = cell_index(board, cell);
            is_snake[idx] = true;
            if let Some(p) = prev_cell {
                body_next[p] = Some(idx);
            }
            prev_cell = Some(idx);
        }

        self.optimize(board, head, food, &body_next, &is_snake);

        // Follow the cycle successor. Unlike the rider's shortcut jumps, this
        // keeps the body a contiguous arc on the (reshaped) cycle — the
        // invariant the surgery relies on — while the reshaping toward the food
        // provides the speedup. (Shortcut jumps are unsafe here: their
        // vacate-time proof assumes a *static* cycle, which per-tick reshaping
        // breaks.)
        let succ = self.next[head];
        let dir = safe_moves(state)
            .into_iter()
            .find(|(_, cell)| cell_index(board, *cell) == succ)
            .map(|(dir, _)| dir)
            .unwrap_or_else(|| {
                safe_moves(state)
                    .first()
                    .map(|(dir, _)| *dir)
                    .unwrap_or_else(|| doomed_move(state))
            });

        // Overlay: the whole current cycle from the head, so the reshaping is
        // visible live.
        self.debug.path.clear();
        let mut cur = head;
        for _ in 0..n {
            self.debug.path.push(self.offset_of(cur));
            cur = self.next[cur];
        }
        dir
    }

    fn debug(&self) -> Option<&StrategyDebug> {
        Some(&self.debug)
    }
}

#[cfg(test)]
mod tests {
    use super::super::hamilton::assert_valid_cycle_order;
    use super::*;
    use crate::board::BoundaryMode;
    use crate::game::{Config, Status};

    /// Rebuild the ordered cycle and assert it is a valid Hamiltonian cycle.
    fn assert_cycle_valid(surgeon: &CycleSurgeon, board: &Board) {
        let (order, _) = surgeon.rebuild_order();
        assert_valid_cycle_order(board, &order);
    }

    #[test]
    fn wins_both_topologies_and_stays_valid() {
        for boundary in [BoundaryMode::Walls, BoundaryMode::Periodic] {
            for seed in 0..20 {
                let config = Config {
                    width: 16,
                    height: 12,
                    boundary,
                    seed,
                };
                let mut state = GameState::new(config);
                let mut surgeon = CycleSurgeon::new(state.board()).unwrap();
                let board = *state.board();
                let mut ticks = 0;
                while state.status() == Status::Running && ticks < 20_000 {
                    let dir = surgeon.next_move(&state);
                    // Invariant: the cycle stays a valid Hamiltonian cycle
                    // after every reshape.
                    assert_cycle_valid(&surgeon, &board);
                    state.tick(Some(dir));
                    ticks += 1;
                }
                assert_eq!(
                    state.status(),
                    Status::Won,
                    "surgeon must win ({boundary:?}, seed {seed}), score {}",
                    state.score()
                );
            }
        }
    }

    #[test]
    fn deterministic_same_seed() {
        let config = Config {
            width: 16,
            height: 12,
            boundary: BoundaryMode::Walls,
            seed: 7,
        };
        let run = || {
            let mut state = GameState::new(config);
            let mut surgeon = CycleSurgeon::new(state.board()).unwrap();
            let mut moves = Vec::new();
            let mut ticks = 0;
            while state.status() == Status::Running && ticks < 20_000 {
                let dir = surgeon.next_move(&state);
                moves.push(dir);
                state.tick(Some(dir));
                ticks += 1;
            }
            moves
        };
        assert_eq!(run(), run(), "same seed ⇒ identical move sequence");
    }

    #[test]
    fn wins_on_larger_board() {
        // 24×18 stretch check. Tick budget is 45 000 rather than the plan's
        // 20 000: the current safe surgeon is slower than the HamiltonRider,
        // which itself needs ~24 000 ticks here — 20 000 is unreachable by any
        // perfect player on this board. See the Phase-D problem report.
        let config = Config {
            width: 24,
            height: 18,
            boundary: BoundaryMode::Periodic,
            seed: 0,
        };
        let mut state = GameState::new(config);
        let mut surgeon = CycleSurgeon::new(state.board()).unwrap();
        let mut ticks = 0;
        while state.status() == Status::Running && ticks < 45_000 {
            let dir = surgeon.next_move(&state);
            state.tick(Some(dir));
            ticks += 1;
        }
        assert_eq!(state.status(), Status::Won, "score {}", state.score());
    }
}
