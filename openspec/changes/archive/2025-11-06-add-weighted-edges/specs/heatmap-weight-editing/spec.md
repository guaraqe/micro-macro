# heatmap-weight-editing Specification Delta

## ADDED Requirements

### Requirement: Weight Display in Heatmap

The system SHALL display edge weights in the adjacency matrix heatmap with one decimal place precision.

#### Scenario: Display existing weight

**GIVEN** an edge exists with weight 2.5
**WHEN** the heatmap is rendered
**THEN** the corresponding cell displays "2.5"

#### Scenario: Display no edge

**GIVEN** no edge exists between two nodes
**WHEN** the heatmap is rendered
**THEN** the corresponding cell displays no weight value
**AND** uses the standard empty cell appearance

#### Scenario: Format weight precision

**GIVEN** an edge with weight 1.0
**WHEN** the heatmap is rendered
**THEN** the cell displays "1.0" with one decimal place

### Requirement: Inline Weight Editing

The system SHALL provide inline text editing for edge weights directly in heatmap cells.

#### Scenario: Start editing on click

**GIVEN** a heatmap cell (empty or with existing weight)
**WHEN** the user clicks the cell
**THEN** a text input appears in the cell
**AND** the input shows the current weight value or is empty
**AND** the input has keyboard focus

#### Scenario: Commit edit with Enter

**GIVEN** the user is editing a cell with value "3.7"
**WHEN** the user presses Enter
**THEN** the weight is committed to the edge
**AND** the edge appears or updates in the graph
**AND** editing mode exits

#### Scenario: Commit and navigate with Tab

**GIVEN** the user is editing a cell at position (x, y)
**WHEN** the user presses Tab
**THEN** the weight is committed to the edge
**AND** editing moves to the next cell (left-to-right, then down)
**AND** the next cell enters edit mode

#### Scenario: Cancel edit with Escape

**GIVEN** the user is editing a cell
**WHEN** the user presses Escape
**THEN** the edit is cancelled
**AND** the original weight value is preserved
**AND** editing mode exits

#### Scenario: Cancel edit by clicking away

**GIVEN** the user is editing a cell
**WHEN** the user clicks outside the cell
**THEN** the edit is cancelled
**AND** the original weight value is preserved
**AND** editing mode exits

#### Scenario: No intermediate updates while typing

**GIVEN** the user is typing "0.5" in a cell
**WHEN** they type "0" (first character)
**THEN** the graph does not update
**AND** the edge does not disappear
**UNTIL** the user commits with Enter or Tab
