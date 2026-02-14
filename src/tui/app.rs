// src/tui/app.rs
//! TUI application state management

use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

/// Maximum number of messages to keep in buffer
const MAX_MESSAGES: usize = 1000;

/// TUI Application state
pub struct TuiApp {
    /// Shared message buffer (thread-safe)
    pub message_buffer: Arc<Mutex<VecDeque<String>>>,
    /// Current scroll offset (0 = bottom, 1 = one line up, etc.)
    pub scroll_offset: usize,
    /// Whether auto-scroll is enabled
    pub auto_scroll: bool,
    /// Current input text
    pub input: String,
    /// Cursor position in input
    pub cursor_position: usize,
    /// Room ID being monitored
    pub room_id: String,
    /// Whether to quit the application
    pub should_quit: bool,
    /// Shared online user count (thread-safe, updated from event handler)
    pub online_count: Arc<AtomicU64>,
    /// Whether to show raw event messages
    pub show_raw: bool,
    /// Shared log buffer for capturing log messages (thread-safe)
    pub log_buffer: Arc<Mutex<VecDeque<String>>>,
    /// Whether to show the logs panel
    pub show_logs: bool,
    /// Scroll offset for logs panel (0 = bottom)
    pub log_scroll_offset: usize,
    /// Whether auto-scroll is enabled for logs panel
    pub log_auto_scroll: bool,
}

impl TuiApp {
    /// Create a new TUI application with shared message buffer
    pub fn new(message_buffer: Arc<Mutex<VecDeque<String>>>, room_id: String) -> Self {
        Self::with_online_count(message_buffer, room_id, Arc::new(AtomicU64::new(0)))
    }

    /// Create a new TUI application with shared message buffer and online count
    pub fn with_online_count(
        message_buffer: Arc<Mutex<VecDeque<String>>>,
        room_id: String,
        online_count: Arc<AtomicU64>,
    ) -> Self {
        Self {
            message_buffer,
            scroll_offset: 0,
            auto_scroll: true,
            input: String::new(),
            cursor_position: 0,
            room_id,
            should_quit: false,
            online_count,
            show_raw: false,
            log_buffer: Arc::new(Mutex::new(VecDeque::new())),
            show_logs: false,
            log_scroll_offset: 0,
            log_auto_scroll: true,
        }
    }

    /// Get the current online count
    pub fn get_online_count(&self) -> u64 {
        self.online_count.load(Ordering::Relaxed)
    }

    /// Update the online count (called from event handler)
    pub fn set_online_count(online_count: &Arc<AtomicU64>, count: u64) {
        online_count.store(count, Ordering::Relaxed);
    }

    /// Add a message to the buffer (called from event handler)
    pub fn add_message(buffer: &Arc<Mutex<VecDeque<String>>>, message: String) {
        if let Ok(mut messages) = buffer.lock() {
            messages.push_back(message);
            // Keep buffer bounded
            while messages.len() > MAX_MESSAGES {
                messages.pop_front();
            }
        }
    }

    /// Get messages for display (returns a copy of the buffer)
    pub fn get_messages(&self) -> Vec<String> {
        if let Ok(messages) = self.message_buffer.lock() {
            messages.iter().cloned().collect()
        } else {
            Vec::new()
        }
    }

    /// Get the number of messages in buffer
    pub fn message_count(&self) -> usize {
        if let Ok(messages) = self.message_buffer.lock() {
            messages.len()
        } else {
            0
        }
    }

    /// Scroll up (increase offset)
    pub fn scroll_up(&mut self, amount: usize) {
        let max_offset = self.message_count().saturating_sub(1);
        self.scroll_offset = (self.scroll_offset + amount).min(max_offset);
        // Disable auto-scroll when user scrolls up
        if self.scroll_offset > 0 {
            self.auto_scroll = false;
        }
    }

