use crate::graph_state::{
    HasName, ObservableNode, ObservableNodeType,
    calculate_observed_graph, default_observable_graph,
    default_state_graph,
};
use crate::graph_view;
use crate::graph_view::{
    ObservableGraphDisplay, ObservedGraphDisplay, StateGraphDisplay,
    setup_graph_display,
};
use crate::heatmap::HeatmapData;
use crate::serialization;
use crate::versioned::Versioned;
use eframe::egui;
use petgraph::stable_graph::NodeIndex;
use petgraph::visit::{EdgeRef, IntoEdgeReferences};
use std::collections::{HashMap, HashSet};
use std::path::Path;

const STATE_FILE: &str = "state.json";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditMode {
    NodeEditor,
    EdgeEditor,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveTab {
    DynamicalSystem,
    ObservableEditor,
    ObservedDynamics,
}

#[derive(Clone)]
/// Tracks when to reset layout based on external version changes
/// (from Versioned or Memoized objects)
pub struct LayoutReset {
    last_acked: u64,
}

impl LayoutReset {
    pub fn new() -> Self {
        Self { last_acked: 0 }
    }

    /// Run the provided function if the external version has changed
    /// Tracks version from Versioned or Memoized objects
    pub fn run_if_version_changed<F>(
        &mut self,
        current_version: u64,
        mut f: F,
    ) where
        F: FnMut(),
    {
        if current_version != self.last_acked {
            f();
            self.last_acked = current_version;
        }
    }
}

#[derive(Clone)]
pub struct NumberEditor {
    node: Option<NodeIndex>,
    value: String,
}

impl NumberEditor {
    pub fn new() -> Self {
        Self {
            node: None,
            value: "".to_string(),
        }
    }

    pub fn node(&mut self) -> Option<NodeIndex> {
        self.node
    }
    pub fn value(&mut self) -> String {
        self.value.clone()
    }

    pub fn focus(&mut self, node: NodeIndex, value: String) {
        self.node = Some(node);
        self.value = value;
    }

    pub fn parse(
        &mut self,
    ) -> Result<f32, std::num::ParseFloatError> {
        self.value.parse::<f32>()
    }
}

#[derive(Clone)]
pub struct StringEditor {
    node: Option<NodeIndex>,
    value: String,
}

impl StringEditor {
    pub fn new() -> Self {
        Self {
            node: None,
            value: "".to_string(),
        }
    }

    pub fn node(&self) -> Option<NodeIndex> {
        self.node
    }

    pub fn value(&self) -> String {
        self.value.clone()
    }

    pub fn focus(&mut self, node: NodeIndex, value: String) {
        self.node = Some(node);
        self.value = value;
    }
}

#[derive(Clone)]
pub struct Store {
    pub state_graph: Versioned<StateGraphDisplay>,
    pub observable_graph: Versioned<ObservableGraphDisplay>,
    pub mode: EditMode,
    pub prev_mode: EditMode,
    pub active_tab: ActiveTab,
    pub dragging_from: Option<(NodeIndex, egui::Pos2)>,
    pub drag_started: bool,
    pub show_labels: bool,
    pub show_weights: bool,
    pub state_layout_reset: LayoutReset,
    pub observable_layout_reset: LayoutReset,
    pub observed_layout_reset: LayoutReset,
    pub observed_graph_dirty: bool,
    pub heatmap_hovered_cell: Option<(usize, usize)>,
    pub heatmap_editing_cell: Option<(usize, usize)>,
    pub heatmap_edit_buffer: String,
    pub weight_editor: NumberEditor,
    pub label_editor: StringEditor,
    pub observed_node_selection: Option<(NodeIndex, bool)>,
    pub error_message: Option<String>,
}

impl Store {
    pub fn new(
        state_graph: StateGraphDisplay,
        observable_graph: ObservableGraphDisplay,
        _observed_graph: ObservedGraphDisplay,
    ) -> Self {
        Self {
            state_graph: Versioned::new(state_graph),
            observable_graph: Versioned::new(observable_graph),
            mode: EditMode::NodeEditor,
            prev_mode: EditMode::NodeEditor,
            active_tab: ActiveTab::DynamicalSystem,
            dragging_from: None,
            drag_started: false,
            show_labels: true,
            show_weights: false,
            state_layout_reset: LayoutReset::new(),
            observable_layout_reset: LayoutReset::new(),
            observed_layout_reset: LayoutReset::new(),
            observed_graph_dirty: false,
            heatmap_hovered_cell: None,
            heatmap_editing_cell: None,
            heatmap_edit_buffer: String::new(),
            weight_editor: NumberEditor::new(),
            label_editor: StringEditor::new(),
            observed_node_selection: None,
            error_message: None,
        }
    }

    pub fn sync_source_nodes(&mut self) {
        let synced = sync_source_nodes_display(
            self.state_graph.get(),
            self.observable_graph.get(),
        );
        self.observable_graph.set(synced);
        // observable_layout_reset will auto-reset via version tracking
        self.mark_observed_graph_dirty();
    }

    pub fn recompute_observed_graph(&mut self) {
        // This is now handled by the cache
        self.observed_graph_dirty = false;
    }

    pub fn mark_observed_graph_dirty(&mut self) {
        self.observed_graph_dirty = true;
    }

    pub fn ensure_observed_graph_fresh(&mut self) {
        if self.observed_graph_dirty {
            self.recompute_observed_graph();
        }
    }

    // mark_all_layouts_dirty() removed - layout resets now automatic via version tracking

    // Uncached versions (used internally by cache)
    pub fn state_heatmap_uncached(&self) -> HeatmapData {
        compute_generic_heatmap_data(self.state_graph.get())
    }

    pub fn observable_heatmap_uncached(&self) -> HeatmapData {
        compute_observable_heatmap_data(self.observable_graph.get())
    }

    pub fn observed_heatmap_from_graph(
        &self,
        observed: &graph_view::ObservedGraphDisplay,
    ) -> HeatmapData {
        compute_generic_heatmap_data(observed)
    }

    pub fn state_sorted_weights_uncached(&self) -> Vec<f32> {
        collect_sorted_weights_from_display(self.state_graph.get())
    }

    pub fn observable_sorted_weights_uncached(&self) -> Vec<f32> {
        collect_sorted_weights_from_display(
            self.observable_graph.get(),
        )
    }

    // observed_sorted_weights_uncached removed - now collected directly from cached observed_graph
    // This eliminates redundant recalculation of the entire observed graph

    pub fn state_selection(&self) -> Vec<usize> {
        self.state_graph
            .get()
            .nodes_iter()
            .filter_map(|(idx, node)| {
                if node.selected() {
                    Some(idx.index())
                } else {
                    None
                }
            })
            .collect()
    }

    pub fn observable_selection(&self) -> Vec<usize> {
        self.observable_graph
            .get()
            .nodes_iter()
            .filter_map(|(idx, node)| {
                if node.selected() {
                    Some(idx.index())
                } else {
                    None
                }
            })
            .collect()
    }

    pub fn state_node_weight_stats(&self) -> Vec<(String, f32)> {
        collect_state_node_weights(self.state_graph.get())
    }

    pub fn observed_node_weight_stats(&self) -> Vec<(String, f32)> {
        let observed = calculate_observed_graph(
            self.state_graph.get(),
            self.observable_graph.get(),
        );
        collect_observed_node_weights(&observed)
    }
}

// Helper functions (converted from tracked queries)

fn sync_source_nodes_display(
    state_graph: &StateGraphDisplay,
    observable_graph: &ObservableGraphDisplay,
) -> ObservableGraphDisplay {
    let mut synced = observable_graph.clone();

    let dyn_nodes: Vec<(NodeIndex, String)> = state_graph
        .nodes_iter()
        .map(|(idx, node)| (idx, node.payload().name.clone()))
        .collect();

    let source_nodes: Vec<(NodeIndex, String)> = synced
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
            let new_idx = synced.add_node(ObservableNode {
                name: dyn_name.clone(),
                node_type: ObservableNodeType::Source,
                state_node_idx: Some(*state_idx),
            });
            if let Some(node) = synced.node_mut(new_idx) {
                node.set_label(dyn_name.clone());
            }
        }
    }

    let dyn_names: HashSet<String> =
        dyn_nodes.iter().map(|(_, name)| name.clone()).collect();

    for (source_idx, source_name) in source_nodes {
        if !dyn_names.contains(&source_name) {
            synced.remove_node(source_idx);
        }
    }

    for (_, dyn_name) in &dyn_nodes {
        if let Some(&source_idx) = source_map.get(dyn_name)
            && let Some(source_node) = synced.node_mut(source_idx)
            && source_node.payload().name != *dyn_name
        {
            source_node.payload_mut().name = dyn_name.clone();
            source_node.set_label(dyn_name.clone());
        }
    }

    synced
}

