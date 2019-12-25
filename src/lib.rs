//! Rust implementation of https://wyag.thb.lt/ tutorial

#[macro_use]
extern crate log;

/// Wrappers that translate CLI commands into the underlying library.
pub mod commands;
/// Functions and types for dealing with repositories.
pub mod repository;
