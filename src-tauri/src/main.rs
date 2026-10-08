#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod commands;
mod desktop;
fn main() {
    desktop::run();
}
