//! Core game logic for HexSnake.
//!
//! This crate is free of UI dependencies, fully deterministic (seedable RNG,
//! no wall clock, no hash-map iteration order) and compiles to
//! `wasm32-unknown-unknown`.

mod board;
mod coords;
mod game;

pub use board::{Board, BoundaryMode};
pub use coords::{Axial, Direction, Offset};
pub use game::{Config, GameState, Status};