fn compute_generic_heatmap_data<N>(
    graph: &graph_view::GraphDisplay<N>,
) -> HeatmapData
where
    N: Clone + HasName,
{
    let mut nodes: Vec<_> = graph
        .nodes_iter()
        .map(|(idx, node)| (idx, node.payload().name()))
        .collect();
    nodes.sort_by(|a, b| a.1.cmp(&b.1));
    if nodes.is_empty() {
        return (vec![], vec![], vec![], vec![], vec![]);
    }

    let labels: Vec<String> =
        nodes.iter().map(|(_, name)| name.clone()).collect();
    let node_count = labels.len();

    let mut index_map = HashMap::new();
    for (pos, (idx, _)) in nodes.iter().enumerate() {
        index_map.insert(*idx, pos);
    }

    let node_indices: Vec<NodeIndex> =
        nodes.iter().map(|(idx, _)| *idx).collect();

    let mut matrix = vec![vec![None; node_count]; node_count];
    let stable_g = graph.g();
    for edge_ref in stable_g.edge_references() {
        let source_idx = edge_ref.source();
        let target_idx = edge_ref.target();
        let weight = *edge_ref.weight().payload();
        if let (Some(&source_pos), Some(&target_pos)) =
            (index_map.get(&source_idx), index_map.get(&target_idx))
        {
            matrix[source_pos][target_pos] = Some(weight);
        }
    }

    (
        labels.clone(),
        labels,
        matrix,
        node_indices.clone(),
        node_indices,
    )
}

