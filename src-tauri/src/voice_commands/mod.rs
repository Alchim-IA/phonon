//! Voice commands module for WakaScribe
//!
//! This module handles parsing of voice commands for punctuation,
//! editing actions, and contextual commands based on dictation mode.

mod parser;

pub use parser::{parse, Action, ParseResult};
