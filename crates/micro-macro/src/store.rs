use crate::graph_state::{
    ObservableNode, ObservableNodeType,
    calculate_observed_graph_from_observable_display,
    compute_observed_weights, default_observable_graph,
    default_state_graph,
};
use crate::graph_view::{
    ObservableGraphDisplay, ObservedGraphDisplay, StateGraphDisplay,
    setup_graph_display,
};
use crate::serialization;
use eframe::egui;
use petgraph::stable_graph::NodeIndex;
use salsa::Storage;
use std::collections::{HashMap, HashSet};
use std::path::Path;

const STATE_FILE: &str = "state.json";

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

#[salsa::db]
#[derive(Clone)]
pub struct Store {
    storage: Storage<Self>,
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
    pub observable_layout_reset_needed: bool,
    pub observed_layout_reset_needed: bool,
    pub heatmap_hovered_cell: Option<(usize, usize)>,
    pub heatmap_editing_cell: Option<(usize, usize)>,
    pub heatmap_edit_buffer: String,
    pub error_message: Option<String>,
}

impl Store {
    pub fn new(
        state_graph: StateGraphDisplay,
        observable_graph: ObservableGraphDisplay,
        observed_graph: ObservedGraphDisplay,
    ) -> Self {
        Self {
            storage: Storage::default(),
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
            observable_layout_reset_needed: false,
            observed_layout_reset_needed: true,
            heatmap_hovered_cell: None,
            heatmap_editing_cell: None,
            heatmap_edit_buffer: String::new(),
            error_message: None,
        }
    }

    pub fn save_to_file(&self, path: &Path) -> Result<(), String> {
        let state = serialization::SerializableState {
            dynamical_system: serialization::graph_to_serializable(
                &self.state_graph,
            ),
            observable:
                serialization::observable_graph_to_serializable(
                    &self.observable_graph,
                ),
        };

        serialization::save_to_file(&state, path)
    }

    pub fn load_from_file(
        &mut self,
        path: &Path,
    ) -> Result<(), String> {
        let (state_graph, observable_graph) =
            load_graphs_from_path(path)?;
        self.state_graph = state_graph;
        self.observable_graph = observable_graph;

        self.recompute_observed_graph();
        self.layout_reset_needed = true;
        self.observable_layout_reset_needed = true;
        self.observed_layout_reset_needed = true;
        Ok(())
    }

    pub fn sync_source_nodes(&mut self) {
        let dyn_nodes: Vec<(NodeIndex, String)> = self
            .state_graph
            .nodes_iter()
            .map(|(idx, node)| (idx, node.payload().name.clone()))
            .collect();

        let source_nodes: Vec<(NodeIndex, String)> = self
            .observable_graph
            .nodes_iter()
            .filter(|(_, node)| {
                node.payload().node_type == ObservableNodeType::Source
            })
            .map(|(idx, node)| (idx, node.payload().name.clone()))
            .collect();

        let source_map: HashMap<String, NodeIndex> = source_nodes
            .iter()
            .map(|(idx, name)| (name.clone(), *idx))
            .collect();

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

        let dyn_names: HashSet<String> =
            dyn_nodes.iter().map(|(_, name)| name.clone()).collect();

        for (source_idx, source_name) in source_nodes {
            if !dyn_names.contains(&source_name) {
                self.observable_graph.remove_node(source_idx);
            }
        }

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

    pub fn recompute_observed_graph(&mut self) {
        let observed_graph_raw =
            calculate_observed_graph_from_observable_display(
                &self.observable_graph,
            );
        self.observed_graph =
            setup_graph_display(&observed_graph_raw);

        match compute_observed_weights(
            &self.state_graph,
            &self.observable_graph,
        ) {
            Ok(weights) => {
                let node_updates: Vec<(NodeIndex, NodeIndex, f64)> =
                    self.observed_graph
                        .nodes_iter()
                        .filter_map(|(obs_idx, node)| {
                            let obs_dest_idx =
                                node.payload().observable_node_idx;
                            weights.get(&obs_dest_idx).map(
                                |&weight| {
                                    (obs_idx, obs_dest_idx, weight)
                                },
                            )
                        })
                        .collect();

                for (obs_idx, _, weight) in node_updates {
                    if let Some(node_mut) =
                        self.observed_graph.node_mut(obs_idx)
                    {
                        node_mut.payload_mut().weight = weight as f32;
                    }
                }
            }
            Err(e) => {
                eprintln!("Weight computation error: {}", e);
                let node_indices: Vec<NodeIndex> = self
                    .observed_graph
                    .nodes_iter()
                    .map(|(obs_idx, _)| obs_idx)
                    .collect();

                for obs_idx in node_indices {
                    if let Some(node_mut) =
                        self.observed_graph.node_mut(obs_idx)
                    {
                        node_mut.payload_mut().weight = 0.0;
                    }
                }
            }
        }

        self.observed_layout_reset_needed = true;
    }
}

#[salsa::db]
impl salsa::Database for Store {}

pub fn load_graphs_from_path(
    path: &Path,
) -> Result<(StateGraphDisplay, ObservableGraphDisplay), String> {
    let state = serialization::load_from_file(path)?;
    let state_graph =
        serialization::serializable_to_graph(&state.dynamical_system);
    let observable_graph =
        serialization::serializable_to_observable_graph(
            &state.observable,
            &state_graph,
        );

    Ok((
        setup_graph_display(&state_graph),
        setup_graph_display(&observable_graph),
    ))
}

pub fn load_or_create_default_state()
-> (StateGraphDisplay, ObservableGraphDisplay) {
    if Path::new(STATE_FILE).exists() {
        if let Ok(graphs) =
            load_graphs_from_path(Path::new(STATE_FILE))
        {
            return graphs;
        }
    }

    let state_graph = default_state_graph();
    let observable_graph = default_observable_graph(&state_graph);
    (
        setup_graph_display(&state_graph),
        setup_graph_display(&observable_graph),
    )
}
