//! egui/eframe frontend for HexSnake. Runs natively and in the browser
//! (`wasm32-unknown-unknown` via trunk).

mod app;
mod game_view;
mod hex_layout;
mod highscores;
mod settings;

pub use app::App;
