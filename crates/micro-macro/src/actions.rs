use crate::effects::Effect;
use crate::graph_state::{
    ObservableNode, ObservableNodeType, StateNode,
};
use crate::store::{ActiveTab, EditMode, Store};
use eframe::egui;
use petgraph::stable_graph::{EdgeIndex, NodeIndex};
use std::path::PathBuf;

/// Actions that can be dispatched to modify the editor state
#[derive(Debug, Clone)]
pub enum Action {
    // State Graph Node Actions
    /// Add a new node to the state graph
    AddStateNode { name: String, weight: f32 },
    /// Remove a node from the state graph
    RemoveStateNode { node_idx: NodeIndex },
    /// Rename a state graph node
    RenameStateNode {
        node_idx: NodeIndex,
        new_name: String,
    },
    /// Update the weight of a state graph node
    UpdateStateNodeWeight {
        node_idx: NodeIndex,
        new_weight: f32,
    },
    /// Set the selection state of a node
    SelectStateNode { node_idx: NodeIndex, selected: bool },

    // State Graph Edge Actions
    /// Add an edge between two nodes in the state graph
    AddStateEdge {
        source_idx: NodeIndex,
        target_idx: NodeIndex,
        weight: f32,
    },
    /// Remove an edge from the state graph (by edge index)
    RemoveStateEdgeByIndex { edge_idx: EdgeIndex },
    /// Special case for heatmap editing (weight of 0.0 removes edge)
    UpdateStateEdgeWeightFromHeatmap {
        source_idx: NodeIndex,
        target_idx: NodeIndex,
        new_weight: f32,
    },

    // Observable Graph Actions
    /// Add a new Destination node in the observable graph
    AddObservableDestinationNode { name: String },
    /// Remove a Destination node from the observable graph
    RemoveObservableDestinationNode { node_idx: NodeIndex },
    /// Rename an observable Destination node
    RenameObservableDestinationNode {
        node_idx: NodeIndex,
        new_name: String,
    },

    // Observable Edge Actions
    /// Add a observable edge from Source to Destination
    AddObservableEdge {
        source_idx: NodeIndex,
        target_idx: NodeIndex,
        weight: f32,
    },
    /// Remove a observable edge (by edge index)
    RemoveObservableEdgeByIndex { edge_idx: EdgeIndex },
    /// Special case for heatmap editing
    UpdateObservableEdgeWeightFromHeatmap {
        source_idx: NodeIndex,
        target_idx: NodeIndex,
        new_weight: f32,
    },

    // UI State Actions
    /// Change between NodeEditor and EdgeEditor modes
    SetEditMode { mode: EditMode },
    /// Switch between DynamicalSystem, ObservableEditor, and ObservedDynamics tabs
    SetActiveTab { tab: ActiveTab },
    /// Toggle node label visibility
    SetShowLabels { show: bool },
    /// Toggle weight display
    SetShowWeights { show: bool },
    /// Clear the state graph layout reset flag
    ClearStateLayoutResetFlag,
    /// Clear the observable graph layout reset flag
    ClearObservableLayoutResetFlag,
    /// Clear the observed graph layout reset flag
    ClearObservedLayoutResetFlag,
    /// Clear all selected edges in the state graph
    ClearEdgeSelections,
    /// Clear all selected edges in the observable graph
    ClearObservableEdgeSelections,
    /// Set the drag start state for edge creation
    SetDraggingFrom {
        node_idx: Option<NodeIndex>,
        position: Option<egui::Pos2>,
    },
    /// Indicate whether a drag operation has started
    SetDragStarted { started: bool },
    /// Set the currently hovered cell in the heatmap
    SetHeatmapHoveredCell { cell: Option<(usize, usize)> },
    /// Set the currently editing cell in the heatmap
    SetHeatmapEditingCell { cell: Option<(usize, usize)> },
    /// Set the text buffer for heatmap editing
    SetHeatmapEditBuffer { buffer: String },

    // File Operations
    /// Save current project to file
    SaveToFile { path: PathBuf },
    /// Load project from file
    LoadFromFile { path: PathBuf },
    /// Clear any error message
    ClearErrorMessage,
}

