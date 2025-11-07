# observed-graph-computation Specification

## Purpose
TBD - created by archiving change add-observed-graph-module. Update Purpose after archive.
## Requirements
### Requirement: Observed Graph Derivation Function

The application SHALL provide a function to compute the `ObservedGraph` from the `StateGraph` and `ObservableGraph`, deriving nodes from Destination nodes in the observable graph.

#### Scenario: Calculate observed graph from source graphs

**Given** a `StateGraph` and an `ObservableGraph`
**When** `calculate_observed_graph(&state_graph, &observable_graph)` is called
**Then** it SHALL return a new `ObservedGraph`
**And** the computation SHALL be deterministic (same inputs produce same output)
**And** the function SHALL not modify the input graphs

#### Scenario: Derive nodes from observable destinations

**Given** an `ObservableGraph` with 5 Destination nodes
**When** calculating the observed graph
**Then** the resulting `ObservedGraph` SHALL have exactly 5 nodes
**And** each node SHALL correspond to one Destination node
**And** each `ObservedNode` SHALL have `observable_node_idx` pointing to its source Destination node

#### Scenario: Copy destination node names

**Given** a Destination node in `ObservableGraph` with name "Temperature"
**When** calculating the observed graph
**Then** the corresponding `ObservedNode` SHALL have name "Temperature"
**And** the name SHALL be copied from the Destination node
**And** names SHALL match exactly (no transformations)

#### Scenario: Handle empty observable graph

**Given** an `ObservableGraph` with no Destination nodes (only Source nodes)
**When** calculating the observed graph
**Then** the resulting `ObservedGraph` SHALL be empty (no nodes, no edges)
**And** the function SHALL not panic or error

### Requirement: Placeholder Edge Logic

Initially, the observed graph computation SHALL create a graph structure with no edges, as the edge computation logic is complex and will be implemented by the user later.

#### Scenario: No edges in initial implementation

**Given** any `StateGraph` and `ObservableGraph` with edges
**When** calculating the observed graph
**Then** the resulting `ObservedGraph` SHALL have zero edges
**And** nodes SHALL exist but remain unconnected
**And** this SHALL serve as a placeholder for future edge computation logic

#### Scenario: Document edge computation placeholder

**Given** the `calculate_observed_graph` function
**When** examining its documentation
**Then** it SHALL include a comment noting "TODO: Implement edge computation logic"
**And** it SHALL explain that edges will be computed based on state transitions and observable mappings
**And** documentation SHALL indicate this is for future user implementation

### Requirement: Automatic Recomputation Trigger

The application SHALL automatically recompute the observed graph whenever the state graph or observable graph changes.

#### Scenario: Recompute on state graph change

**Given** an existing observed graph
**When** a node is added to or removed from the state graph
**Then** the observed graph SHALL be recomputed
**And** the display SHALL update to reflect the new computation

#### Scenario: Recompute on observable graph change

**Given** an existing observed graph
**When** a Destination node is added, removed, or renamed in the observable graph
**Then** the observed graph SHALL be recomputed
**And** the corresponding observed nodes SHALL reflect the changes

#### Scenario: Recompute on observable edge change

**Given** an existing observed graph
**When** an edge is added or removed in the observable graph
**Then** the observed graph SHALL be recomputed
**And** this SHALL prepare for future edge computation (currently no-op since edges are placeholder)

### Requirement: Node Index Mapping

The observed graph computation SHALL maintain a clear mapping between observed nodes and their source destination nodes through stored node indices.

#### Scenario: Store observable node index in observed node

**Given** a Destination node at index `idx` in the observable graph
**When** calculating the observed graph
**Then** the corresponding `ObservedNode` SHALL store `idx` in its `observable_node_idx` field
**And** this reference SHALL remain valid as long as the Destination node exists

#### Scenario: Query source destination from observed node

**Given** an `ObservedNode` with `observable_node_idx = idx`
**When** looking up `observable_graph.node(idx)`
**Then** it SHALL return the source Destination node
**And** the Destination node's name SHALL match the observed node's name
**And** the Destination node type SHALL be `ObservableNodeType::Destination`

