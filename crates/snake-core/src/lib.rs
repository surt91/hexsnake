//! Core game logic for HexSnake.
//!
//! This crate is free of UI dependencies, fully deterministic (seedable RNG,
//! no wall clock, no hash-map iteration order) and compiles to
//! `wasm32-unknown-unknown`.

pub mod benchmark;
mod board;
mod coords;
mod game;
pub mod strategy;

pub use board::{Board, BoundaryMode};
pub use coords::{Axial, Direction, Offset};
pub use game::{Config, GameState, Status};
pub use strategy::Strategy;
