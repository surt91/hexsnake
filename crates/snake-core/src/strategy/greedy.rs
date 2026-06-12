use super::{doomed_move, safe_moves, Strategy};
use crate::coords::Direction;
use crate::game::GameState;

/// Picks the safe move that minimizes the (torus-aware) hex distance to the
/// food. Fast and simple, but happily walls itself in.
pub struct Greedy;

impl Strategy for Greedy {
    fn next_move(&mut self, state: &GameState) -> Direction {
        let board = state.board();
        let food = state.food();
        safe_moves(state)
            .into_iter()
            // min_by_key is stable on ties: first direction in ALL order wins.
            .min_by_key(|(_, n)| board.distance(*n, food))
            .map(|(dir, _)| dir)
            .unwrap_or_else(|| doomed_move(state))
    }
}
