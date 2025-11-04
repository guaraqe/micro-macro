## Why

Macroscopic dynamics studies how dynamical systems behave when observed through coarser measurements. To analyze this, users need to define a single observable—a function that maps states (nodes) to categorical destination values. This capability is essential for the theoretical framework of the tool and enables future analysis of induced dynamics on observable values.

## What Changes

- Add a new "Observable Editor" tab alongside the existing dynamical system graph view
- Introduce a second graph in global state: a bipartite mapping graph with Source nodes (mirroring dynamical system) and Destination nodes (observable values)
- Synchronize Source nodes in mapping graph with dynamical system nodes (add/delete/rename)
- Provide UI for creating, naming, editing, and deleting Destination nodes (observable values)
- Display bipartite graph visualization: left column = Source nodes (alphabetically), right column = Destination nodes, edges only from left to right
- Add heatmap showing mapping (sources horizontal, destinations vertical)
- Reuse existing graph visualization library by marking nodes as Source or Destination types
- Create new bipartite layout module for two-column visualization

## Impact

- Affected specs: `graph-editor`, new `observable-definition` capability
- Affected code:
  - `src/main.rs` - Add tab navigation with `egui_tabs`, second graph state with separate `NodeData` type, node synchronization
  - New module `layout_bipartite.rs` for two-column layout
  - New module for mapping graph `NodeData` with `NodeType` enum (Source/Destination)
  - `heatmap.rs` - Adapt for source-destination mapping display
- Dependencies: Add `egui_tabs` crate to `Cargo.toml`
- Migration: No breaking changes, purely additive feature
- Future work: This lays the foundation for macroscopic dynamics computation (separate change)
