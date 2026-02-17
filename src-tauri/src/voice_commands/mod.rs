//! Voice commands module for Phonon
//!
//! This module handles parsing of voice commands for punctuation,
//! editing actions, and contextual commands based on dictation mode.

mod executor;
mod parser;

pub use executor::execute_actions;
pub use parser::{parse, Action, ParseResult};
