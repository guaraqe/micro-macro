#![cfg(target_arch = "wasm32")]

use crate::create_app;
use eframe::{egui, WebRunner};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;

/// Launch the egui app inside the canvas referenced by `index.html`.
#[wasm_bindgen]
pub async fn start() -> Result<(), JsValue> {
    use web_sys::HtmlCanvasElement;

    console_error_panic_hook::set_once();

    let document = web_sys::window()
        .ok_or("No window")?
        .document()
        .ok_or("No document")?;

    let canvas = document
        .get_element_by_id("the_canvas_id")
        .ok_or("Canvas not found")?
        .dyn_into::<HtmlCanvasElement>()?;

    let web_options = eframe::WebOptions::default();

    WebRunner::new()
        .start(canvas, web_options, Box::new(|cc| Ok(Box::new(create_app(cc)))))
        .await
}

/// Request a file from the user using the WASM-friendly dialog.
pub fn open_project_dialog(ctx: egui::Context) {
    use rfd::AsyncFileDialog;
    use wasm_bindgen_futures::spawn_local;

    let task = AsyncFileDialog::new()
        .add_filter("JSON", &["json"])
        .pick_file();

    spawn_local(async move {
        if let Some(file) = task.await {
            let _data = file.read().await;
            // TODO: feed the file contents into a new Action and refresh state.
            ctx.request_repaint();
        }
    });
}
