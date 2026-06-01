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
    /// Whether the help overlay is visible
    pub show_help: bool,
    /// Whether Vim-style visual selection is active
    pub visual_mode: bool,
    /// Frozen message snapshot used while visual mode is active
    frozen_messages: Vec<String>,
    /// Frozen log snapshot used while visual mode is active
    frozen_logs: Vec<String>,
    /// Rendered wrapped lines for the active pane
    rendered_lines: Vec<String>,
    /// First visible rendered line for the active pane
    rendered_start_line: usize,
    /// Visible height for the active pane
    rendered_visible_height: usize,
    /// Selection anchor in rendered line coordinates
    visual_anchor: usize,
    /// Current cursor in rendered line coordinates
    visual_cursor: usize,
    /// Current line cursor in normal pane navigation
    pane_cursor: usize,
    /// Whether the pane cursor has been initialized
    pane_cursor_initialized: bool,
    /// History of submitted input messages (oldest first)
    input_history: Vec<String>,
    /// Position when browsing input history; None means editing the live draft
    history_index: Option<usize>,
    /// Draft saved when we start browsing input history
    history_draft: String,
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
            show_help: false,
            visual_mode: false,
            frozen_messages: Vec::new(),
            frozen_logs: Vec::new(),
            rendered_lines: Vec::new(),
            rendered_start_line: 0,
            rendered_visible_height: 0,
            visual_anchor: 0,
            visual_cursor: 0,
            pane_cursor: 0,
            pane_cursor_initialized: false,
            input_history: Vec::new(),
            history_index: None,
            history_draft: String::new(),
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
            while messages.len() > MAX_MESSAGES {
                messages.pop_front();
            }
        }
    }

    /// Get messages for display (returns a copy of the buffer)
    pub fn get_messages(&self) -> Vec<String> {
        if self.visual_mode {
            return self.frozen_messages.clone();
        }

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
        if self.scroll_offset > 0 {
            self.auto_scroll = false;
        }
    }

    /// Scroll down (decrease offset)
    pub fn scroll_down(&mut self, amount: usize) {
        self.scroll_offset = self.scroll_offset.saturating_sub(amount);
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
        let byte_pos = self.byte_index();
        self.input.insert(byte_pos, c);
        self.cursor_position += 1;
    }

    /// Delete character before cursor
    pub fn delete_char(&mut self) {
        if self.cursor_position > 0 {
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

    fn byte_index(&self) -> usize {
        self.input
            .char_indices()
            .nth(self.cursor_position)
            .map(|(idx, _)| idx)
            .unwrap_or(self.input.len())
    }

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
        if !input.is_empty() && self.input_history.last() != Some(&input) {
            self.input_history.push(input.clone());
        }
        self.history_index = None;
        self.history_draft.clear();
        input
    }

    /// Recall the previous (older) entry from input history into the input box
    pub fn history_prev(&mut self) {
        if self.input_history.is_empty() {
            return;
        }

        let new_index = match self.history_index {
            None => {
                self.history_draft = self.input.clone();
                self.input_history.len() - 1
            }
            Some(0) => 0,
            Some(i) => i - 1,
        };

        self.history_index = Some(new_index);
        self.input = self.input_history[new_index].clone();
        self.cursor_position = self.input.chars().count();
    }

    /// Recall the next (newer) entry from input history, or restore the draft
    pub fn history_next(&mut self) {
        let Some(index) = self.history_index else {
            return;
        };

        if index + 1 < self.input_history.len() {
            let new_index = index + 1;
            self.history_index = Some(new_index);
            self.input = self.input_history[new_index].clone();
        } else {
            self.history_index = None;
            self.input = std::mem::take(&mut self.history_draft);
        }
        self.cursor_position = self.input.chars().count();
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
        self.show_help = false;
        if self.show_logs {
            self.log_scroll_offset = 0;
            self.log_auto_scroll = true;
        }
    }

    /// Toggle help overlay visibility
    pub fn toggle_help(&mut self) {
        self.show_help = !self.show_help;
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
        if self.visual_mode {
            return self.frozen_logs.clone();
        }

        if let Ok(logs) = self.log_buffer.lock() {
            logs.iter().cloned().collect()
        } else {
            Vec::new()
        }
    }

    /// Store the current wrapped-line model for the active pane
    pub fn set_rendered_lines(
        &mut self,
        lines: Vec<String>,
        start_line: usize,
        visible_height: usize,
    ) -> usize {
        let old_total_lines = self.rendered_lines.len();
        let old_start_line = self.rendered_start_line;
        let old_visible_height = self.rendered_visible_height.max(1);
        let old_last_visible = old_start_line
            .saturating_add(old_visible_height.saturating_sub(1))
            .min(old_total_lines.saturating_sub(1));
        let was_following_bottom = self.pane_cursor_initialized
            && old_total_lines > 0
            && self.pane_cursor >= old_last_visible
            && self.active_auto_scroll();

        self.rendered_lines = lines;
        self.rendered_start_line = start_line;
        self.rendered_visible_height = visible_height.max(1);

        if self.visual_mode {
            let max_index = self.rendered_lines.len().saturating_sub(1);
            self.visual_anchor = self.visual_anchor.min(max_index);
            self.visual_cursor = self.visual_cursor.min(max_index);
            self.sync_visual_view();
        } else if self.rendered_lines.is_empty() {
            self.pane_cursor = 0;
            self.pane_cursor_initialized = false;
        } else {
            let max_index = self.rendered_lines.len() - 1;
            if !self.pane_cursor_initialized || was_following_bottom {
                self.pane_cursor = self.initial_visible_cursor();
                self.pane_cursor_initialized = true;
            } else {
                self.pane_cursor = self.pane_cursor.min(max_index);
                self.sync_pane_view();
            }
        }

        self.rendered_start_line
    }

    /// Enter Vim-style visual selection mode
    pub fn enter_visual_mode(&mut self) {
        if self.visual_mode || self.rendered_lines.is_empty() {
            return;
        }

        self.frozen_messages = self
            .message_buffer
            .lock()
            .map(|messages| messages.iter().cloned().collect())
            .unwrap_or_default();
        self.frozen_logs = self
            .log_buffer
            .lock()
            .map(|logs| logs.iter().cloned().collect())
            .unwrap_or_default();
        self.show_help = false;
        self.visual_mode = true;
        self.visual_cursor = self.pane_cursor;
        self.visual_anchor = self.visual_cursor;
        self.sync_visual_view();
    }

    /// Exit visual selection mode and resume live updates
    pub fn exit_visual_mode(&mut self) {
        self.visual_mode = false;
        self.frozen_messages.clear();
        self.frozen_logs.clear();
    }

    /// Toggle visual selection mode
    pub fn toggle_visual_mode(&mut self) {
        if self.visual_mode {
            self.exit_visual_mode();
        } else {
            self.enter_visual_mode();
        }
    }

    /// Move the normal pane cursor up
    pub fn pane_up(&mut self, amount: usize) {
        if self.visual_mode || self.rendered_lines.is_empty() {
            return;
        }

        self.pane_cursor = self.pane_cursor.saturating_sub(amount);
        self.sync_pane_view();
    }

    /// Move the normal pane cursor down
    pub fn pane_down(&mut self, amount: usize) {
        if self.visual_mode || self.rendered_lines.is_empty() {
            return;
        }

        let max_index = self.rendered_lines.len().saturating_sub(1);
        self.pane_cursor = (self.pane_cursor + amount).min(max_index);
        self.sync_pane_view();
    }

    /// Jump the normal pane cursor to the first line
    pub fn pane_top(&mut self) {
        if self.visual_mode || self.rendered_lines.is_empty() {
            return;
        }

        self.pane_cursor = 0;
        self.sync_pane_view();
    }

    /// Jump the normal pane cursor to the last line
    pub fn pane_bottom(&mut self) {
        if self.visual_mode || self.rendered_lines.is_empty() {
            return;
        }

        self.pane_cursor = self.rendered_lines.len() - 1;
        self.sync_pane_view();
    }

    /// Move the visual cursor up
    pub fn visual_up(&mut self, amount: usize) {
        if !self.visual_mode {
            return;
        }

        self.visual_cursor = self.visual_cursor.saturating_sub(amount);
        self.sync_visual_view();
    }

    /// Move the visual cursor down
    pub fn visual_down(&mut self, amount: usize) {
        if !self.visual_mode {
            return;
        }

        let max_index = self.rendered_lines.len().saturating_sub(1);
        self.visual_cursor = (self.visual_cursor + amount).min(max_index);
        self.sync_visual_view();
    }

    /// Jump the visual cursor to the first line
    pub fn visual_top(&mut self) {
        if !self.visual_mode || self.rendered_lines.is_empty() {
            return;
        }

        self.visual_cursor = 0;
        self.sync_visual_view();
    }

    /// Jump the visual cursor to the last line
    pub fn visual_bottom(&mut self) {
        if !self.visual_mode || self.rendered_lines.is_empty() {
            return;
        }

        self.visual_cursor = self.rendered_lines.len() - 1;
        self.sync_visual_view();
    }

    /// Get the selected rendered-line range
    pub fn visual_range(&self) -> Option<(usize, usize)> {
        if !self.visual_mode || self.rendered_lines.is_empty() {
            return None;
        }

        Some((
            self.visual_anchor.min(self.visual_cursor),
            self.visual_anchor.max(self.visual_cursor),
        ))
    }

    /// Get the current visual cursor position
    pub fn visual_cursor(&self) -> Option<usize> {
        if self.visual_mode && !self.rendered_lines.is_empty() {
            Some(self.visual_cursor)
        } else {
            None
        }
    }

    /// Get the current pane cursor position
    pub fn pane_cursor(&self) -> Option<usize> {
        if !self.visual_mode && self.pane_cursor_initialized && !self.rendered_lines.is_empty() {
            Some(self.pane_cursor)
        } else {
            None
        }
    }

    /// Return the selected text from the current visual range
    pub fn selected_text(&self) -> Option<String> {
        let (start, end) = self.visual_range()?;
        Some(self.rendered_lines[start..=end].join("\n"))
    }

    fn initial_visible_cursor(&self) -> usize {
        if self.rendered_lines.is_empty() {
            return 0;
        }

        let last_visible = self
            .rendered_start_line
            .saturating_add(self.rendered_visible_height.saturating_sub(1));
        last_visible.min(self.rendered_lines.len() - 1)
    }

    fn sync_pane_view(&mut self) {
        if self.visual_mode || self.rendered_lines.is_empty() {
            return;
        }

        let total_lines = self.rendered_lines.len();
        let visible_height = self.rendered_visible_height.max(1).min(total_lines);
        let max_start = total_lines.saturating_sub(visible_height);
        let mut start_line = self.rendered_start_line.min(max_start);

        if self.pane_cursor < start_line {
            start_line = self.pane_cursor;
        } else if self.pane_cursor >= start_line + visible_height {
            start_line = self.pane_cursor + 1 - visible_height;
        }

        self.rendered_start_line = start_line;

        let scroll_offset = total_lines.saturating_sub(visible_height + start_line);
        if self.show_logs {
            self.log_scroll_offset = scroll_offset;
            self.log_auto_scroll = scroll_offset == 0;
        } else {
            self.scroll_offset = scroll_offset;
            self.auto_scroll = scroll_offset == 0;
        }
    }

    fn active_auto_scroll(&self) -> bool {
        if self.show_logs {
            self.log_auto_scroll
        } else {
            self.auto_scroll
        }
    }

    fn sync_visual_view(&mut self) {
        if !self.visual_mode || self.rendered_lines.is_empty() {
            return;
        }

        let total_lines = self.rendered_lines.len();
        let visible_height = self.rendered_visible_height.max(1).min(total_lines);
        let max_start = total_lines.saturating_sub(visible_height);
        let mut start_line = self.rendered_start_line.min(max_start);

        if self.visual_cursor < start_line {
            start_line = self.visual_cursor;
        } else if self.visual_cursor >= start_line + visible_height {
            start_line = self.visual_cursor + 1 - visible_height;
        }

        self.rendered_start_line = start_line;

        let scroll_offset = total_lines.saturating_sub(visible_height + start_line);
        if self.show_logs {
            self.log_scroll_offset = scroll_offset;
            self.log_auto_scroll = scroll_offset == 0;
        } else {
            self.scroll_offset = scroll_offset;
            self.auto_scroll = scroll_offset == 0;
        }
    }
}
