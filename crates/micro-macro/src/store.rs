use crate::graph_state::{ObservableNode, ObservableNodeType, StateNode};
use crate::graph_view::{
    ObservableGraphDisplay, ObservedGraphDisplay, StateGraphDisplay,
    setup_graph_display,
};
use crate::graph_state::calculate_observed_graph_from_observable_display;
use eframe::egui;
use petgraph::stable_graph::NodeIndex;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EditMode {
    NodeEditor,
    EdgeEditor,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ActiveTab {
    DynamicalSystem,
    ObservableEditor,
    ObservedDynamics,
}

/// Actions that can be dispatched to modify the GraphEditor state
#[derive(Debug, Clone)]
pub enum Action {
    /// Add a new node to the state graph
    AddStateNode { name: String, weight: f32 },
    /// Save current project to file
    SaveToFile { path: PathBuf },
    /// Load project from file
    LoadFromFile { path: PathBuf },
}

/// Deferred effects that must run outside the main reducer (e.g., file IO)
#[derive(Debug, Clone)]
pub enum Effect {
    /// Save current project to disk
    SaveToFile { path: PathBuf },
    /// Load a project from disk
    LoadFromFile { path: PathBuf },
}

pub struct GraphEditor {
    pub state_graph: StateGraphDisplay,
    pub observable_graph: ObservableGraphDisplay,
    pub observed_graph: ObservedGraphDisplay,
    pub mode: EditMode,
    pub prev_mode: EditMode,
    pub active_tab: ActiveTab,
    pub dragging_from: Option<(NodeIndex, egui::Pos2)>,
    pub drag_started: bool,
    pub show_labels: bool,
    pub show_weights: bool,
    pub layout_reset_needed: bool,
    pub mapping_layout_reset_needed: bool,
    pub observed_layout_reset_needed: bool,
    pub heatmap_hovered_cell: Option<(usize, usize)>,
    pub heatmap_editing_cell: Option<(usize, usize)>,
    pub heatmap_edit_buffer: String,
    pub error_message: Option<String>,
    /// Queue of actions to be processed
    action_queue: Vec<Action>,
    /// Queue of deferred effects (currently file IO) to be executed
    effect_queue: Vec<Effect>,
}

impl GraphEditor {
    /// Create a new GraphEditor with default state
    pub fn new(
        state_graph: StateGraphDisplay,
        observable_graph: ObservableGraphDisplay,
        observed_graph: ObservedGraphDisplay,
    ) -> Self {
        Self {
            state_graph,
            observable_graph,
            observed_graph,
            mode: EditMode::NodeEditor,
            prev_mode: EditMode::NodeEditor,
            active_tab: ActiveTab::DynamicalSystem,
            dragging_from: None,
            drag_started: false,
            show_labels: true,
            show_weights: false,
            layout_reset_needed: false,
            mapping_layout_reset_needed: false,
            observed_layout_reset_needed: true,
            heatmap_hovered_cell: None,
            heatmap_editing_cell: None,
            heatmap_edit_buffer: String::new(),
            error_message: None,
            action_queue: Vec::new(),
            effect_queue: Vec::new(),
        }
    }

    /// Dispatch an action to be processed later
    pub fn dispatch(&mut self, action: Action) {
        self.action_queue.push(action);
    }

    /// Flush the action queue and apply all pending actions
    pub fn flush_actions(&mut self) {
        let actions = std::mem::take(&mut self.action_queue);
        for action in actions {
            let mut effects = self.apply_action(action);
            self.effect_queue.append(&mut effects);
        }
    }

    /// Apply a single action to modify the state
    fn apply_action(&mut self, action: Action) -> Vec<Effect> {
        match action {
            Action::AddStateNode { name, weight } => {
                let node_idx = self.state_graph.add_node(StateNode {
                    name: name.clone(),
                    weight,
                });
                if let Some(node) =
                    self.state_graph.node_mut(node_idx)
                {
                    node.set_label(name);
                }
                // Directly apply side effects needed for this action
                self.layout_reset_needed = true;
                self.observed_layout_reset_needed = true;
                self.sync_source_nodes();
                self.recompute_observed_graph();
                vec![]
            }
            Action::SaveToFile { path } => {
                vec![Effect::SaveToFile { path }]
            }
            Action::LoadFromFile { path } => {
                vec![Effect::LoadFromFile { path }]
            }
        }
    }

    /// Flush the effect queue and execute all pending effects
    pub fn flush_effects(&mut self) {
        let effects = std::mem::take(&mut self.effect_queue);
        for effect in effects {
            self.run_effect(effect);
        }
    }

    /// Execute a single effect
    fn run_effect(&mut self, effect: Effect) {
        match effect {
            Effect::SaveToFile { path } => {
                if let Err(e) = self.save_to_file(&path) {
                    self.error_message = Some(e);
                }
            }
            Effect::LoadFromFile { path } => {
                if let Err(e) = self.load_from_file(&path) {
                    self.error_message = Some(e);
                }
            }
        }
    }

    /// Synchronize mapping graph Source nodes with dynamical system nodes
    pub fn sync_source_nodes(&mut self) {
        // Get current dynamical system nodes
        let dyn_nodes: Vec<(NodeIndex, String)> = self
            .state_graph
            .nodes_iter()
            .map(|(idx, node)| (idx, node.payload().name.clone()))
            .collect();

        // Get current Source nodes in mapping graph
        let source_nodes: Vec<(NodeIndex, String)> = self
            .observable_graph
            .nodes_iter()
            .filter(|(_, node)| {
                node.payload().node_type == ObservableNodeType::Source
            })
            .map(|(idx, node)| (idx, node.payload().name.clone()))
            .collect();

        // Build a map of Source nodes by name for quick lookup
        let source_map: HashMap<String, NodeIndex> =
            source_nodes
                .iter()
                .map(|(idx, name)| (name.clone(), *idx))
                .collect();

        // Add missing Source nodes
        for (state_idx, dyn_name) in &dyn_nodes {
            if !source_map.contains_key(dyn_name) {
                let new_idx =
                    self.observable_graph.add_node(ObservableNode {
                        name: dyn_name.clone(),
                        node_type: ObservableNodeType::Source,
                        state_node_idx: Some(*state_idx),
                    });
                if let Some(node) =
                    self.observable_graph.node_mut(new_idx)
                {
                    node.set_label(dyn_name.clone());
                }
            }
        }

        // Remove Source nodes that no longer exist in dynamical system
        let dyn_names: HashSet<String> =
            dyn_nodes.iter().map(|(_, name)| name.clone()).collect();

        for (source_idx, source_name) in source_nodes {
            if !dyn_names.contains(&source_name) {
                self.observable_graph.remove_node(source_idx);
            }
        }

        // Update names of Source nodes (in case of renames)
        for (_, dyn_name) in &dyn_nodes {
            if let Some(&source_idx) = source_map.get(dyn_name)
                && let Some(source_node) =
                    self.observable_graph.node_mut(source_idx)
                && source_node.payload().name != *dyn_name
            {
                source_node.payload_mut().name = dyn_name.clone();
                source_node.set_label(dyn_name.clone());
            }
        }
    }

    /// Helper to recompute observed graph from current state
    pub fn recompute_observed_graph(&mut self) {
        let observed_graph_raw =
            calculate_observed_graph_from_observable_display(
                &self.observable_graph,
            );
        self.observed_graph =
            setup_graph_display(&observed_graph_raw);

        // Compute and apply weights
        match crate::graph_state::compute_observed_weights(
            &self.state_graph,
            &self.observable_graph,
        ) {
            Ok(weights) => {
                // Collect indices first to avoid borrow checker issues
                let node_updates: Vec<(NodeIndex, NodeIndex, f64)> = self
                    .observed_graph
                    .nodes_iter()
                    .filter_map(|(obs_idx, node)| {
                        let obs_dest_idx = node.payload().observable_node_idx;
                        weights.get(&obs_dest_idx).map(|&weight| (obs_idx, obs_dest_idx, weight))
                    })
                    .collect();

                // Now apply the updates
                for (obs_idx, _, weight) in node_updates {
                    if let Some(node_mut) = self.observed_graph.node_mut(obs_idx) {
                        node_mut.payload_mut().weight = weight as f32;
                    }
                }
            }
            Err(e) => {
                eprintln!("Weight computation error: {}", e);
                // Collect indices first to avoid borrow checker issues
                let node_indices: Vec<NodeIndex> = self
                    .observed_graph
                    .nodes_iter()
                    .map(|(obs_idx, _)| obs_idx)
                    .collect();

                // Set all weights to 0.0 on error
                for obs_idx in node_indices {
                    if let Some(node_mut) = self.observed_graph.node_mut(obs_idx) {
                        node_mut.payload_mut().weight = 0.0;
                    }
                }
            }
        }

        self.observed_layout_reset_needed = true;
    }
}
