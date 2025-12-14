#![warn(clippy::all, rust_2018_idioms)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use baobab::run_baobab;

fn main() -> Result<(), String> {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).
    run_baobab().map_err(|e| e.to_string())
}
