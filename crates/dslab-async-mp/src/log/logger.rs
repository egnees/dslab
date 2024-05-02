//! Logging.

use std::{
    fs::{File, OpenOptions},
    io::Write,
    path::Path,
};

use super::log_entry::LogEntry;

#[derive(Default)]
/// Implements logging of events to console and optionally to a file.
/// Also provides the access to the list of all logged events (trace).  
pub struct Logger {
    log_file: Option<File>,
    trace: Vec<LogEntry>,
}

impl Logger {
    /// Creates a new console-only logger.
    pub(crate) fn new() -> Self {
        Self {
            log_file: None,
            trace: vec![],
        }
    }

    /// Creates a new logger writing events both to console and the specified file.
    pub(crate) fn with_log_file(log_path: &Path) -> Self {
        let log_file = Some(
            OpenOptions::new()
                .create(true)
                .truncate(true)
                .write(true)
                .open(log_path)
                .unwrap(),
        );
        Self {
            log_file,
            trace: vec![],
        }
    }

    pub(crate) fn has_log_file(&self) -> bool {
        self.log_file.is_some()
    }

    pub(crate) fn log(&mut self, event: LogEntry) {
        if let Some(log_file) = self.log_file.as_mut() {
            let serialized = serde_json::to_string(&event).unwrap();
            log_file.write_all(serialized.as_bytes()).unwrap();
            log_file.write_all("\n".as_bytes()).unwrap();
        }

        self.trace.push(event.clone());

        event.print();
    }

    /// Returns a reference to a vector with all logged events.
    pub fn trace(&self) -> &Vec<LogEntry> {
        &self.trace
    }
}
