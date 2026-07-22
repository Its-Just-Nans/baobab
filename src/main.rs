#![warn(clippy::all, rust_2018_idioms)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use baobab::BaobabApp;
use bladvak::app::{Bladvak, MainResult};

fn main() -> MainResult {
    Bladvak::<BaobabApp>::bladvak_main()
}
