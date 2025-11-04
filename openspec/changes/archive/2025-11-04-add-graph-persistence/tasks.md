# Tasks: Add Graph Persistence

## Implementation Tasks

### 1. Add serialization dependencies
- [x] Add `serde_json` to Cargo.toml dependencies
- [x] Add `rfd` (Rust File Dialog) for native file picker support
- [x] Add both using `cargo add`, do not change deps file manually
- [x] Verify dependencies compile

### 2. Define serializable state structures
- [x] Create `SerializableGraphState` struct to hold dynamical system graph data (nodes with names, edges)
- [x] Create `SerializableObservableState` struct for observable graph data (nodes with names and types, edges)
- [x] Derive `Serialize` and `Deserialize` for both structs
- [x] Implement conversion methods: from `Graph<NodeData>` to `SerializableGraphState`
- [x] Implement conversion methods: from `Graph<MappingNodeData>` to `SerializableObservableState`

### 3. Implement default state function
- [x] Create `fn default_graph_state() -> Graph<NodeData>` that builds the initial dynamical system graph
- [x] Create `fn default_observable_state() -> Graph<MappingNodeData>` that builds the initial observable graph
- [x] Ensure these functions are explicit and reproducible
- **Dependency**: Must be done before startup loading

### 4. Implement state.json auto-load on startup
- [x] Modify `GraphEditor::default()` or equivalent initialization
- [x] Check if `state.json` exists in working directory using `std::fs`
- [x] If exists: read file, deserialize JSON, convert to graph structures, initialize with loaded state
- [x] If not exists: call default state functions
- [x] Handle errors: if `state.json` is corrupted, log error and fall back to default state
- **Dependency**: Requires task 2 (serializable structures) and task 3 (default state function)

### 5. Implement save functionality
- [x] Add method `fn save_to_file(&self, path: &Path) -> Result<(), String>` to save current graph
- [x] Serialize active tab's graph to JSON
- [x] Write JSON to specified file path
- [x] Handle file I/O errors and return descriptive error messages
- **Dependency**: Requires task 2 (serializable structures)

### 6. Implement load functionality
- [x] Add method `fn load_from_file(&mut self, path: &Path) -> Result<(), String>` to load graph from file
- [x] Read file contents
- [x] Deserialize JSON based on active tab (dynamical system or observable)
- [x] Replace current graph with loaded graph
- [x] Set layout reset flag to redraw graph
- [x] Handle file read and parse errors with descriptive messages
- **Dependency**: Requires task 2 (serializable structures)

### 7. Add file menu UI
- [x] Add menu bar to top panel using `egui::MenuBar`
- [x] Add "File" menu with "Save" and "Load" options
- [x] Connect "Save" option to file picker dialog using `rfd::FileDialog::new().add_filter("JSON", &["json"]).save_file()`
- [x] Connect "Load" option to file picker dialog using `rfd::FileDialog::new().add_filter("JSON", &["json"]).pick_file()`
- [x] Handle user cancellation (None returned from dialog)
- **Dependency**: Requires tasks 5 and 6 (save/load functionality)

### 8. Wire up save button action
- [x] When "Save" is clicked and file is chosen, call `save_to_file()` with chosen path
- [x] Display success message or error dialog using `egui::Window` for errors
- [x] Ensure correct graph (dynamical system or observable) is saved based on active tab
- **Dependency**: Requires task 7 (file menu UI)

### 9. Wire up load button action
- [x] When "Load" is clicked and file is chosen, call `load_from_file()` with chosen path
- [x] Display error dialog if load fails
- [x] Update UI to reflect loaded graph state
- **Dependency**: Requires task 7 (file menu UI)

### 10. Test and validate
- [x] Test auto-load: Start app, verify default state, save to `state.json`, restart app, verify state loads
- [x] Test manual save: Save graph to custom filename, verify file contents
- [x] Test manual load: Load previously saved file, verify graph matches
- [x] Test error handling: Try loading invalid JSON, verify error message and stable state
- [x] Test both tabs: Verify save/load works for both dynamical system and observable graphs
- [x] Run `cargo build` and ensure it passes
- **Dependency**: Requires all previous tasks

## Notes
- Tasks 2, 3 can be done in parallel
- Task 4 depends on tasks 2 and 3
- Tasks 5, 6 can be done in parallel after task 2
- Task 7 depends on tasks 5 and 6
- Tasks 8, 9 can be done in parallel after task 7
- Task 10 is the final validation step
