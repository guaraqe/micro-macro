# Project Context

The theoretical framework for this is macroscopic dynamics. Given a discrete dynamical system s: X -> X, and a macroscopic observable Y -> Y, observed the dynamics on the values on Y. Both functions are to be studies using (weighted) graph building.

## Purpose
Tool for studying small dynamical systems and the resulting dynamics on observables (functions). The main tool for building such systems and observables is interactive graph visualization and editing, as well as node trees and adjacency matrices. Progressively functionality will be added for plots and deeper mathematics.

## Tech Stack
- **Language**: Rust
- **GUI Framework**: eframe (egui-based immediate mode GUI)
- **Graph Library**: petgraph (StableGraph with directed edges)
- **Graph Visualization**: egui_graphs
- **Plotting**: egui_plot
- **Serialization**: serde with derive features
- **Build System**: Nix flakes for reproducible development environment

## Project Conventions

### Code Style
- **Naming conventions**: default Rust conventions
- **Documentation**: Inline comments explaining complex workflows
- **Code organization**:
  Modular structure with separate files for major components

### Architecture Patterns
- **Centralized state**: All application state in `GraphEditor` struct
- **Modal editing**: Ctrl key toggles between Node Editor and Edge Editor modes
- **Three-panel layout**:
  - In node view:
    - Left panel: Node list and connection inspector
    - Center panel: Interactive graph visualization
    - Right panel: Adjacency matrix heatmap

### Testing Strategy

**Manual testing only**
- Changes should be verified by the AI:
1. Run `cargo build` and make sure it passes.

No automated tests required at this stage.

### Git Workflow
- **Branch**: Development on `master` branch
- **Commit style**: Keep current informal style with short messages ("Fix", "Working", "Heatmap")
- **No formal process**: Direct commits to master for rapid iteration

## Domain Context

### Dynamical Systems Concepts
This tool models discrete dynamical systems where:
- **States** are represented as nodes in a directed graph
- **Transitions** are directed edges showing how states evolve
- **Analysis** (planned long term) will compute properties like cycles, attractors, and transient behavior

### Graph Editing Model

In the dynamical system tab:

- **Node Editor Mode** (default): Select nodes, edit names, view incoming/outgoing connections
- **Edge Editor Mode** (Ctrl held): Drag from source to target to create edges, click edge twice to delete
- **Layout**: Nodes arranged in circular pattern, sorted alphabetically by default
- **Heatmap**: Shows adjacency matrix with row=target, column=source; hover highlights connections

## Important Constraints

### Performance Requirements
- **Target scale**: Small graphs (<20 nodes) for detailed exploration
- **Memory**: All graph data kept in memory, no external storage currently, but planned

### Technical Constraints
- **Immediate mode rendering**: State management requires careful handling of frame-to-frame updates
- **Single-frame interactions**: Layout resets and mode transitions need explicit state tracking

### Future Features Planned
1. **Graph persistence**: Save/load functionality to persist graphs between sessions
2. **Analysis features**: Compute graph properties (cycles, strongly connected components, etc.)
3. **Graph-based layout**: Layout the graph in more significant ways, circular is just default
3. **Observable definition**: Define functions on nodes via graph editing
4. **Observable dynamics**: From observable and underlying dynamical system, derive macroscopic dynamics

## External Dependencies
All dependencies are Rust crates from crates.io, no external services, APIs, or network dependencies.
