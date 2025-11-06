# Proposal: Add Weighted Edges

## Summary
Add floating-point weights to graph edges with inline editing in the adjacency matrix heatmap and visual thickness scaling in the graph visualization.

## Motivation
Enable users to model weighted dynamical systems and weighted observable mappings by associating numeric values with transitions. This allows representation of probabilities, costs, or strengths of relationships between states.

## Scope
- Both Dynamical System graph and Observable Editor mapping graph
- Data model changes to support edge weights (f32)
- Heatmap: inline weight editing with Enter/Tab commit and display with 1 decimal precision
- Graph visualization: edge thickness proportional to weight (1.0-5.0px range)
- Serialization support for weighted edges
- Zero weight = no edge (edge removal)

## User Experience
Users can click on a heatmap cell to enter a weight value directly via inline text input. They type the value and press Enter to commit (edge appears/updates with corresponding thickness) or Tab to commit and move to the next cell (left-to-right, then down). Escape or clicking away cancels the edit. Setting weight to zero removes the edge. The workflow is fast and visual, with no modal dialogs.

## Dependencies
None. This is a self-contained feature enhancement.

## Risks
- egui_graphs library may not support custom edge thickness (mitigation: investigate alternative rendering approaches)
- Inline editing in heatmap requires careful state management across frames (mitigation: follow egui immediate mode patterns)

## Alternatives Considered
1. Modal dialog for weight editing - rejected as slower workflow
2. Edge clicking in graph view for editing - rejected as less precise than heatmap
3. Real-time updates while typing - rejected to avoid premature edge deletion when typing "0.5"
