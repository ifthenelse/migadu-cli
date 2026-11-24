//! migadu: a command line for Migadu email hosting.
//!
//! `bin` (`main.rs`) wires real HTTP and process spawning into [`cli::run`]; tests wire fakes.

pub mod api;
pub mod cli;
pub mod html;
pub mod http;
pub mod secrets;
pub mod web;
