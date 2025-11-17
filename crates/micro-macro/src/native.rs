#![cfg(not(target_arch = "wasm32"))]

use crate::create_app;

/// Entry point used by the native executable.
pub fn run() -> eframe::Result<()> {
    let native_options = eframe::NativeOptions::default();

    eframe::run_native(
        "Graph Editor",
        native_options,
        Box::new(|cc| Ok(Box::new(create_app(cc)))),
    )
}
