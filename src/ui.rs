//! Slint UI composition and callback wiring.
//!
//! This module connects the application tracker to generated Slint components;
//! controller details remain private to the UI boundary.

mod bindings;
mod controller;
mod tests;

/// Wires callbacks, initializes projections, and starts the one-second UI timer.
pub(crate) use bindings::bind;
