//! Rubik's Cube DNA observatory: the simulation half.
//!
//! `engine` is carried over unchanged from the softbuffer port (cube state,
//! quarter-turn algebra, timeline, net mapping, lighting model). `sim` wraps it
//! with the DNA panel's transient state and produces one snapshot per frame for
//! QML; `mesh` builds the rounded cubie and sticker geometry for Qt Quick 3D.
//! Nothing here depends on Qt, so all of it is unit-testable with plain
//! `cargo test`.

pub mod color;
pub mod engine;
pub mod mesh;
pub mod sim;

use std::io::Write;

/// Diagnostic line to stdout that never panics on a closed handle.
pub fn log_line(msg: &str) {
    let _ = writeln!(std::io::stdout(), "{msg}");
}

/// [`log_line`] to stderr.
pub fn log_line_err(msg: &str) {
    let _ = writeln!(std::io::stderr(), "{msg}");
}