    /// Scroll down (decrease offset)
    pub fn scroll_down(&mut self, amount: usize) {
        self.scroll_offset = self.scroll_offset.saturating_sub(amount);
        // Re-enable auto-scroll when scrolled to bottom
        if self.scroll_offset == 0 {
            self.auto_scroll = true;
        }
    }

    /// Scroll to bottom
    pub fn scroll_to_bottom(&mut self) {
        self.scroll_offset = 0;
        self.auto_scroll = true;
    }

    /// Handle character input
    pub fn enter_char(&mut self, c: char) {
        // Get byte position from character position
        let byte_pos = self.byte_index();
        self.input.insert(byte_pos, c);
        self.cursor_position += 1;
    }

    /// Delete character before cursor
    pub fn delete_char(&mut self) {
        if self.cursor_position > 0 {
            // Get the byte position of the character before cursor
            let byte_pos = self.byte_index_at(self.cursor_position - 1);
            self.input.remove(byte_pos);
            self.cursor_position -= 1;
        }
    }

    /// Move cursor left
    pub fn move_cursor_left(&mut self) {
        if self.cursor_position > 0 {
            self.cursor_position -= 1;
        }
    }

    /// Move cursor right
    pub fn move_cursor_right(&mut self) {
        let char_count = self.input.chars().count();
        if self.cursor_position < char_count {
            self.cursor_position += 1;
        }
    }

    /// Get byte index for current cursor position (character-based)
    fn byte_index(&self) -> usize {
        self.input
            .char_indices()
            .nth(self.cursor_position)
            .map(|(idx, _)| idx)
            .unwrap_or(self.input.len())
    }

    /// Get byte index for a specific character position
    fn byte_index_at(&self, char_pos: usize) -> usize {
        self.input
            .char_indices()
            .nth(char_pos)
            .map(|(idx, _)| idx)
            .unwrap_or(self.input.len())
    }

    /// Get current input and clear it
    pub fn take_input(&mut self) -> String {
        let input = self.input.clone();
        self.input.clear();
        self.cursor_position = 0;
        input
    }

    /// Quit the application
    pub fn quit(&mut self) {
        self.should_quit = true;
    }

    /// Toggle raw message visibility
    pub fn toggle_show_raw(&mut self) {
        self.show_raw = !self.show_raw;
    }

    /// Toggle logs panel visibility
    pub fn toggle_show_logs(&mut self) {
        self.show_logs = !self.show_logs;
        // Reset log scroll when toggling
        if self.show_logs {
            self.log_scroll_offset = 0;
            self.log_auto_scroll = true;
        }
    }

    /// Get the number of log messages in buffer
    pub fn log_message_count(&self) -> usize {
        if let Ok(logs) = self.log_buffer.lock() {
            logs.len()
        } else {
            0
        }
    }

    /// Scroll logs up (increase offset)
    pub fn log_scroll_up(&mut self, amount: usize) {
        let max_offset = self.log_message_count().saturating_sub(1);
        self.log_scroll_offset = (self.log_scroll_offset + amount).min(max_offset);
        if self.log_scroll_offset > 0 {
            self.log_auto_scroll = false;
        }
    }

    /// Scroll logs down (decrease offset)
    pub fn log_scroll_down(&mut self, amount: usize) {
        self.log_scroll_offset = self.log_scroll_offset.saturating_sub(amount);
        if self.log_scroll_offset == 0 {
            self.log_auto_scroll = true;
        }
    }

    /// Scroll logs to bottom
    pub fn log_scroll_to_bottom(&mut self) {
        self.log_scroll_offset = 0;
        self.log_auto_scroll = true;
    }

    /// Set the log buffer (used to share with the TuiLogger)
    pub fn set_log_buffer(&mut self, log_buffer: Arc<Mutex<VecDeque<String>>>) {
        self.log_buffer = log_buffer;
    }

    /// Get log messages for display
    pub fn get_log_messages(&self) -> Vec<String> {
        if let Ok(logs) = self.log_buffer.lock() {
            logs.iter().cloned().collect()
        } else {
            Vec::new()
        }
    }
}
