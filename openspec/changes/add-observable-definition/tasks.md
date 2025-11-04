## 1. Dependencies and Core Types

- [x] 1.1 Add `egui_tabs` dependency to `Cargo.toml`
- [x] 1.2 Create `NodeType` enum (Source, Destination) for mapping graph
- [x] 1.3 Create new `MappingNodeData` struct with `name: String` and `node_type: NodeType`
- [x] 1.4 Add second `Graph` field to global state for mapping graph (using `MappingNodeData`)
- [x] 1.5 Initialize mapping graph on startup with default source nodes and two default destination nodes

## 2. Tab Navigation with egui_tabs

- [x] 2.1 Integrate native egui tab navigation into main UI structure
- [x] 2.2 Create two tabs: "Dynamical System" and "Observable Editor"
- [x] 2.3 Implement tab switching logic to show/hide appropriate views
- [x] 2.4 Set "Dynamical System" as default active tab on startup

## 3. Source Node Synchronization

- [x] 3.1 Implement function to sync mapping graph Source nodes from dynamical system nodes
- [x] 3.2 Call sync on dynamical system node creation (add new Source node to mapping graph)
- [x] 3.3 Call sync on dynamical system node deletion (remove Source node and its edges from mapping graph)
- [x] 3.4 Call sync on dynamical system node rename (update Source node name in mapping graph)
- [x] 3.5 Ensure synchronization happens regardless of active tab

## 4. Bipartite Layout Module

- [x] 4.1 Create `layout_bipartite.rs` module
- [x] 4.2 Implement `LayoutBipartite` struct with two-column positioning logic
- [x] 4.3 Implement `LayoutStateBipartite` for layout state management
- [x] 4.4 Add node type detection to position Source nodes in left column, Destination nodes in right column
- [x] 4.5 Implement alphabetical sorting for Source nodes in left column
- [x] 4.6 Implement vertical spacing for both columns

## 5. Observable Editor UI Structure

- [x] 5.1 Create three-panel layout for Observable Editor (left/center/right)
- [x] 5.2 Preserve state in both tabs when switching
- [x] 5.3 Implement layout reset for bipartite graph when needed

## 6. Destination Node Management Panel (Left)

- [x] 6.1 Implement left panel with "Add Value" button
- [x] 6.2 Display list of Destination nodes with editable names
- [x] 6.3 Add delete button for each Destination node
- [x] 6.4 Implement collapsible/selection behavior for Destination nodes
- [x] 6.5 When Destination selected, display incoming Source nodes that map to it
- [x] 6.6 Show count of incoming edges for each Destination

## 7. Bipartite Graph Visualization (Center)

- [x] 7.1 Configure GraphView for mapping graph with bipartite layout
- [x] 7.2 Display Source nodes in left column (vertically, alphabetically sorted)
- [x] 7.3 Display Destination nodes in right column (vertically)
- [x] 7.4 Set appropriate node sizes and visual styling
- [x] 7.5 Implement Node Editor mode for selecting nodes
- [x] 7.6 Implement Edge Editor mode (Ctrl key) for creating/deleting edges

## 8. Edge Creation Constraints

- [x] 8.1 Implement node type checking in edge creation logic
- [x] 8.2 Allow edge creation only when dragging from Source (left) to Destination (right)
- [x] 8.3 Prevent Source-to-Source edges
- [x] 8.4 Prevent Destination-to-Destination edges
- [x] 8.5 Prevent Destination-to-Source edges (reverse direction)
- [x] 8.6 Clear edge labels after creation (consistent with existing behavior)

## 9. Observable Mapping Heatmap (Right)

- [x] 9.1 Adapt heatmap module for mapping graph structure
- [x] 9.2 Build matrix with Sources as columns (x-axis) and Destinations as rows (y-axis)
- [x] 9.3 Display heatmap in right panel of Observable Editor
- [x] 9.4 Implement hover highlighting to show Source-Destination connections
- [x] 9.5 Handle empty state when no Destinations exist or no edges exist
- [x] 9.6 Display edge count metadata at bottom of right panel

## 10. Integration and Testing

- [x] 10.1 Test node synchronization: add/delete/rename nodes in Dynamical System tab
- [x] 10.2 Verify synchronization works when Observable Editor tab is not visible
- [x] 10.3 Test edge creation and deletion in Observable Editor
- [x] 10.4 Verify edge constraints prevent invalid connections
- [x] 10.5 Test tab switching preserves state correctly
- [x] 10.6 Verify heatmap updates correctly when edges are added/removed
- [x] 10.7 Add visual hints/instructions for Observable Editor usage (mode hints displayed)
- [x] 10.8 Ensure consistent styling between both tabs
