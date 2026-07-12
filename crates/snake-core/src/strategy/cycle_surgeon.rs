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
//! At the end of every tick `next`/`prev` is a single Hamiltonian cycle and the
//! snake body is a **contiguous directed subpath** of it (tail…neck→head); no
//! repair ever touches a body-occupied edge. The snake then follows the cycle
//! **successor strictly (offset 1)** — the move is always `next[head]`, which
//! sits at cycle position 1 and is a body cell only when the board is full
//! (`Won`). So it can never collide and the board always fills. (Shortcut
//! jumps — as the rider takes — are *unsafe* here: their vacate-time proof
//! assumes a *static* cycle, which per-tick reshaping breaks, killing the
//! snake. See the plan's Phase-D problem report.) The property tests are the
//! arbiter.
//!
//! Because the body is contiguous, its cells occupy a fixed **interval** of
//! cycle positions (`{0} ∪ [n−len+1, n−1]`, with the head at 0), so
//! "is this cell free / behind the food?" is O(1) integer arithmetic on the
//! per-tick position labels.
//!
//! # Data structure & operations
//!
//! Cell index = `row * width + col` (row-major, offset coordinates). The cycle
//! is a doubly linked ring (`next`/`prev`). Reshaping uses a single
//! direction-preserving primitive (no segment reversal, so body direction is
//! never at risk):
//!
//! - **cross-swap** `(a→b),(u→v) ⇒ (a→v),(u→b)` (needs `a` adj `v`, `u` adj
//!   `b`). On one cycle with `b…u` a segment it **splits** off the ring
//!   `[b…u]`; across two cycles it **merges** them. It is its own inverse
//!   (O(1) rollback, no clones).
//! - **excise-and-transplant** (the workhorse): split a free chunk out of the
//!   head→food arc — shortening that distance by the chunk length — then
//!   cross-swap the detached ring back in at an edge **behind the food**, so
//!   the shortening sticks. The split site always exists because the N/S
//!   column rungs are present for every cell in odd-q layout; no reversal and
//!   no length cap are needed. Determinism: fixed scan orders
//!   (`Direction::ALL`, ascending cycle position), no RNG, no map iteration.

use super::hamilton::serpentine_cycle;
use super::{cell_index, doomed_move, safe_moves, HamiltonRider, Strategy, StrategyDebug};
use crate::board::Board;
use crate::coords::{Direction, Offset};
use crate::game::GameState;

/// Max excise-and-transplant compounds applied per tick.
const MAX_COMPOUND: usize = 48;
/// Split candidates tried (best-first) before giving up on a compound.
const SPLIT_TRIES: usize = 8;

/// Hamiltonian-cycle follower that repairs the cycle toward the food each tick.
pub struct CycleSurgeon {
    width: usize,
    /// Cell index → next cell in cycle order.
    next: Vec<usize>,
    /// Cell index → previous cell in cycle order.
    prev: Vec<usize>,
    /// Per-tick scratch: cell index → position in cycle order from the head.
    pos: Vec<u32>,
    /// Per-tick scratch: cycle position → cell index.
    cell_by_pos: Vec<usize>,
    /// Generation-stamped marker for "cell is in the detached ring".
    ring_mark: Vec<u32>,
    ring_gen: u32,
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
            pos: vec![0; n],
            cell_by_pos: vec![0; n],
            ring_mark: vec![0; n],
            ring_gen: 0,
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

    /// Label cycle positions (from the head) into `pos`/`cell_by_pos`.
    fn label(&mut self, head: usize) {
        let n = self.next.len();
        let mut cur = head;
        for i in 0..n {
            self.pos[cur] = i as u32;
            self.cell_by_pos[i] = cur;
            cur = self.next[cur];
        }
    }

    /// Is the snake body a contiguous directed subpath of the current cycle?
    /// (`next[s_{i+1}] == s_i` for every consecutive body pair.) The surgery's
    /// interval arithmetic and its winning proof both require this; only the
    /// first couple of ticks after spawn fail it.
    fn body_contiguous(&self, state: &GameState, board: &Board) -> bool {
        let mut prev: Option<usize> = None;
        for cell in state.snake() {
            let idx = cell_index(board, cell);
            if let Some(p) = prev {
                if self.next[idx] != p {
                    return false;
                }
            }
            prev = Some(idx);
        }
        true
    }

    /// The cross-swap primitive: with `b = next[a]`, `v = next[u]`, replace
    /// edges `(a→b)`, `(u→v)` by `(a→v)`, `(u→b)`. Preconditions (`a` adj `v`,
    /// `u` adj `b`) are the caller's. Its own inverse: calling it again with
    /// the same `(a, u)` restores the original edges.
    fn cross_swap(&mut self, a: usize, u: usize) {
        let b = self.next[a];
        let v = self.next[u];
        self.next[a] = v;
        self.prev[v] = a;
        self.next[u] = b;
        self.prev[b] = u;
    }

