//! Core engine module for processing and evaluation.

pub mod eval;

use crate::models::Value;

/// Errors that can occur in the engine
#[derive(Debug)]
pub enum EngineError {
    /// Division by zero
    DivisionByZero,
    /// Unknown variable
    UnknownVariable(String),
    /// Stack overflow
    StackOverflow,
}

impl std::fmt::Display for EngineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EngineError::DivisionByZero => write!(f, "Division by zero"),
            EngineError::UnknownVariable(name) => write!(f, "Unknown variable: {}", name),
            EngineError::StackOverflow => write!(f, "Stack overflow"),
        }
    }
}

/// Process a list of values
pub fn process(values: &[Value]) -> Result<Value, EngineError> {
    if values.is_empty() {
        return Ok(Value::Null);
    }
    // Just return the first value for now
    Ok(values[0].clone())
}

/// Configuration for the engine
pub struct EngineConfig {
    pub max_depth: usize,
    pub trace: bool,
}

impl Default for EngineConfig {
    fn default() -> Self {
        EngineConfig {
            max_depth: 100,
            trace: false,
        }
    }
}
