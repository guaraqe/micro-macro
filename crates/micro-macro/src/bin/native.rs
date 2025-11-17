#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result<()> {
    micro_macro::native::run()
}

#[cfg(target_arch = "wasm32")]
fn main() {
    // This binary is not meant to be used for WASM.
    // Use the library's start() function instead.
}
