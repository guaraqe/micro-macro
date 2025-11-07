# Proposal: Add Observed Graph Module

## Overview

Introduce a third graph type (`ObservedGraph`) to model macroscopic dynamics derived from the existing dynamical system and observable definition. This requires refactoring the graph state management from the monolithic `main.rs` structure into a separate, independently manipulable module.

## Problem Statement

Currently, the application manages two graphs (`StateGraph` for the dynamical system and `ObservableGraph` for observable mappings) directly within the `GraphEditor` struct in `main.rs`. To support the third graph representing observed dynamics, we need:

1. **Separation of concerns**: Graph state should be managed independently from UI logic
2. **Derived graph computation**: A mechanism to compute `ObservedGraph` from the other two graphs
3. **Consistent API**: Common operations (add/remove nodes, edges, etc.) across all graph types

## Motivation

The observed graph represents the macroscopic dynamics induced by:
- The underlying dynamical system (state transitions)
- The observable function (mapping states to values)

This is a core concept in macroscopic dynamics theory. By separating graph management and providing a computation function, we enable:
- Clear separation between UI and domain logic
- Testability of graph operations
- Future extensibility for additional derived graphs

## Proposed Solution

### 1. Graph Module Refactoring

Create a new `graph_state` module that encapsulates:
- All three graph types with their node types
- Common graph manipulation operations
- Conversion utilities

### 2. Observed Graph Implementation

- **Nodes**: `ObservedNode` contains same data as `StateNode` (just a name), but represents observable values
- **Node derivation**: Nodes in `ObservedGraph` correspond to destination nodes in `ObservableGraph`
- **Computation function**: `compute_observed_graph(state_graph: &StateGraph, observable_graph: &ObservableGraph) -> ObservedGraph`
- **Edge logic**: Initially placeholder (user will implement complex edge logic later)

### 3. Integration Points

- Update `GraphEditor` to use the new module
- Trigger recomputation when either source graph changes (State or Observable)
- Add third tab "Observed Dynamics" with same three-panel layout:
  - **Left panel**: Read-only node list (no add/edit/delete buttons)
  - **Center panel**: Read-only graph visualization (no node/edge editing modes)
  - **Right panel**: Read-only adjacency matrix heatmap (no weight editing)
- All editing must be done via the Observable Editor tab (tab 2)

## Success Criteria

1. All three graphs managed through consistent API
2. Graph state separated from UI code
3. Observed graph correctly derives nodes from observable destinations
4. Third tab displays observed graph in read-only mode with all visualization features
5. Observed graph automatically recomputes when observable or state graph changes
6. Existing functionality unchanged (backward compatibility)
7. Code compiles and runs without errors

## Out of Scope

- Complex edge computation logic for observed graph (user will implement)
- Editing capabilities in the observed graph tab (strictly read-only)
- Performance optimization for large graphs
- Advanced graph analysis features

## Dependencies

- Existing `StateGraph` and `ObservableGraph` implementations
- `petgraph` library for graph data structures
- Current serialization infrastructure (will need extension)
- Existing three-panel layout pattern from other tabs

## Risks and Mitigations

| Risk | Mitigation |
|------|------------|
| Breaking existing code | Incremental refactoring with backward compatibility |
| Unclear edge computation requirements | Leave edge logic as placeholder/empty initially |
| Serialization complexity | Observed graph is derived, doesn't need serialization |
| UI code duplication | Reuse existing panel rendering logic with read-only flags |

## Timeline Estimate

- Graph module extraction: 2-3 tasks
- Observed graph types and derivation: 2-3 tasks
- Third tab UI with read-only visualization: 3-4 tasks
- Integration and testing: 2-3 tasks
- Total: ~9-13 implementation tasks
