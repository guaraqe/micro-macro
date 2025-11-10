# state-node-weights Specification

## Purpose
Enable modeling of probability distributions over states by allowing users to assign weights to nodes in the dynamical system graph.

## ADDED Requirements

### Requirement: State Node Weight Model
The system SHALL store a weight value for each node in the state graph.

#### Scenario: Node has weight field
- **GIVEN** a StateNode in the dynamical system graph
- **WHEN** the node is created
- **THEN** the node has a weight field of type f32
- **AND** the weight is initialized to 1.0 by default

#### Scenario: Weight accepts positive values
- **GIVEN** a StateNode with a weight field
- **WHEN** the weight is set to any positive value
- **THEN** the system accepts and stores the value
- **AND** negative values are clamped to 0.0 if entered

#### Scenario: Weight persists across application lifecycle
- **GIVEN** a StateNode with weight value W
- **WHEN** the application saves and loads the graph
- **THEN** the node's weight is restored to value W
- **AND** no data loss occurs

### Requirement: Weight Editing UI
The system SHALL provide UI controls for editing node weights in the left panel of the Dynamical System tab.

#### Scenario: Display weight editor in node list
- **GIVEN** the Dynamical System tab is active
- **WHEN** the left panel displays the node list
- **THEN** each node entry shows a "Weight:" label and text input field
- **AND** the input field displays the current weight value

#### Scenario: Edit weight via text input
- **GIVEN** a node with weight W₁
- **WHEN** user types a new value W₂ in the weight input field
- **AND** W₂ is a valid positive number
- **THEN** the node's weight is updated to W₂ immediately
- **AND** the observed graph is recomputed

#### Scenario: Invalid weight input
- **GIVEN** a node with weight W
- **WHEN** user types an invalid value (non-numeric, negative)
- **THEN** the system either rejects the input or clamps to valid range
- **AND** the node retains its previous valid weight

#### Scenario: Weight change triggers recomputation
- **GIVEN** a state graph with weighted nodes
- **WHEN** any node's weight is modified
- **THEN** the system triggers observed graph weight recomputation
- **AND** observed node weights update accordingly

### Requirement: Optional Weight Display in Graph Visualization
The system SHALL allow users to toggle whether weights are displayed in the graph visualization.

#### Scenario: Weight display toggle control
- **GIVEN** the Dynamical System tab is active
- **WHEN** the user views the bottom controls section
- **THEN** a "Show Weights" checkbox is visible
- **AND** the checkbox state can be toggled on/off

#### Scenario: Weights hidden by default
- **GIVEN** the application starts or a tab is opened
- **WHEN** no prior preference is set
- **THEN** the "Show Weights" toggle is OFF by default
- **AND** weights are not displayed in the graph visualization

#### Scenario: Show weights in graph when enabled
- **GIVEN** the "Show Weights" toggle is turned ON
- **WHEN** the graph is rendered
- **THEN** each node displays its weight value alongside or below its name
- **AND** the format is clear and readable (e.g., "Node A (1.5)")

#### Scenario: Hide weights in graph when disabled
- **GIVEN** the "Show Weights" toggle is turned OFF
- **WHEN** the graph is rendered
- **THEN** nodes display only their names
- **AND** no weight information is visible in the visualization

### Requirement: Weight Serialization
The system SHALL include node weights when saving and loading graph state to/from JSON files.

#### Scenario: Save weights to file
- **GIVEN** a state graph with nodes having various weights
- **WHEN** the user saves the graph to a JSON file
- **THEN** the file contains weight values for each node
- **AND** the JSON structure includes a "weight" field per node

#### Scenario: Load weights from file
- **GIVEN** a saved JSON file containing node weights
- **WHEN** the user loads the file
- **THEN** each node is restored with its saved weight value
- **AND** the UI displays the loaded weights correctly

#### Scenario: Backward compatibility with weightless files
- **GIVEN** a JSON file from an older version without weight fields
- **WHEN** the user loads the file
- **THEN** all nodes are assigned the default weight of 1.0
- **AND** no errors occur during loading

### Requirement: Weight Display in Left Panel
The system SHALL always display node weights in the left panel node list regardless of the graph visualization toggle.

#### Scenario: Weights visible in panel
- **GIVEN** the Dynamical System tab is active
- **WHEN** the left panel displays nodes
- **THEN** each node's weight is visible in the weight input field
- **AND** weights are displayed independent of the "Show Weights" graph toggle
