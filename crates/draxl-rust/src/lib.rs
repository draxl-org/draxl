#![forbid(unsafe_code)]
//! Rust-profile support for Draxl.
//!
//! The crate currently exposes lowering utilities for validated Draxl
//! Rust-profile input, and keeps that functionality namespaced so other
//! Rust-profile helpers can be added alongside it over time.

pub mod lower;

pub use lower::lower_file;