fn compute_observable_heatmap_data(
    graph: &ObservableGraphDisplay,
) -> HeatmapData {
    let mut source_nodes: Vec<_> = graph
        .nodes_iter()
        .filter(|(_, node)| {
            node.payload().node_type == ObservableNodeType::Source
        })
        .map(|(idx, node)| (idx, node.payload().name.clone()))
        .collect();
    let mut dest_nodes: Vec<_> = graph
        .nodes_iter()
        .filter(|(_, node)| {
            node.payload().node_type
                == ObservableNodeType::Destination
        })
        .map(|(idx, node)| (idx, node.payload().name.clone()))
        .collect();
    source_nodes.sort_by(|a, b| a.1.cmp(&b.1));
    dest_nodes.sort_by(|a, b| a.1.cmp(&b.1));

    if source_nodes.is_empty() || dest_nodes.is_empty() {
        return (vec![], vec![], vec![], vec![], vec![]);
    }

    let x_labels: Vec<String> =
        dest_nodes.iter().map(|(_, name)| name.clone()).collect();
    let y_labels: Vec<String> =
        source_nodes.iter().map(|(_, name)| name.clone()).collect();

    let mut source_index_map = HashMap::new();
    for (y_pos, (idx, _)) in source_nodes.iter().enumerate() {
        source_index_map.insert(*idx, y_pos);
    }

    let mut dest_index_map = HashMap::new();
    for (x_pos, (idx, _)) in dest_nodes.iter().enumerate() {
        dest_index_map.insert(*idx, x_pos);
    }

    let x_node_indices: Vec<NodeIndex> =
        dest_nodes.iter().map(|(idx, _)| *idx).collect();
    let y_node_indices: Vec<NodeIndex> =
        source_nodes.iter().map(|(idx, _)| *idx).collect();

    let mut matrix = vec![vec![None; x_labels.len()]; y_labels.len()];
    let stable_g = graph.g();
    for edge_ref in stable_g.edge_references() {
        let source_idx = edge_ref.source();
        let target_idx = edge_ref.target();
        let weight = *edge_ref.weight().payload();
        if let (Some(&source_row), Some(&dest_col)) = (
            source_index_map.get(&source_idx),
            dest_index_map.get(&target_idx),
        ) {
            matrix[source_row][dest_col] = Some(weight);
        }
    }

    (x_labels, y_labels, matrix, x_node_indices, y_node_indices)
}

fn collect_sorted_weights_from_display<N>(
    graph: &graph_view::GraphDisplay<N>,
) -> Vec<f32>
where
    N: Clone,
{
    let mut weights: Vec<f32> = graph
        .edges_iter()
        .map(|(_, edge)| *edge.payload())
        .collect();
    weights.sort_by(|a, b| {
        a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal)
    });
    weights.insert(0, 0.0);
    weights
}

fn collect_state_node_weights(
    graph: &StateGraphDisplay,
) -> Vec<(String, f32)> {
    let mut pairs: Vec<(String, f32)> = graph
        .nodes_iter()
        .map(|(_, node)| {
            let payload = node.payload();
            (payload.name.clone(), payload.weight)
        })
        .collect();
    pairs.sort_by(|a, b| a.0.cmp(&b.0));
    pairs
}

fn collect_observed_node_weights(
    graph: &ObservedGraphDisplay,
) -> Vec<(String, f32)> {
    let mut pairs: Vec<(String, f32)> = graph
        .nodes_iter()
        .map(|(_, node)| {
            let payload = node.payload();
            (payload.name.clone(), payload.weight)
        })
        .collect();
    pairs.sort_by(|a, b| a.0.cmp(&b.0));
    pairs
}

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
    if Path::new(STATE_FILE).exists()
        && let Ok(graphs) =
            load_graphs_from_path(Path::new(STATE_FILE))
    {
        return graphs;
    }

    let state_graph = default_state_graph();
    let observable_graph = default_observable_graph(&state_graph);
    (
        setup_graph_display(&state_graph),
        setup_graph_display(&observable_graph),
    )
}
