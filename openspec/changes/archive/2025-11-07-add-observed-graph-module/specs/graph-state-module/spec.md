# Spec: Graph State Module

## ADDED Requirements

### Requirement: Separate Graph State Module

The application SHALL provide a dedicated `graph_state` module that encapsulates all graph type definitions and common operations, separating graph domain logic from UI presentation logic.

#### Scenario: Define all graph types in module

**Given** the application codebase
**When** examining the `graph_state` module
**Then** it SHALL export `StateGraph`, `ObservableGraph`, and `ObservedGraph` types
**And** it SHALL export corresponding node types: `StateNode`, `ObservableNode`, and `ObservedNode`
**And** all type definitions SHALL be in the `graph_state` module, not scattered across files

#### Scenario: Import graph types from module

**Given** a source file that needs graph types (e.g., `main.rs`, `graph_view.rs`)
**When** the file imports graph-related types
**Then** imports SHALL come from the `graph_state` module
**And** no direct petgraph type aliases SHALL be defined outside the module

### Requirement: Consistent Graph Construction

The graph state module SHALL provide consistent initialization functions for all graph types following the existing patterns.

#### Scenario: Create default state graph

**Given** the application starts without saved state
**When** `default_state_graph()` is called
**Then** it SHALL return a `StateGraph` with 3 default nodes
**And** nodes SHALL be connected in a cycle with weight 1.0
**And** this SHALL match existing behavior

#### Scenario: Create default observable graph

**Given** a state graph exists
**When** `default_observable_graph(&state_graph)` is called
**Then** it SHALL return an `ObservableGraph` with Source nodes mirroring the state graph
**And** it SHALL include 2 default Destination nodes
**And** this SHALL match existing behavior

### Requirement: Graph Type Consistency

All graph types SHALL use the same underlying structure with different node types, maintaining consistency with existing patterns.

#### Scenario: Use StableGraph for all types

**Given** the graph type definitions
**When** examining their implementation
**Then** `StateGraph` SHALL be `StableGraph<StateNode, f32>`
**And** `ObservableGraph` SHALL be `StableGraph<ObservableNode, f32>`
**And** `ObservedGraph` SHALL be `StableGraph<ObservedNode, f32>`
**And** all SHALL use `f32` edge weights

#### Scenario: Maintain existing node traits

**Given** the node type definitions
**When** examining their implementations
**Then** all node types SHALL implement the `HasName` trait
**And** `HasName::name()` SHALL return the node's name as `String`
**And** this SHALL enable consistent label extraction across graph types

### Requirement: ObservedNode Structure

The `ObservedNode` type SHALL contain a reference to its corresponding Destination node in the `ObservableGraph`, enabling easy cross-referencing between graphs.

#### Scenario: Store observable node reference

**Given** an `ObservedNode` definition
**When** examining its fields
**Then** it SHALL have a `name: String` field (same as `StateNode`)
**And** it SHALL have an `observable_node_idx: NodeIndex` field
**And** the `observable_node_idx` SHALL reference the corresponding Destination node in the `ObservableGraph`

#### Scenario: Access observable reference

**Given** an observed graph node with index `obs_idx`
**When** accessing the node's data
**Then** `node.observable_node_idx` SHALL return the `NodeIndex` of the corresponding Destination node
**And** this index SHALL be valid in the source `ObservableGraph`
**And** looking up that index SHALL return the Destination node that generated this observed node
