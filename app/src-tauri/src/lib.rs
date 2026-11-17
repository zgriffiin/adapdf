//! The Adapdf desktop app (milestone M5a): a Tauri shell with an IBM Carbon page
//! (`app/src`) over the `adapdf` library.
//!
//! - [`commands`]: what the page asks of the library, as plain functions. Each
//!   calls the same library function as the matching `adapdf` command (row A-07).
//! - [`signoff`]: saves the human checklist and the sign-off into the report
//!   (row A-08).
//! - `shell`: the Tauri commands, which only call the two modules above off the
//!   main thread, and the start of the window.
//!
//! The tests of the two modules need no window and no Node: run them with
//! `cargo test -p adapdf-app --lib`. The shell (`shell.rs`) reads the built page
//! (`app/dist`) when it is compiled, so it is left out of the test build.

pub mod commands;
pub mod signoff;

#[cfg(not(test))]
mod shell;
#[cfg(not(test))]
pub use shell::run;

#[cfg(test)]
mod tests;
