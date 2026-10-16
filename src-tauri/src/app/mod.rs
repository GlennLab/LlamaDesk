//! Application layer: shared state, background monitor loops, and Tauri commands.
//!
//! This layer wires the pure `domain` types to the `infra` adapters through the
//! object-safe `ports`. It owns no domain logic of its own beyond orchestrating
//! polling and exposing the results to the frontend.

pub mod commands;
pub mod monitor;
pub mod state;
#[cfg(desktop)]
pub mod tray;

pub use state::AppState;