/// Apply a single action to modify the store state
pub fn update(store: &mut Store, action: Action) -> Vec<Effect> {
    match action {
        // State Graph Node Actions
        Action::AddStateNode { name, weight } => {
            let node_idx = store.state_graph.add_node(StateNode {
                name: name.clone(),
                weight,
            });
            if let Some(node) = store.state_graph.node_mut(node_idx) {
                node.set_label(name);
            }
            store.state_layout_reset_needed = true;
            store.observed_layout_reset_needed = true;
            store.observable_layout_reset_needed = true;
            store.sync_source_nodes();
            store.recompute_observed_graph();
            vec![]
        }
        Action::RemoveStateNode { node_idx } => {
            store.state_graph.remove_node(node_idx);
            store.state_layout_reset_needed = true;
            store.observed_layout_reset_needed = true;
            store.observable_layout_reset_needed = true;
            store.sync_source_nodes();
            store.recompute_observed_graph();
            vec![]
        }
        Action::RenameStateNode { node_idx, new_name } => {
            if let Some(node) = store.state_graph.node_mut(node_idx) {
                node.payload_mut().name = new_name.clone();
                node.set_label(new_name);
            }
            store.state_layout_reset_needed = true;
            store.observed_layout_reset_needed = true;
            store.observable_layout_reset_needed = true;
            store.sync_source_nodes();
            store.recompute_observed_graph();
            vec![]
        }
        Action::UpdateStateNodeWeight {
            node_idx,
            new_weight,
        } => {
            if let Some(node) = store.state_graph.node_mut(node_idx) {
                node.payload_mut().weight = new_weight;
            }
            store.recompute_observed_graph();
            vec![]
        }
        Action::SelectStateNode { node_idx, selected } => {
            if let Some(node) = store.state_graph.node_mut(node_idx) {
                node.set_selected(selected);
            }
            vec![]
        }

        // State Graph Edge Actions
        Action::AddStateEdge {
            source_idx,
            target_idx,
            weight,
        } => {
            store.state_graph.add_edge_with_label(
                source_idx,
                target_idx,
                weight,
                String::new(),
            );
            vec![]
        }
        Action::RemoveStateEdgeByIndex { edge_idx } => {
            store.state_graph.remove_edge(edge_idx);
            vec![]
        }
        Action::UpdateStateEdgeWeightFromHeatmap {
            source_idx,
            target_idx,
            new_weight,
        } => {
            if new_weight == 0.0 {
                if let Some(edge_idx) = store
                    .state_graph
                    .g()
                    .find_edge(source_idx, target_idx)
                {
                    store.state_graph.remove_edge(edge_idx);
                }
            } else if let Some(edge_idx) = store
                .state_graph
                .g()
                .find_edge(source_idx, target_idx)
            {
                if let Some(edge) =
                    store.state_graph.edge_mut(edge_idx)
                {
                    *edge.payload_mut() = new_weight;
                }
            } else {
                store.state_graph.add_edge_with_label(
                    source_idx,
                    target_idx,
                    new_weight,
                    String::new(),
                );
            }
            vec![]
        }

        // Observable Graph Actions
        Action::AddObservableDestinationNode { name } => {
            let node_idx =
                store.observable_graph.add_node(ObservableNode {
                    name: name.clone(),
                    node_type: ObservableNodeType::Destination,
                    state_node_idx: None,
                });
            if let Some(node) =
                store.observable_graph.node_mut(node_idx)
            {
                node.set_label(name);
            }
            store.observable_layout_reset_needed = true;
            store.recompute_observed_graph();
            vec![]
        }
        Action::RemoveObservableDestinationNode { node_idx } => {
            store.observable_graph.remove_node(node_idx);
            store.observable_layout_reset_needed = true;
            store.recompute_observed_graph();
            vec![]
        }
        Action::RenameObservableDestinationNode {
            node_idx,
            new_name,
        } => {
            if let Some(node) =
                store.observable_graph.node_mut(node_idx)
            {
                node.payload_mut().name = new_name.clone();
                node.set_label(new_name);
            }
            store.recompute_observed_graph();
            vec![]
        }

        // Observable Edge Actions
        Action::AddObservableEdge {
            source_idx,
            target_idx,
            weight,
        } => {
            // Validate that source is Source type and target is Destination type
            if let Some(source_node) =
                store.observable_graph.node(source_idx)
            {
                if source_node.payload().node_type
                    == ObservableNodeType::Source
                {
                    if let Some(target_node) =
                        store.observable_graph.node(target_idx)
                    {
                        if target_node.payload().node_type
                            == ObservableNodeType::Destination
                        {
                            store
                                .observable_graph
                                .add_edge_with_label(
                                    source_idx,
                                    target_idx,
                                    weight,
                                    String::new(),
                                );
                            store.recompute_observed_graph();
                        }
                    }
                }
            }
            vec![]
        }
        Action::RemoveObservableEdgeByIndex { edge_idx } => {
            store.observable_graph.remove_edge(edge_idx);
            store.recompute_observed_graph();
            vec![]
        }
        Action::UpdateObservableEdgeWeightFromHeatmap {
            source_idx,
            target_idx,
            new_weight,
        } => {
            if new_weight == 0.0 {
                if let Some(edge_idx) = store
                    .observable_graph
                    .g()
                    .find_edge(source_idx, target_idx)
                {
                    store.observable_graph.remove_edge(edge_idx);
                }
            } else if let Some(edge_idx) = store
                .observable_graph
                .g()
                .find_edge(source_idx, target_idx)
            {
                if let Some(edge) =
                    store.observable_graph.edge_mut(edge_idx)
                {
                    *edge.payload_mut() = new_weight;
                }
            } else {
                if let Some(source_node) =
                    store.observable_graph.node(source_idx)
                {
                    if source_node.payload().node_type
                        == ObservableNodeType::Source
                    {
                        if let Some(target_node) =
                            store.observable_graph.node(target_idx)
                        {
                            if target_node.payload().node_type
                                == ObservableNodeType::Destination
                            {
                                store
                                    .observable_graph
                                    .add_edge_with_label(
                                        source_idx,
                                        target_idx,
                                        new_weight,
                                        String::new(),
                                    );
                            }
                        }
                    }
                }
            }
            store.recompute_observed_graph();
            vec![]
        }

        // UI State Actions
        Action::SetEditMode { mode } => {
            store.prev_mode = store.mode;
            store.mode = mode;
            if store.mode != EditMode::EdgeEditor {
                store.state_graph.set_selected_edges(Vec::new());
                store.observable_graph.set_selected_edges(Vec::new());
            }
            vec![]
        }
        Action::SetActiveTab { tab } => {
            store.active_tab = tab;
            vec![]
        }
        Action::SetShowLabels { show } => {
            store.show_labels = show;
            vec![]
        }
        Action::SetShowWeights { show } => {
            store.show_weights = show;
            vec![]
        }
        Action::ClearStateLayoutResetFlag => {
            store.state_layout_reset_needed = false;
            vec![]
        }
        Action::ClearObservableLayoutResetFlag => {
            store.observable_layout_reset_needed = false;
            vec![]
        }
        Action::ClearObservedLayoutResetFlag => {
            store.observed_layout_reset_needed = false;
            vec![]
        }
        Action::ClearEdgeSelections => {
            store.state_graph.set_selected_edges(Vec::new());
            vec![]
        }
        Action::ClearObservableEdgeSelections => {
            store.observable_graph.set_selected_edges(Vec::new());
            vec![]
        }
        Action::SetDraggingFrom { node_idx, position } => {
            store.dragging_from = match (node_idx, position) {
                (Some(idx), Some(pos)) => Some((idx, pos)),
                _ => None,
            };
            vec![]
        }
        Action::SetDragStarted { started } => {
            store.drag_started = started;
            vec![]
        }
        Action::SetHeatmapHoveredCell { cell } => {
            store.heatmap_hovered_cell = cell;
            vec![]
        }
        Action::SetHeatmapEditingCell { cell } => {
            store.heatmap_editing_cell = cell;
            vec![]
        }
        Action::SetHeatmapEditBuffer { buffer } => {
            store.heatmap_edit_buffer = buffer;
            vec![]
        }

        // File Operations
        Action::SaveToFile { path } => {
            vec![Effect::SaveToFile { path }]
        }
        Action::LoadFromFile { path } => {
            vec![Effect::LoadFromFile { path }]
        }
        Action::ClearErrorMessage => {
            store.error_message = None;
            vec![]
        }
    }
}
