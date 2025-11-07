# Tasks: Add Observed Graph Module

## Implementation Order

Tasks are ordered to maintain working code at each step, with verification checkpoints.

### Phase 1: Graph State Module Refactoring

- [x] **Create graph_state module file**
  - Create `src/graph_state.rs`
  - Add `mod graph_state;` to `src/main.rs`
  - Verify: `cargo build` succeeds

- [x] **Move StateGraph and StateNode to graph_state**
  - Copy `StateGraph`, `StateNode`, `HasName` trait to `graph_state.rs`
  - Copy `default_state_graph()` function
  - Export types: `pub type StateGraph`, `pub struct StateNode`, etc.
  - Update `src/graph.rs` to re-export from `graph_state` (maintain backward compatibility temporarily)
  - Verify: `cargo build` succeeds, existing functionality unchanged

- [x] **Move ObservableGraph and ObservableNode to graph_state**
  - Copy `ObservableGraph`, `ObservableNode`, `ObservableNodeType` to `graph_state.rs`
  - Copy `default_observable_graph()` function
  - Export types from module
  - Update `src/graph.rs` to re-export from `graph_state`
  - Verify: `cargo build` succeeds, Observable Editor tab works

- [x] **Update imports across codebase**
  - Change imports in `main.rs` to use `graph_state::{...}`
  - Change imports in `graph_view.rs` to use `graph_state::{...}`
  - Change imports in `serialization.rs` to use `graph_state::{...}`
  - Remove re-exports from `src/graph.rs` (or deprecate old module)
  - Verify: `cargo build` succeeds, all tabs functional

### Phase 2: Observed Graph Types

- [x] **Define ObservedNode struct**
  - Add `ObservedNode` struct to `graph_state.rs`:
    ```rust
    pub struct ObservedNode {
        pub name: String,
        pub observable_node_idx: NodeIndex,
    }
    ```
  - Implement `Clone` for `ObservedNode`
  - Implement `HasName` trait for `ObservedNode`
  - Verify: `cargo build` succeeds

- [x]**Define ObservedGraph type alias**
  - Add `pub type ObservedGraph = StableGraph<ObservedNode, f32>;` to `graph_state.rs`
  - Verify: `cargo build` succeeds

- [x]**Implement calculate_observed_graph function**
  - Add function signature: `pub fn calculate_observed_graph(state_graph: &StateGraph, observable_graph: &ObservableGraph) -> ObservedGraph`
  - Iterate over `ObservableGraph` nodes, filter by `ObservableNodeType::Destination`
  - For each Destination node, create `ObservedNode` with name and index
  - Add nodes to new `ObservedGraph` (no edges yet)
  - Add TODO comment: "TODO: Implement edge computation logic"
  - Return the graph
  - Verify: `cargo build` succeeds

- [x]**Add observed graph to GraphEditor state**
  - Add field `observed_graph: ObservedGraph` to `GraphEditor` struct in `main.rs`
  - Initialize in `load_or_create_default_state()` by calling `calculate_observed_graph(&g, &mg)`
  - Initialize in `load_graphs_from_path()` similarly
  - Verify: `cargo build` succeeds, app starts and loads correctly

### Phase 3: Graph View Integration

- [x]**Create ObservedGraphDisplay type alias**
  - Add type alias in `graph_view.rs`:
    ```rust
    pub type ObservedGraphDisplay = Graph<
        ObservedNode,
        f32,
        Directed,
        DefaultIx,
        DefaultNodeShape,
        WeightedEdgeShape,
    >;
    ```
  - Verify: `cargo build` succeeds

- [x]**Create ObservedGraphView type alias**
  - Add type alias in `graph_view.rs`:
    ```rust
    pub type ObservedGraphView<'a> = GraphView<
        'a,
        ObservedNode,
        f32,
        Directed,
        DefaultIx,
        DefaultNodeShape,
        WeightedEdgeShape,
        LayoutStateCircular,
        LayoutCircular,
    >;
    ```
  - Verify: `cargo build` succeeds

- [x]**Add observed_graph display field to GraphEditor**
  - Change `observed_graph: ObservedGraph` to `observed_graph: ObservedGraphDisplay`
  - Update initialization to use `setup_graph_display(&observed_graph)`
  - Verify: `cargo build` succeeds

