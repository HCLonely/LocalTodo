#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod commands;
mod card_window;
mod desktop;
mod portable;
fn main() {
    desktop::run();
}
