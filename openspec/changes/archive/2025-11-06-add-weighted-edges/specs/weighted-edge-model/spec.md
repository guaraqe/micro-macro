# weighted-edge-model Specification Delta

## ADDED Requirements

### Requirement: Edge Weight Storage

The system SHALL store a floating-point weight value for each edge in both the Dynamical System graph and Observable Editor mapping graph.

#### Scenario: Create edge with default weight

**GIVEN** a user creates a new edge by dragging from source to target node
**WHEN** the edge is added to the graph
**THEN** the edge is created with weight 1.0

#### Scenario: Store custom weight value

**GIVEN** a user sets an edge weight to a specific value
**WHEN** the weight is committed
**THEN** the edge stores the exact floating-point value

#### Scenario: Zero weight removes edge

**GIVEN** an existing edge with non-zero weight
**WHEN** the user sets the weight to 0.0
**THEN** the edge is removed from the graph
**AND** no edge exists between those nodes

### Requirement: Weight Serialization

The system SHALL persist edge weights when saving and loading graph state.

#### Scenario: Save graph with weighted edges

**GIVEN** a graph containing edges with various weights
**WHEN** the graph is saved to JSON
**THEN** each edge's weight is included in the serialized data

#### Scenario: Load graph with weighted edges

**GIVEN** a saved JSON file containing weighted edges
**WHEN** the graph is loaded
**THEN** all edges are restored with their original weight values
