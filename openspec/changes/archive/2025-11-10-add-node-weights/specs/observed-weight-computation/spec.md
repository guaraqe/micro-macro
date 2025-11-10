# observed-weight-computation Specification

## Purpose
Automatically compute observed node weights by propagating state node weights through the observable mapping using probability mathematics from the markov crate.

## ADDED Requirements

### Requirement: Observed Node Weight Model
The system SHALL store computed weight values for nodes in the observed graph.

#### Scenario: Observed node has weight field
- **GIVEN** an ObservedNode in the observed dynamics graph
- **WHEN** the node is created or updated
- **THEN** the node has a weight field of type f32
- **AND** the weight reflects the computed probability mass

#### Scenario: Weight is read-only in UI
- **GIVEN** an observed node with a computed weight
- **WHEN** the user views the Observed Dynamics tab
- **THEN** the weight is displayed but not editable
- **AND** weights can only be changed by modifying source state or observable graphs

### Requirement: Weight Computation Function
The system SHALL provide a self-contained function that computes observed weights from state and observable graphs.

#### Scenario: Compute weights from valid inputs
- **GIVEN** a state graph with N weighted nodes
- **AND** an observable graph with M destination nodes and weighted edges
- **WHEN** the computation function is called
- **THEN** it returns a mapping of destination node indices to computed weights
- **AND** the computation follows: Prob(state_weights).dot(Markov(observable_edges))

#### Scenario: State weights to Prob conversion
- **GIVEN** state nodes with weights [w₁, w₂, ..., wₙ]
- **WHEN** the function constructs a Prob vector
- **THEN** it creates Prob::from_assoc(N, [(idx₁, w₁), (idx₂, w₂), ...])
- **AND** uses NodeIndex as the label type
- **AND** uses f64 for numerical precision

#### Scenario: Observable edges to Markov conversion
- **GIVEN** observable graph edges from sources S to destinations D with weights
- **WHEN** the function constructs a Markov matrix
- **THEN** it creates Markov::from_assoc(|S|, |D|, [(s_idx, d_idx, weight), ...])
- **AND** uses NodeIndex for row and column labels
- **AND** the matrix is row-stochastic (each source's edges sum to 1.0)

#### Scenario: Probability propagation via dot product
- **GIVEN** a Prob vector P over state nodes
- **AND** a Markov matrix M mapping states to observed values
- **WHEN** the function computes P.dot(&M)
- **THEN** it returns a new Prob vector over destination nodes
- **AND** each destination weight equals Σᵢ P[i] × M[i,j]

#### Scenario: Empty state graph error
- **GIVEN** an empty state graph (no nodes)
- **WHEN** the computation function is called
- **THEN** it returns an error indicating empty state graph
- **AND** no observed weights are computed

#### Scenario: Empty observed graph
- **GIVEN** an observable graph with no destination nodes
- **WHEN** the computation function is called
- **THEN** it returns an empty weight mapping (valid but trivial)
- **AND** no error occurs

#### Scenario: Observable with no edges
- **GIVEN** an observable graph with destination nodes but no edges
- **WHEN** the computation function is called
- **THEN** all destination nodes receive weight 0.0
- **AND** computation succeeds (valid configuration)

### Requirement: Automatic Recomputation
The system SHALL automatically recompute observed weights whenever the state or observable graph changes.

#### Scenario: Recompute on state weight change
- **GIVEN** observed graph with computed weights
- **WHEN** a state node's weight is modified
- **THEN** observed weights are recomputed immediately
- **AND** the Observed Dynamics tab displays updated weights

#### Scenario: Recompute on state graph topology change
- **GIVEN** observed graph with computed weights
- **WHEN** a state node is added or removed
- **THEN** observed weights are recomputed
- **AND** the computation uses the updated set of state nodes

#### Scenario: Recompute on observable edge change
- **GIVEN** observed graph with computed weights
- **WHEN** an observable edge is added, removed, or its weight changes
- **THEN** observed weights are recomputed
- **AND** the new Markov matrix reflects the updated mapping

#### Scenario: Recompute on observable destination change
- **GIVEN** observed graph with computed weights
- **WHEN** a destination node is added or removed in the observable
- **THEN** the observed graph structure is updated
- **AND** weights are recomputed for the new set of destinations

### Requirement: Weight Display in Observed Dynamics Tab
The system SHALL display computed weights in the Observed Dynamics tab with optional graph visualization display.

#### Scenario: Display weights in left panel
- **GIVEN** the Observed Dynamics tab is active
- **WHEN** the left panel displays observed nodes
- **THEN** each node entry shows its computed weight value
- **AND** the weight is displayed as read-only information

#### Scenario: Optional weight display in graph
- **GIVEN** the Observed Dynamics tab with the "Show Weights" toggle
- **WHEN** the toggle is ON
- **THEN** observed node weights are displayed in the graph visualization
- **AND** the format matches the state graph weight display

#### Scenario: Hide weights in graph when toggle off
- **GIVEN** the "Show Weights" toggle is OFF
- **WHEN** viewing the observed graph visualization
- **THEN** only node names are shown
- **AND** weights remain visible in the left panel

### Requirement: Error Handling and Validation
The system SHALL handle edge cases and invalid configurations gracefully during weight computation.

#### Scenario: Handle probability construction errors
- **GIVEN** state weights that violate Prob construction rules
- **WHEN** Prob::from_assoc returns an error
- **THEN** the computation function propagates the error
- **AND** the system logs or displays the error to help debugging

#### Scenario: Handle Markov construction errors
- **GIVEN** observable edges that violate Markov construction rules
- **WHEN** Markov::from_assoc returns an error
- **THEN** the computation function propagates the error
- **AND** observed weights remain unchanged or are set to zero

#### Scenario: Handle missing source nodes
- **GIVEN** an observable graph with source nodes not in the state graph
- **WHEN** attempting to compute weights
- **THEN** the function detects the inconsistency
- **AND** returns an error or ignores unmapped sources

### Requirement: Computation Isolation
The system SHALL implement weight computation as a pure function separate from UI concerns.

#### Scenario: Function signature is self-contained
- **GIVEN** the weight computation function
- **WHEN** examining its signature
- **THEN** it takes state_graph and observable_graph as inputs
- **AND** returns Result<HashMap<NodeIndex, f64>, Error>
- **AND** has no side effects or UI dependencies

#### Scenario: Testable independently
- **GIVEN** the weight computation function
- **WHEN** unit tests are written
- **THEN** tests can construct minimal graphs and verify computations
- **AND** no UI or application state is required for testing
