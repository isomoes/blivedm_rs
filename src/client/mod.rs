// src/client/lib.rs
//! Library entry for the client package

pub mod auth;
#[cfg(feature = "browser_cookies")]
pub mod browser_cookies;
pub mod models;
pub mod scheduler;
pub mod websocket;

// Re-export commonly used functions
pub use auth::get_cookies_or_browser;
