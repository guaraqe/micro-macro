use crate::effects::Effect;
use crate::graph_state::{ObservableNode, ObservableNodeType, StateNode};
use crate::layout_settings::{BipartiteTabLayoutSettings, CircularTabLayoutSettings};
use crate::store::{ActiveTab, EditMode, Store};
use eframe::egui;
use petgraph::stable_graph::{EdgeIndex, NodeIndex};
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub enum LayoutSettingChange {
    NodeRadius(f64),
    LabelGap(f64),
    LabelFontSize(f64),
    ShowLabels(bool),
    EdgeMinWidth(f64),
    EdgeMaxWidth(f64),
    CircularBaseRadius(f64),
    LoopRadius(f64),
    BipartiteLayerGap(f64),
    BipartiteNodeGap(f64),
}

/// Actions that can be dispatched to modify the editor state
#[derive(Debug, Clone)]
pub enum Action {
    // State Graph Node Actions
    /// Add a new node to the state graph
    AddStateNode { name: String, weight: f64 },
    /// Remove a node from the state graph
    RemoveStateNode { node_idx: NodeIndex },
    /// Rename a state graph node
    RenameStateNode {
        node_idx: NodeIndex,
        new_name: String,
    },
    /// Update the weight of a state graph node
    UpdateStateNodeWeightEditor { node_idx: NodeIndex, value: String },
    /// Update the weight of a state graph node
    UpdateStateNodeWeight {
        node_idx: NodeIndex,
        new_weight: f64,
    },
    /// Update the label editor for a state graph node
    UpdateStateNodeLabelEditor { node_idx: NodeIndex, value: String },
    /// Set the selection state of a state graph node
    SelectStateNode { node_idx: NodeIndex, selected: bool },

    // State Graph Edge Actions
    /// Add an edge between two nodes in the state graph
    AddStateEdge {
        source_idx: NodeIndex,
        target_idx: NodeIndex,
        weight: f64,
    },
    /// Remove an edge from the state graph (by edge index)
    RemoveStateEdgeByIndex { edge_idx: EdgeIndex },
    /// Special case for heatmap editing (weight of 0.0 removes edge)
    UpdateStateEdgeWeightFromHeatmap {
        source_idx: NodeIndex,
        target_idx: NodeIndex,
        new_weight: f64,
    },

    // Observable Graph Actions
    /// Add a new Destination node in the observable graph
    AddObservableDestinationNode { name: String },
    /// Remove a Destination node from the observable graph
    RemoveObservableDestinationNode { node_idx: NodeIndex },
    /// Update the label editor for an observable Destination node
    UpdateObservableDestinationNodeLabelEditor { node_idx: NodeIndex, value: String },
    /// Rename an observable Destination node
    RenameObservableDestinationNode {
        node_idx: NodeIndex,
        new_name: String,
    },
    /// Set the selection state of an observable graph node
    SelectObservableNode { node_idx: NodeIndex, selected: bool },
    /// Set the selection state of an observed graph node (cached)
    SelectObservedNode { node_idx: NodeIndex, selected: bool },

    // Observable Edge Actions
    /// Add a observable edge from Source to Destination
    AddObservableEdge {
        source_idx: NodeIndex,
        target_idx: NodeIndex,
        weight: f64,
    },
    /// Remove a observable edge (by edge index)
    RemoveObservableEdgeByIndex { edge_idx: EdgeIndex },
    /// Special case for heatmap editing
    UpdateObservableEdgeWeightFromHeatmap {
        source_idx: NodeIndex,
        target_idx: NodeIndex,
        new_weight: f64,
    },

    // UI State Actions
    /// Change between NodeEditor and EdgeEditor modes
    SetEditMode { mode: EditMode },
    /// Switch between DynamicalSystem, ObservableEditor, and ObservedDynamics tabs
    SetActiveTab { tab: ActiveTab },
    /// Update a layout setting for a tab
    UpdateLayoutSetting {
        tab: ActiveTab,
        change: LayoutSettingChange,
    },
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
            // Add node to state graph
            let node_idx = store.state.graph.get_mut().add_node(StateNode {
                name: name.clone(),
                weight,
            });
            if let Some(node) = store.state.graph.get_mut().node_mut(node_idx) {
                node.set_label(name.clone());
            }

