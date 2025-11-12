use crate::graph_view::{
    ObservableGraphDisplay, StateGraphDisplay, setup_graph_display,
};
use crate::serialization;
use crate::store::Store;
use std::fs;
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
            let state = serializable_state_from_graphs(
                store.state_graph.get(),
                store.observable_graph.get(),
            );
            if let Err(e) = serialization::save_to_file(&state, &path)
            {
                store.error_message = Some(e);
            }
        }
        Effect::LoadFromFile { path } => {
            let result = (|| -> Result<(), String> {
                let raw = fs::read_to_string(&path).map_err(|e| {
                    format!("Failed to read file: {e}")
                })?;
                let state: serialization::SerializableState =
                    serde_json::from_str(&raw).map_err(|e| {
                        format!("Failed to parse JSON: {e}")
                    })?;
                let state_graph_raw =
                    serialization::serializable_to_graph(
                        &state.dynamical_system,
                    );
                let observable_graph_raw =
                    serialization::serializable_to_observable_graph(
                        &state.observable,
                        &state_graph_raw,
                    );
                store
                    .state_graph
                    .set(setup_graph_display(&state_graph_raw));
                store
                    .observable_graph
                    .set(setup_graph_display(&observable_graph_raw));

                store.recompute_observed_graph();
                // Layout resets now automatic via version tracking
                Ok(())
            })();
            if let Err(e) = result {
                store.error_message = Some(e);
            }
        }
    }
}

fn serializable_state_from_graphs(
    state_graph: &StateGraphDisplay,
    observable_graph: &ObservableGraphDisplay,
) -> serialization::SerializableState {
    serialization::SerializableState {
        dynamical_system: serialization::graph_to_serializable(
            state_graph,
        ),
        observable: serialization::observable_graph_to_serializable(
            observable_graph,
        ),
    }
}