### Phase 4: Observed Dynamics Tab UI

- [x]**Add ObservedDynamics to ActiveTab enum**
  - Add `ObservedDynamics` variant to `ActiveTab` enum in `main.rs`
  - Add tab button in tab bar: `"Observed Dynamics"`
  - Add match arm in `update()` for `ActiveTab::ObservedDynamics`
  - Create empty `render_observed_dynamics_tab(&mut self, ctx: &egui::Context)` method (stub with placeholder text)
  - Verify: `cargo build` succeeds, third tab appears and switches correctly

- [x]**Implement read-only left panel for observed nodes**
  - Copy structure from `render_observable_editor_tab` left panel
  - Change heading to "Observed Values"
  - Remove "Add Value" button
  - Display node names as labels (not text_edit_singleline)
  - Remove delete buttons
  - Keep collapsible arrow for connection display
  - Show "Values: N" count in footer
  - Verify: Panel displays observed nodes correctly

- [x]**Implement read-only center panel for graph visualization**
  - Copy structure from `render_dynamical_system_tab` center panel
  - Change heading to "Observed Graph"
  - Use `ObservedGraphView::new(&mut self.observed_graph)`
  - Apply `LayoutStateCircular` layout
  - Remove all editing interaction (no Ctrl mode switching, no edge creation/deletion)
  - Keep "Show Labels" checkbox
  - Remove mode hints from footer
  - Add layout reset flag `observed_layout_reset_needed` to `GraphEditor`
  - Verify: Graph visualizes correctly, no editing possible

- [x]**Implement read-only right panel for heatmap**
  - Copy structure from `render_dynamical_system_tab` right panel
  - Change heading to "Observed Dynamics Heatmap"
  - Call `build_observed_heatmap_data()` (implement helper method similar to `build_heatmap_data()`)
  - Display heatmap with hover highlighting but no editing
  - Use separate hover state or reuse existing (since only one tab active at a time)
  - Show "Edges: N" count in footer
  - Verify: Heatmap displays correctly, no editing possible

### Phase 5: Automatic Recomputation

- [x]**Trigger recomputation on state graph changes**
  - In `render_dynamical_system_tab`, after operations that modify state graph (add/remove/rename nodes, add/remove edges)
  - Call `self.observed_graph = setup_graph_display(&calculate_observed_graph(self.state_graph.g(), self.observable_graph.g()))`
  - Set `self.observed_layout_reset_needed = true`
  - Verify: Changes in state graph reflect in observed graph

- [x]**Trigger recomputation on observable graph changes**
  - In `render_observable_editor_tab`, after operations that modify observable graph (add/remove/rename Destination nodes, add/remove edges)
  - Call same recomputation as above
  - Set `self.observed_layout_reset_needed = true`
  - Verify: Changes in observable graph reflect in observed graph immediately

- [x]**Ensure sync_source_nodes triggers recomputation**
  - After `sync_source_nodes()` completes (which modifies observable graph)
  - Trigger observed graph recomputation
  - Verify: Adding nodes in dynamical system updates observed graph via observable sync

### Phase 6: Testing and Validation

- [x]**Manual testing: Basic observed graph display**
  - Start app, switch to Observed Dynamics tab
  - Verify: Default 2 observed nodes displayed (matching 2 default Destination nodes)
  - Verify: Nodes shown in all three panels
  - Verify: No editing controls visible

- [x]**Manual testing: Node synchronization**
  - Add a Destination node in Observable Editor
  - Switch to Observed Dynamics tab
  - Verify: New node appears in observed graph
  - Rename a Destination node
  - Verify: Name updates in observed graph
  - Delete a Destination node
  - Verify: Node removed from observed graph

- [x]**Manual testing: Read-only enforcement**
  - In Observed Dynamics tab, attempt to edit node name
  - Verify: No text field appears
  - Hold Ctrl and try to create edge
  - Verify: No edge creation possible
  - Click heatmap cell
  - Verify: No edit field appears

- [x]**Build and run verification**
  - Run `cargo build`
  - Verify: No errors or warnings
  - Run `cargo run`
  - Verify: Application starts without crashes
  - Verify: All three tabs functional
  - Verify: Save/Load works for state and observable graphs
