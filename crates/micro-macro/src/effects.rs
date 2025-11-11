use crate::store::Store;
use std::path::PathBuf;

/// Deferred effects that must run outside the main reducer (e.g., file IO)
#[derive(Debug, Clone)]
pub enum Effect {
    /// Save current project to disk
    SaveToFile { path: PathBuf },
    /// Load a project from disk
    LoadFromFile { path: PathBuf },
}

/// Execute a single effect against the store
pub fn run(store: &mut Store, effect: Effect) {
    match effect {
        Effect::SaveToFile { path } => {
            if let Err(e) = store.save_to_file(&path) {
                store.error_message = Some(e);
            }
        }
        Effect::LoadFromFile { path } => {
            if let Err(e) = store.load_from_file(&path) {
                store.error_message = Some(e);
            }
        }
    }
}
