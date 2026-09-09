// Prevents an additional console window on Windows in release. Menlo is macOS-only
// (§1.5), but the attribute is harmless and keeps the scaffold's contract intact.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    menlo_lib::run()
}
