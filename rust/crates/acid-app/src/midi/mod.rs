//! MIDI input for the app: port choice, message parsing, and a connection that never blocks the audio
//! thread.
//!
//! Nothing here may panic or block when no interface is attached — that is the common case, and the app has
//! to open and play without one.

pub mod ports;
pub mod message;
pub mod tempo;
pub mod cc;
pub mod supervisor;
