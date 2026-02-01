//! Sample crate for testing the codebase index generator.
//!
//! This crate demonstrates various Rust constructs that the indexer
//! should be able to parse and catalog.

pub mod engine;
pub mod models;
mod utils;

use std::collections::HashMap;

/// Application configuration
pub struct Config {
    pub name: String,
    pub debug: bool,
    port: u16,
}

/// Possible errors in the application
#[derive(Debug)]
pub enum AppError {
    /// An I/O error occurred
    Io(std::io::Error),
    /// A configuration error
    Config(String),
    /// An engine error
    Engine(engine::EngineError),
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppError::Io(e) => write!(f, "IO error: {}", e),
            AppError::Config(s) => write!(f, "Config error: {}", s),
            AppError::Engine(e) => write!(f, "Engine error: {}", e),
        }
    }
}

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        AppError::Io(e)
    }
}

impl From<engine::EngineError> for AppError {
    fn from(e: engine::EngineError) -> Self {
        AppError::Engine(e)
    }
}

/// Initialize the application with default config
pub fn init() -> Config {
    Config {
        name: "sample".to_string(),
        debug: false,
        port: 8080,
    }
}

/// Run the application with the given config
pub fn run(config: &Config) -> Result<(), AppError> {
    if config.debug {
        eprintln!("Running in debug mode");
    }
    Ok(())
}

/// A type alias for results
pub type AppResult<T> = Result<T, AppError>;

/// A constant for the default port
pub const DEFAULT_PORT: u16 = 8080;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init() {
        let config = init();
        assert_eq!(config.name, "sample");
    }
}
