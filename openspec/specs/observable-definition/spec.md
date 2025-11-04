# observable-definition Specification

## Purpose
TBD - created by archiving change add-observable-definition. Update Purpose after archive.
## Requirements
### Requirement: Bipartite Mapping Graph State

The system SHALL maintain a second graph representing the observable mapping as a bipartite graph with Source and Destination node types.

#### Scenario: Initialize mapping graph

- **WHEN** the application starts
- **THEN** the system creates an bipartite graph using the default source nodes
- **AND** created two default destination nodes
- **AND** there is no mapping between this set of nodes yet

#### Scenario: Source nodes mirror dynamical system

- **WHEN** the dynamical system graph contains nodes
- **THEN** the mapping graph Source nodes mirror those nodes with matching names
- **AND** Source nodes are kept synchronized with dynamical system changes

### Requirement: Source Node Synchronization

The system SHALL automatically synchronize Source nodes in the mapping graph when dynamical system nodes change.

#### Scenario: Add node to dynamical system

- **WHEN** a node is added to the dynamical system graph
- **THEN** a corresponding Source node is added to the mapping graph
- **AND** the Source node has the same name
- **AND** no edges are created automatically

#### Scenario: Delete node from dynamical system

- **WHEN** a node is deleted from the dynamical system graph
- **THEN** the corresponding Source node is removed from the mapping graph
- **AND** all edges connected to that Source node are deleted

#### Scenario: Rename node in dynamical system

- **WHEN** a node is renamed in the dynamical system graph
- **THEN** the corresponding Source node name is updated in the mapping graph
- **AND** all edges connected to that Source node are preserved

### Requirement: Destination Node Management

The system SHALL allow users to create, rename, and delete Destination nodes representing observable values.

#### Scenario: Create Destination node

- **WHEN** user creates a new Destination node in the Observable Editor
- **THEN** the system adds a Destination node with a default name (e.g., "Value 0")
- **AND** the node has no incoming edges initially

#### Scenario: Rename Destination node

- **WHEN** user edits a Destination node name
- **THEN** the system updates the name immediately
- **AND** preserves all edges connected to that node

#### Scenario: Delete Destination node

- **WHEN** user deletes a Destination node
- **THEN** the system removes the node and all its incoming edges
- **AND** does not affect Source nodes or the dynamical system

### Requirement: Observable Mapping Edge Creation

The system SHALL allow users to create edges only from Source nodes to Destination nodes in the mapping graph.

#### Scenario: Create mapping edge

- **WHEN** user drags from a Source node to a Destination node in Edge Editor mode
- **THEN** the system creates a directed edge from Source to Destination
- **AND** the edge represents that the source state maps to the destination value

#### Scenario: Prevent invalid edge creation

- **WHEN** user attempts to create an edge between two Source nodes or two Destination nodes
- **THEN** the system prevents the edge creation
- **AND** no edge is added to the graph

#### Scenario: Prevent reverse direction edges

- **WHEN** user attempts to create an edge from Destination to Source
- **THEN** the system prevents the edge creation
- **AND** only left-to-right edges are allowed

### Requirement: Observable Editor Tab Navigation

The system SHALL provide a separate Observable Editor tab with independent UI for the bipartite mapping graph.

#### Scenario: Switch to Observable Editor tab

- **WHEN** user switches to the Observable Editor tab
- **THEN** the system displays the bipartite graph interface
- **AND** hides the dynamical system graph view
- **AND** maintains the state of both graphs

#### Scenario: Switch back to Dynamical System tab

- **WHEN** user switches from Observable Editor back to Dynamical System tab
- **THEN** the system displays the dynamical system graph
- **AND** preserves the mapping graph state
- **AND** Source node synchronization continues to function

### Requirement: Bipartite Graph Layout

The system SHALL display the mapping graph in a two-column bipartite layout.

#### Scenario: Display Source nodes on left

- **WHEN** user views the Observable Editor
- **THEN** Source nodes are displayed in a vertical column on the left
- **AND** nodes are sorted alphabetically by name

#### Scenario: Display Destination nodes on right

- **WHEN** user views the Observable Editor
- **THEN** Destination nodes are displayed in a vertical column on the right
- **AND** nodes can be in any order (or sorted alphabetically)

#### Scenario: Display edges between columns

- **WHEN** mapping edges exist
- **THEN** edges are drawn from left column to right column
- **AND** edges visually connect Source to Destination nodes

### Requirement: Destination Node Panel

The system SHALL provide a left panel for managing Destination nodes and inspecting mappings.

#### Scenario: Create Destination node via panel

- **WHEN** user clicks "Add Destination" button in left panel
- **THEN** a new Destination node is created and appears in the right column
- **AND** the node has a default name that can be edited

#### Scenario: Display reverse mapping on selection

- **WHEN** user selects a Destination node in the left panel
- **THEN** the panel displays all Source nodes that map to this Destination
- **AND** shows incoming edge count

#### Scenario: Edit Destination node name in panel

- **WHEN** user edits a Destination node name in the left panel
- **THEN** the name updates in both the panel and the graph visualization

### Requirement: Observable Mapping Heatmap

The system SHALL display a heatmap showing the mapping between Source and Destination nodes.

#### Scenario: Display mapping heatmap

- **WHEN** user is in the Observable Editor tab
- **THEN** the right panel displays a heatmap
- **AND** Source nodes are arranged horizontally (x-axis)
- **AND** Destination nodes are arranged vertically (y-axis)
- **AND** cells are marked where edges exist (Source maps to Destination)

#### Scenario: Hover highlights connections

- **WHEN** user hovers over a heatmap cell
- **THEN** the corresponding Source and Destination labels are highlighted
- **AND** the edge in the graph visualization is highlighted

#### Scenario: Empty heatmap

- **WHEN** no Destination nodes exist or no edges exist
- **THEN** the heatmap displays an empty grid or placeholder
- **AND** indicates no mappings are defined

