// src/tui/mod.rs
//! TUI module for displaying messages and handling user input

pub mod app;
pub mod event;
pub mod logger;
pub mod ui;

pub use app::TuiApp;
pub use event::run_tui;
pub use logger::TuiLogger;
