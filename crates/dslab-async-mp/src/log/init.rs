//! Utility which is needed to initialize logging.

/// Enables printing logs to the console.
pub fn enable_console_log() {
    env_logger::Builder::new().filter_level(log::LevelFilter::Debug).init();
}
