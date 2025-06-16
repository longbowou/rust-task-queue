//! Task modules for the actix integration example
//!
//! This module demonstrates how to organize tasks in separate files
//! while still enabling auto-registration functionality.

pub mod communication;
pub mod processing;

// Re-export all tasks for convenience
pub use communication::*;
pub use processing::*;