    /// Mark the ring reachable from `b` along `next` (after a split, that is
    /// exactly the detached segment `[b…u]`).
    fn mark_ring(&mut self, b: usize) {
        self.ring_gen += 1;
        let gen = self.ring_gen;
        let mut cur = b;
        loop {
            self.ring_mark[cur] = gen;
            cur = self.next[cur];
            if cur == b {
                break;
            }
        }
    }

    /// After a split whose ring starts at `b`, find a merge site that reattaches
    /// the ring at an edge behind the food: a main edge `(g→k)` with `g` free
    /// and at/behind the food, and a ring edge `(e→f)` with `g` adj `f`,
    /// `e` adj `k`. Returns the cross-swap tails `(g, e)`. Positions are the
    /// pre-split labels, still order-valid for the untouched cells.
    fn find_merge(
        &self,
        board: &Board,
        b: usize,
        dfood: usize,
        last_free: usize,
    ) -> Option<(usize, usize)> {
        let gen = self.ring_gen;
        let mut e = b;
        loop {
            let f = self.next[e];
            for kn in self.neighbors(board, e) {
                let Some(k) = kn else { continue };
                if self.ring_mark[k] == gen {
                    continue; // k must be in the main cycle
                }
                let g = self.prev[k];
                if self.ring_mark[g] == gen {
                    continue; // g must be in the main cycle
                }
                let pg = self.pos[g] as usize;
                // g must be free (≤ last free position) and at/behind the food.
                if pg < dfood || pg > last_free {
                    continue;
                }
                if self.adjacent(board, g, f) {
                    return Some((g, e));
                }
            }
            e = self.next[e];
            if e == b {
                break;
            }
        }
        None
    }

    /// Reshape the cycle to shorten the head→food distance, one
    /// excise-and-transplant compound at a time. Each applied compound reduces
    /// the distance by the excised chunk length; safety is independent of how
    /// many fire (fewer ⇒ slower, never unsafe).
    fn reshape(&mut self, board: &Board, head: usize, food: usize, len: usize) {
        let n = self.next.len();
        let last_free = n - len; // cycle positions 1..=last_free are free
        for _ in 0..MAX_COMPOUND {
            self.label(head);
            let dfood = self.pos[food] as usize;
            if dfood <= 1 {
                return;
            }
            // Split candidates: chunk [b…u] strictly inside the head→food arc.
            // Tuple: (chunk length, pb, dir index, a-tail, u-tail, ring start b).
            let mut cands: Vec<(usize, usize, usize, usize, usize, usize)> = Vec::new();
            for pb in 1..dfood {
                let b = self.cell_by_pos[pb];
                let a = self.cell_by_pos[pb - 1];
                for (dir, un) in self.neighbors(board, b).into_iter().enumerate() {
                    let Some(u) = un else { continue };
                    let pu = self.pos[u] as usize;
                    if pu < pb || pu >= dfood {
                        continue; // segment must be forward and before the food
                    }
                    let v = self.cell_by_pos[pu + 1];
                    if self.adjacent(board, a, v) {
                        cands.push((pu - pb + 1, pb, dir, a, u, b));
                    }
                }
            }
            // Longest chunk first (biggest shortening), then deterministic ties.
            cands.sort_by(|x, y| y.0.cmp(&x.0).then(x.1.cmp(&y.1)).then(x.2.cmp(&y.2)));

            let mut applied = false;
            for &(_m, _pb, _dir, a, u, b) in cands.iter().take(SPLIT_TRIES) {
                self.cross_swap(a, u); // split: ring = [b…u]
                self.mark_ring(b);
                if let Some((g, e)) = self.find_merge(board, b, dfood, last_free) {
                    self.cross_swap(g, e); // merge behind the food
                    applied = true;
                    break;
                }
                self.cross_swap(a, u); // rollback (self-inverse)
            }
            if !applied {
                return;
            }
        }
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
}

impl Strategy for CycleSurgeon {
    fn next_move(&mut self, state: &GameState) -> Direction {
        let board = state.board();
        let n = board.num_cells();
        let head = cell_index(board, state.head());
        let food = cell_index(board, state.food());

        // Reshape only once the body lies contiguously on the cycle (the
        // interval arithmetic and the winning proof require it); the first
        // couple of ticks after spawn just follow the seed cycle.
        if self.body_contiguous(state, board) {
            self.reshape(board, head, food, state.snake_len());
        }

        // Follow the cycle successor strictly (offset 1) — keeps the body a
        // contiguous arc. The safe_moves filter is belt-and-suspenders: once
        // synced, next[head] is provably a safe, non-neck move.
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
        let config = Config {
            width: 24,
            height: 18,
            boundary: BoundaryMode::Periodic,
            seed: 3,
        };
        let mut state = GameState::new(config);
        let mut surgeon = CycleSurgeon::new(state.board()).unwrap();
        let mut ticks = 0;
        while state.status() == Status::Running && ticks < 20_000 {
            let dir = surgeon.next_move(&state);
            state.tick(Some(dir));
            ticks += 1;
        }
        assert_eq!(state.status(), Status::Won, "score {}", state.score());
    }
}
