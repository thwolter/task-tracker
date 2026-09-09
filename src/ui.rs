//! Slint UI composition and callback wiring.
//!
//! This module connects the application tracker to generated Slint components;
//! controller details remain private to the UI boundary.

mod controller;
#[cfg(test)]
mod tests;

/// Initializes projections and gives the UI command handler ownership of the tracker.
pub(crate) use controller::bind;
