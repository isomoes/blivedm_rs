// src/lib.rs
//! Bilibili live room danmaku WebSocket client library with TTS and plugin support

pub mod client;
pub mod plugins;
pub mod tui;

// Re-export commonly used items from client
pub use client::{auth, get_cookies_or_browser, models, scheduler, websocket};
#[cfg(feature = "browser_cookies")]
pub use client::browser_cookies;

// Re-export plugin modules and helpers
pub use plugins::{
    auto_reply, auto_reply_handler, terminal_display, terminal_display_handler, tts, tts_handler,
    tts_handler_command, tts_handler_default,
};
