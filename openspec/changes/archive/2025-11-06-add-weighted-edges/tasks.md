# Tasks: Add Weighted Edges

## Implementation Tasks

### 1. Update core data structures
- [x] Change `MyGraphView` edge type from `()` to `f32`
- [x] Change `MappingGraphView` edge type from `()` to `f32`
- [x] Update `SerializableEdge` to include `weight: f32` field
- [x] Update `setup_graph()` signature to use `StableGraph<NodeData, f32>`
- [x] Update `setup_mapping_graph()` signature to use `StableGraph<MappingNodeData, f32>`
- [x] Update `generate_graph()` to return `StableGraph<NodeData, f32>` and use weight 1.0
- [x] Update `generate_mapping_graph()` to return `StableGraph<MappingNodeData, f32>`
- [x] Update `GraphEditor` struct to use `Graph<NodeData, f32>` and `Graph<MappingNodeData, f32>`
- [x] Verify types compile without errors

### 2. Modify serialization functions
- [x] Update `graph_to_serializable()` to extract and save edge weights
- [x] Update `serializable_to_graph()` to load edge weights from serialized data
- [x] Update `mapping_graph_to_serializable()` to extract and save edge weights
- [x] Update `serializable_to_mapping_graph()` to load edge weights from serialized data
- [x] Fixed edge weight extraction using `.payload()` method from egui_graphs Edge wrapper
- [x] Verify save/load preserves weights correctly
- **Dependency**: Requires task 1 (data structures)

### 3. Add editing state fields to GraphEditor
- [x] Add `heatmap_editing_cell: Option<(usize, usize)>` field to struct
- [x] Add `heatmap_edit_buffer: String` field to struct
- [x] Initialize both fields in `main()` constructor (None and empty string)
- [x] Verify state persists across frames
- **Dependency**: Requires task 1 (data structures)

### 4. Update build_heatmap_data functions
- [x] Change `build_heatmap_data()` return type to `(Vec<String>, Vec<String>, Vec<Vec<Option<f32>>>)`
- [x] Extract weight from edges using `.payload()` method instead of returning bool
- [x] Return `None` for cells with no edge, `Some(weight)` for cells with edges
- [x] Change `build_mapping_heatmap_data()` return type to `(Vec<String>, Vec<String>, Vec<Vec<Option<f32>>>)`
- [x] Apply same weight extraction logic to mapping graph
- [x] Verify functions return weight data correctly
- **Dependency**: Requires task 1 (data structures)

### 5. Enhance heatmap display
- [x] Change `show_heatmap()` signature to accept `matrix: &[Vec<Option<f32>>]`
- [x] Update cell rendering to display weight value when `Some(weight)`
- [x] Format weight with 1 decimal place using `format!("{:.1}", weight)`
- [x] Render weight text centered in cell
- [x] Show empty cell appearance for `None` values
- [x] Adjust cell coloring to distinguish weighted vs empty cells
- [x] Verify weights display correctly in heatmap
- **Dependency**: Requires task 4 (build_heatmap_data functions)

### 6. Implement inline editing in heatmap
- [x] Add `editing_cell` and `edit_buffer` parameters to `show_heatmap()` function
- [x] Detect left-click on cell to enter edit mode
- [x] When entering edit mode: store cell coordinates, populate buffer with current weight (or empty)
- [x] Render `TextEdit::singleline()` widget in editing cell
- [x] Handle Enter key: parse buffer to f32, return weight change event, clear editing state
- [x] Handle Tab key: parse buffer, return weight change and move-to-next event, update editing cell to next position (left-to-right, wrap to next row)
- [x] Handle Escape key: clear editing state without changes
- [x] Detect click outside editing cell to cancel edit
- [x] Created EditingState struct and WeightChange struct for clean API
- [x] Return tuple with editing state and optional weight change
- [x] Fixed keyboard event handling to work correctly in egui
- [x] Improved TextEdit styling with centered layout
- [x] Verify editing works with proper keyboard navigation
- **Dependency**: Requires tasks 3, 4, and 5

### 7. Wire up heatmap editing to graph updates
- [x] In `render_dynamical_system_tab()`, receive weight change events from `show_heatmap()`
- [x] When weight change occurs: map (x, y) to (source_node, target_node) indices
- [x] If weight is 0.0: remove edge using `self.g.remove_edge()`
- [x] If weight is non-zero and edge exists: update edge weight using `edge.payload_mut()`
- [x] If weight is non-zero and edge doesn't exist: add edge using `self.g.add_edge(source, target, weight)`
- [x] Apply same logic in `render_observable_editor_tab()` for mapping graph
- [x] Created `apply_weight_change_to_graph()` and `apply_weight_change_to_mapping_graph()` helper functions
- [x] Verify graph updates when weights are edited
- **Dependency**: Requires task 6 (inline editing)

### 8. Update edge operations for drag-to-create
- [x] Find `handle_edge_creation()` for dynamical system graph
- [x] Update `add_edge()` call to pass weight 1.0 instead of `()`
- [x] Find `handle_mapping_edge_creation()` for observable graph
- [x] Update `add_edge()` call to pass weight 1.0
- [x] Verify edges created via drag have default weight 1.0
- **Dependency**: Requires task 1 (data structures)

### 9. Implement edge thickness visualization
- [x] Investigate `egui_graphs` edge rendering API for custom thickness
- [x] Created custom WeightedEdgeShape that automatically calculates width from weight
- [x] Implement linear scaling: `thickness = 1.0 + weight.min(4.0)` clamped to 1.0-5.0px
- [x] Determine max_weight dynamically or use fixed scale (e.g., weight 5.0 = 5.0px)
- [x] Apply thickness to both dynamical system and observable editor graphs
- [x] Width automatically updates in WeightedEdgeShape::from() and update() methods
- [x] Fixed edge_stroke_hook to preserve calculated widths
- [x] Verify edge thickness scales proportionally with weight
- **Dependency**: Can be done in parallel with tasks 4-8

### 10. Integration and testing
- [x] Run `cargo build` and fix any compilation errors
- [x] Test: Create edge via drag, verify weight 1.0 in heatmap
- [x] Test: Click cell, type "2.5", press Enter, verify edge updates
- [x] Test: Click cell, type "3.0", press Tab, verify moves to next cell
- [x] Test: Type "0.0", press Enter, verify edge disappears
- [x] Test: Edge thickness visually reflects weight differences
- [x] Test: Save graph, load graph, verify weights preserved
- [x] Test: Both dynamical system and observable editor tabs work correctly
- [x] Verify all functionality works end-to-end
- **Dependency**: Requires all previous tasks

## Notes
- Tasks 1, 2, 3, 4, 8 can be done in parallel initially
- Task 5 depends on task 4
- Task 6 depends on tasks 3, 4, 5
- Task 7 depends on task 6
- Task 9 can be done independently in parallel
- Task 10 depends on all previous tasks