            // Add corresponding Source node to observable graph
            let source_idx = store.observable.graph.get_mut().add_node(ObservableNode {
                name: name.clone(),
                node_type: ObservableNodeType::Source,
                state_node_idx: Some(node_idx),
            });
            if let Some(node) = store.observable.graph.get_mut().node_mut(source_idx) {
                node.set_label(name);
            }

            vec![]
        }
        Action::RemoveStateNode { node_idx } => {
            // Remove node from state graph
            store.state.graph.get_mut().remove_node(node_idx);

            // Find and remove corresponding Source node from observable graph
            let source_node_to_remove =
                store
                    .observable
                    .graph
                    .get()
                    .g()
                    .node_indices()
                    .find(|&idx| {
                        if let Some(node) = store.observable.graph.get().node(idx) {
                            node.payload().node_type == ObservableNodeType::Source
                                && node.payload().state_node_idx == Some(node_idx)
                        } else {
                            false
                        }
                    });

            if let Some(source_idx) = source_node_to_remove {
                store.observable.graph.get_mut().remove_node(source_idx);
            }

            vec![]
        }
        Action::RenameStateNode { node_idx, new_name } => {
            // Rename node in state graph
            if let Some(node) = store.state.graph.get_mut().node_mut(node_idx) {
                node.payload_mut().name = new_name.clone();
                node.set_label(new_name.clone());
            }

            // Find and rename corresponding Source node in observable graph
            let source_node_idx = store
                .observable
                .graph
                .get()
                .g()
                .node_indices()
                .find(|&idx| {
                    if let Some(node) = store.observable.graph.get().node(idx) {
                        node.payload().node_type == ObservableNodeType::Source
                            && node.payload().state_node_idx == Some(node_idx)
                    } else {
                        false
                    }
                });

            if let Some(source_idx) = source_node_idx
                && let Some(node) = store.observable.graph.get_mut().node_mut(source_idx)
            {
                node.payload_mut().name = new_name.clone();
                node.set_label(new_name);
            }

            vec![]
        }
        Action::UpdateStateNodeWeightEditor { node_idx, value } => {
            store.weight_editor.focus(node_idx, value);
            vec![]
        }
        Action::UpdateStateNodeWeight {
            node_idx,
            new_weight,
        } => {
            if let Some(node) = store.state.graph.get_mut().node_mut(node_idx) {
                node.payload_mut().weight = new_weight;
            }
            vec![]
        }
        Action::UpdateStateNodeLabelEditor { node_idx, value } => {
            store.label_editor.focus(node_idx, value);
            vec![]
        }
        Action::SelectStateNode { node_idx, selected } => {
            if selected {
                // Collect all node indices first to avoid borrow conflicts
                let graph = store.state.graph.get_mut();
                let all_indices: Vec<_> = graph.g().node_indices().collect();

                // Deselect all other nodes first
                for idx in all_indices {
                    if idx != node_idx
                        && let Some(node) = graph.node_mut(idx)
                    {
                        node.set_selected(false);
                    }
                }
                // Select the target node
                if let Some(node) = graph.node_mut(node_idx) {
                    node.set_selected(true);
                }
            } else {
                // Just deselect the target node
                if let Some(node) = store.state.graph.get_mut().node_mut(node_idx) {
                    node.set_selected(false);
                }
            }
            vec![]
        }

        // State Graph Edge Actions
        Action::AddStateEdge {
            source_idx,
            target_idx,
            weight,
        } => {
            if store
                .state
                .graph
                .get()
                .g()
                .find_edge(source_idx, target_idx)
                .is_some()
            {
                // Edge already exists, do nothing
                return vec![];
            }
            store.state.graph.get_mut().add_edge_with_label(
                source_idx,
                target_idx,
                weight,
                String::new(),
            );
            vec![]
        }
        Action::RemoveStateEdgeByIndex { edge_idx } => {
            store.state.graph.get_mut().remove_edge(edge_idx);
            vec![]
        }
        Action::UpdateStateEdgeWeightFromHeatmap {
            source_idx,
            target_idx,
            new_weight,
        } => {
            if new_weight == 0.0 {
                if let Some(edge_idx) = store
                    .state
                    .graph
                    .get()
                    .g()
                    .find_edge(source_idx, target_idx)
                {
                    store.state.graph.get_mut().remove_edge(edge_idx);
                }
            } else if let Some(edge_idx) = store
                .state
                .graph
                .get()
                .g()
                .find_edge(source_idx, target_idx)
            {
                if let Some(edge) = store.state.graph.get_mut().edge_mut(edge_idx) {
                    *edge.payload_mut() = new_weight;
                }
            } else {
                store.state.graph.get_mut().add_edge_with_label(
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
            let node_idx = store.observable.graph.get_mut().add_node(ObservableNode {
                name: name.clone(),
                node_type: ObservableNodeType::Destination,
                state_node_idx: None,
            });
            if let Some(node) = store.observable.graph.get_mut().node_mut(node_idx) {
                node.set_label(name);
            }
            vec![]
        }
        Action::RemoveObservableDestinationNode { node_idx } => {
            store.observable.graph.get_mut().remove_node(node_idx);
            vec![]
        }
        Action::UpdateObservableDestinationNodeLabelEditor { node_idx, value } => {
            store.label_editor.focus(node_idx, value);
            vec![]
        }
        Action::RenameObservableDestinationNode { node_idx, new_name } => {
            if let Some(node) = store.observable.graph.get_mut().node_mut(node_idx) {
                node.payload_mut().name = new_name.clone();
                node.set_label(new_name);
            }
            vec![]
        }
        Action::SelectObservableNode { node_idx, selected } => {
            if selected {
                // Collect all node indices first to avoid borrow conflicts
                let graph = store.observable.graph.get_mut();
                let all_indices: Vec<_> = graph.g().node_indices().collect();

                // Deselect all other nodes first
                for idx in all_indices {
                    if idx != node_idx
                        && let Some(node) = graph.node_mut(idx)
                    {
                        node.set_selected(false);
                    }
                }
                // Select the target node
                if let Some(node) = graph.node_mut(node_idx) {
                    node.set_selected(true);
                }
            } else {
                // Just deselect the target node
                if let Some(node) = store.observable.graph.get_mut().node_mut(node_idx) {
                    node.set_selected(false);
                }
            }
            vec![]
        }
        Action::SelectObservedNode { node_idx, selected } => {
            // Store the selection request to be applied to cached graph
            // The cache will handle this through its own mechanism
            store.observed_node_selection = Some((node_idx, selected));
            vec![]
        }

        // Observable Edge Actions
        Action::AddObservableEdge {
            source_idx,
            target_idx,
            weight,
        } => {
            if let Some(source_node) = store.observable.graph.get().node(source_idx)
                && source_node.payload().node_type == ObservableNodeType::Source
                && let Some(target_node) = store.observable.graph.get().node(target_idx)
                && target_node.payload().node_type == ObservableNodeType::Destination
            {
                store.observable.graph.get_mut().add_edge_with_label(
                    source_idx,
                    target_idx,
                    weight,
                    String::new(),
                );
            }
            vec![]
        }
        Action::RemoveObservableEdgeByIndex { edge_idx } => {
            store.observable.graph.get_mut().remove_edge(edge_idx);
            vec![]
        }
        Action::UpdateObservableEdgeWeightFromHeatmap {
            source_idx,
            target_idx,
            new_weight,
        } => {
            if new_weight == 0.0 {
                if let Some(edge_idx) = store
                    .observable
                    .graph
                    .get()
                    .g()
                    .find_edge(source_idx, target_idx)
                {
                    store.observable.graph.get_mut().remove_edge(edge_idx);
                }
            } else if let Some(edge_idx) = store
                .observable
                .graph
                .get()
                .g()
                .find_edge(source_idx, target_idx)
            {
                if let Some(edge) = store.observable.graph.get_mut().edge_mut(edge_idx) {
                    *edge.payload_mut() = new_weight;
                }
            } else if let Some(source_node) = store.observable.graph.get().node(source_idx)
                && source_node.payload().node_type == ObservableNodeType::Source
                && let Some(target_node) = store.observable.graph.get().node(target_idx)
                && target_node.payload().node_type == ObservableNodeType::Destination
            {
                store.observable.graph.get_mut().add_edge_with_label(
                    source_idx,
                    target_idx,
                    new_weight,
                    String::new(),
                );
            }
            vec![]
        }

        // UI State Actions
        Action::SetEditMode { mode } => {
            store.prev_mode = store.mode;
            store.mode = mode;
            if store.mode != EditMode::EdgeEditor {
                store.state.graph.get_mut().set_selected_edges(Vec::new());
                store
                    .observable
                    .graph
                    .get_mut()
                    .set_selected_edges(Vec::new());
            }
            vec![]
        }
        Action::SetActiveTab { tab } => {
            store.active_tab = tab;
            vec![]
        }
        Action::UpdateLayoutSetting { tab, change } => {
            match tab {
                ActiveTab::DynamicalSystem => {
                    apply_circular_setting(&mut store.layout_settings.dynamical_system, change);
                }
                ActiveTab::ObservedDynamics => {
                    apply_circular_setting(&mut store.layout_settings.observed_dynamics, change);
                }
                ActiveTab::ObservableEditor => {
                    apply_bipartite_setting(&mut store.layout_settings.observable_editor, change);
                }
            }
            vec![]
        }
        Action::ClearEdgeSelections => {
            store.state.graph.get_mut().set_selected_edges(Vec::new());
            vec![]
        }
        Action::ClearObservableEdgeSelections => {
            store
                .observable
                .graph
                .get_mut()
                .set_selected_edges(Vec::new());
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

fn apply_circular_setting(settings: &mut CircularTabLayoutSettings, change: LayoutSettingChange) {
    match change {
        LayoutSettingChange::NodeRadius(value) => {
            settings.visuals.node_radius = value;
        }
        LayoutSettingChange::LabelGap(value) => {
            settings.visuals.label_gap = value;
        }
        LayoutSettingChange::LabelFontSize(value) => {
            settings.visuals.label_font_size = value;
        }
        LayoutSettingChange::ShowLabels(value) => {
            settings.visuals.show_labels = value;
        }
        LayoutSettingChange::EdgeMinWidth(value) => {
            settings.edges.min_width = value.min(settings.edges.max_width);
        }
        LayoutSettingChange::EdgeMaxWidth(value) => {
            settings.edges.max_width = value.max(settings.edges.min_width);
        }
        LayoutSettingChange::CircularBaseRadius(value) => {
            settings.layout.base_radius = value;
        }
        LayoutSettingChange::LoopRadius(value) => {
            settings.layout.loop_radius = value.max(0.1);
        }
        _ => {}
    }
}

fn apply_bipartite_setting(settings: &mut BipartiteTabLayoutSettings, change: LayoutSettingChange) {
    match change {
        LayoutSettingChange::NodeRadius(value) => {
            settings.visuals.node_radius = value;
        }
        LayoutSettingChange::LabelGap(value) => {
            settings.visuals.label_gap = value;
        }
        LayoutSettingChange::LabelFontSize(value) => {
            settings.visuals.label_font_size = value;
        }
        LayoutSettingChange::ShowLabels(value) => {
            settings.visuals.show_labels = value;
        }
        LayoutSettingChange::EdgeMinWidth(value) => {
            settings.edges.min_width = value.min(settings.edges.max_width);
        }
        LayoutSettingChange::EdgeMaxWidth(value) => {
            settings.edges.max_width = value.max(settings.edges.min_width);
        }
        LayoutSettingChange::BipartiteLayerGap(value) => {
            settings.layout.layer_gap = value;
        }
        LayoutSettingChange::BipartiteNodeGap(value) => {
            settings.layout.node_gap = value;
        }
        _ => {}
    }
}
