// src/tui/logger.rs
//! Custom logger that captures log messages into a shared buffer for TUI display

use log::{Log, Metadata, Record};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::Instant;

/// Maximum number of log messages to keep in buffer
const MAX_LOG_MESSAGES: usize = 1000;

/// A logger that writes log messages to a shared buffer for TUI display.
/// It also optionally forwards to env_logger for file/stderr output.
pub struct TuiLogger {
    buffer: Arc<Mutex<VecDeque<String>>>,
    level: log::LevelFilter,
    start_time: Instant,
}

impl TuiLogger {
    /// Create a new TuiLogger with the given shared buffer and level filter.
    pub fn new(buffer: Arc<Mutex<VecDeque<String>>>, level: log::LevelFilter) -> Self {
        Self {
            buffer,
            level,
            start_time: Instant::now(),
        }
    }

    /// Initialize this logger as the global logger.
    /// Returns the shared buffer so it can be passed to TuiApp.
    pub fn init(level: log::LevelFilter) -> Arc<Mutex<VecDeque<String>>> {
        let buffer = Arc::new(Mutex::new(VecDeque::new()));
        let logger = TuiLogger::new(Arc::clone(&buffer), level);
        log::set_boxed_logger(Box::new(logger)).expect("Failed to set TuiLogger");
        log::set_max_level(level);
        buffer
    }
}

impl Log for TuiLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= self.level
    }

    fn log(&self, record: &Record) {
        if !self.enabled(record.metadata()) {
            return;
        }

        let elapsed = self.start_time.elapsed();
        let secs = elapsed.as_secs();
        let mins = secs / 60;
        let hours = mins / 60;
        let timestamp = format!("{:02}:{:02}:{:02}", hours, mins % 60, secs % 60);
        let msg = format!(
            "[{}] [{}] [{}] {}",
            timestamp,
            record.level(),
            record.target(),
            record.args()
        );

        if let Ok(mut buf) = self.buffer.lock() {
            buf.push_back(msg);
            while buf.len() > MAX_LOG_MESSAGES {
                buf.pop_front();
            }
        }
    }

    fn flush(&self) {}
}
