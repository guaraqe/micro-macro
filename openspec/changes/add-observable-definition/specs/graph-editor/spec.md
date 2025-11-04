## MODIFIED Requirements

### Requirement: Application Tab Navigation

The system SHALL provide tab navigation between Dynamical System and Observable Editor views.

#### Scenario: Display available tabs

- **WHEN** the application is running
- **THEN** the system displays two tabs: "Dynamical System" and "Observable Editor"
- **AND** the active tab is visually indicated

#### Scenario: Switch between tabs

- **WHEN** user clicks a different tab
- **THEN** the system switches the view to the selected tab
- **AND** preserves state in both tabs independently

#### Scenario: Default tab on startup

- **WHEN** the application starts
- **THEN** the Dynamical System tab is displayed by default
- **AND** the Observable Editor tab is available but not shown

## ADDED Requirements

### Requirement: Node Change Propagation

The system SHALL propagate node changes from the Dynamical System graph to the Observable Editor's Source nodes.

#### Scenario: Propagate node addition

- **WHEN** a node is added in the Dynamical System tab
- **THEN** the change is immediately reflected in the Observable Editor's Source nodes
- **AND** synchronization happens even when Observable Editor tab is not visible

#### Scenario: Propagate node deletion

- **WHEN** a node is deleted in the Dynamical System tab
- **THEN** the corresponding Source node is removed in the Observable Editor
- **AND** all edges to that Source node are deleted in the mapping graph

#### Scenario: Propagate node rename

- **WHEN** a node is renamed in the Dynamical System tab
- **THEN** the corresponding Source node name is updated in the Observable Editor
- **AND** the update is visible when switching to the Observable Editor tab
