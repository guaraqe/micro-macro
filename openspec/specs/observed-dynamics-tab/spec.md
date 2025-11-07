# observed-dynamics-tab Specification

## Purpose
TBD - created by archiving change add-observed-graph-module. Update Purpose after archive.
## Requirements
### Requirement: Observed Dynamics Tab Navigation

The application SHALL provide a third tab "Observed Dynamics" to display the read-only observed graph visualization.

#### Scenario: Display three tabs

**Given** the application is running
**When** viewing the tab bar
**Then** it SHALL display "Dynamical System", "Observable Editor", and "Observed Dynamics" tabs
**And** tabs SHALL be ordered left to right as listed above
**And** all tabs SHALL be clickable

#### Scenario: Switch to Observed Dynamics tab

**Given** the user is on any tab
**When** the user clicks "Observed Dynamics"
**Then** the tab SHALL become active
**And** the observed graph visualization SHALL be displayed
**And** the three-panel layout SHALL be shown

#### Scenario: Default to Dynamical System tab

**Given** the application starts
**When** the initial UI is rendered
**Then** the "Dynamical System" tab SHALL be active by default
**And** the "Observed Dynamics" tab SHALL not be selected initially

### Requirement: Read-Only Node Panel

The Observed Dynamics tab SHALL display a left panel showing the list of observed nodes without any editing capabilities.

#### Scenario: Display observed nodes list

**Given** an observed graph with 3 nodes named "A", "B", "C"
**When** viewing the Observed Dynamics tab left panel
**Then** it SHALL display all 3 node names
**And** nodes SHALL be displayed in alphabetical order
**And** the panel heading SHALL be "Observed Values"

#### Scenario: No add button in observed panel

**Given** the Observed Dynamics tab left panel
**When** examining the panel controls
**Then** there SHALL NOT be an "Add Node" or "Add Value" button
**And** users SHALL NOT be able to add nodes from this panel

#### Scenario: No edit controls for observed nodes

**Given** the Observed Dynamics tab left panel displaying nodes
**When** examining each node entry
**Then** node names SHALL be displayed as read-only text (not editable text fields)
**And** there SHALL NOT be a delete button for nodes
**And** users SHALL NOT be able to modify node names

#### Scenario: Collapsible node details

**Given** an observed node in the left panel
**When** the user clicks the arrow button next to the node
**Then** the node details SHALL expand/collapse
**And** expanded view SHALL show incoming and outgoing connections (when edges are implemented)
**And** this SHALL match the behavior of other tabs' panels

#### Scenario: Node count metadata

**Given** the Observed Dynamics tab left panel
**When** viewing the panel footer
**Then** it SHALL display "Values: N" where N is the count of observed nodes
**And** this SHALL match the pattern used in other tabs

### Requirement: Read-Only Graph Visualization

The Observed Dynamics tab SHALL display a center panel with an interactive but read-only graph visualization.

#### Scenario: Display observed graph in center panel

**Given** an observed graph with nodes
**When** viewing the Observed Dynamics tab center panel
**Then** the observed graph SHALL be rendered using circular layout
**And** nodes SHALL be positioned in a circle, sorted alphabetically
**And** the panel heading SHALL be "Observed Graph"

#### Scenario: No edit modes in observed visualization

**Given** the Observed Dynamics tab center panel
**When** the user presses Ctrl key
**Then** the mode SHALL remain in view-only state
**And** no "Edge Editor" mode SHALL be available
**And** the footer SHALL NOT display mode switching hints

#### Scenario: No node editing via graph

**Given** the observed graph visualization
**When** the user clicks on a node
**Then** the node SHALL be selected (visual feedback)
**And** the node SHALL NOT be editable
**And** no text editing field SHALL appear

#### Scenario: No edge creation in observed graph

**Given** the observed graph visualization in center panel
**When** the user attempts to drag from one node to another
**Then** no edge preview line SHALL be drawn
**And** no edge SHALL be created
**And** dragging SHALL not initiate edge creation workflow

#### Scenario: Show labels toggle works

**Given** the Observed Dynamics tab center panel
**When** the user toggles the "Show Labels" checkbox
**Then** node labels SHALL show/hide accordingly
**And** this SHALL match the behavior of other tabs

### Requirement: Read-Only Heatmap Panel

The Observed Dynamics tab SHALL display a right panel with a read-only adjacency matrix heatmap.

#### Scenario: Display observed graph heatmap

**Given** an observed graph with nodes
**When** viewing the Observed Dynamics tab right panel
**Then** it SHALL display an adjacency matrix heatmap
**And** rows and columns SHALL represent observed nodes, sorted alphabetically
**And** the panel heading SHALL be "Observed Dynamics Heatmap"

#### Scenario: Heatmap shows edge weights

**Given** an observed graph with edges (when edge computation is implemented)
**When** viewing the heatmap
**Then** cells SHALL display edge weights as numbers
**And** empty cells (no edge) SHALL be blank or show "-"
**And** weight display SHALL match the format of other tabs' heatmaps

#### Scenario: Hover highlights but no editing

**Given** the observed graph heatmap
**When** the user hovers over a cell
**Then** the cell SHALL be highlighted
**And** corresponding nodes SHALL be highlighted in the graph visualization
**And** clicking the cell SHALL NOT open an edit field

#### Scenario: No weight editing in observed heatmap

**Given** the observed graph heatmap
**When** the user clicks on a cell with a weight
**Then** no text input field SHALL appear
**And** the weight SHALL remain read-only
**And** users SHALL NOT be able to modify edge weights

#### Scenario: Edge count metadata

**Given** the Observed Dynamics tab right panel
**When** viewing the panel footer
**Then** it SHALL display "Edges: N" where N is the count of edges in the observed graph
**And** this SHALL match the pattern used in other tabs

### Requirement: Consistent Layout with Other Tabs

The Observed Dynamics tab SHALL use the same three-panel layout structure as the Dynamical System and Observable Editor tabs.

#### Scenario: Three-panel split

**Given** the Observed Dynamics tab
**When** viewing the layout
**Then** it SHALL have left, center, and right panels
**And** each panel SHALL occupy exactly 1/3 of the available width
**And** this SHALL match the layout proportions of other tabs

#### Scenario: Panel styling consistency

**Given** all three tabs
**When** comparing panel styling
**Then** panel frames, margins, and separators SHALL be consistent
**And** heading fonts and sizes SHALL match across tabs
**And** footer layouts SHALL match across tabs

### Requirement: Observed Graph Display Integration

The observed graph SHALL integrate with existing graph visualization infrastructure, reusing display and layout components.

#### Scenario: Use existing graph display setup

**Given** the observed graph needs visualization
**When** setting up the display graph
**Then** it SHALL use `setup_graph_display(&observed_graph)` from `graph_view` module
**And** nodes SHALL have labels set to their names
**And** node sizes SHALL be 75% of default (matching other graphs)

#### Scenario: Apply circular layout

**Given** the observed graph visualization in center panel
**When** the layout is computed
**Then** it SHALL use `LayoutStateCircular` layout algorithm
**And** the layout SHALL be reset when the observed graph is recomputed
**And** this SHALL match the layout used for the dynamical system tab

#### Scenario: Create ObservedGraphView type

**Given** the `graph_view` module
**When** examining type aliases
**Then** it SHALL define `ObservedGraphView` type alias
**And** the type SHALL use `ObservedNode` as the node type
**And** it SHALL use `LayoutStateCircular` for layout (same as state graph)
**And** it SHALL follow the same pattern as `StateGraphView`

