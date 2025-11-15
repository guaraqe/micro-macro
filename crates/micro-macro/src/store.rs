use crate::graph_state::{
    HasName, ObservableNodeType,
    default_observable_graph, default_state_graph,
};
use crate::graph_view;
use crate::graph_view::{
    ObservableGraphDisplay, ObservedGraphDisplay, StateGraphDisplay,
    setup_observable_graph_display, setup_state_graph_display,
};
use crate::heatmap::HeatmapData;
use crate::layout_settings::LayoutSettings;
use crate::node_shapes::VisualParams;
use crate::serialization;
use crate::versioned::Versioned;
use eframe::egui;
use egui_graphs::DisplayNode;
use petgraph::{
    Directed,
    graph::DefaultIx,
    stable_graph::NodeIndex,
    visit::{EdgeRef, IntoEdgeReferences},
};
use std::collections::HashMap;
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
pub struct LayoutReset<K = u64> {
    last_acked: Option<K>,
}

impl<K: PartialEq + Clone> LayoutReset<K> {
    pub fn new() -> Self {
        Self { last_acked: None }
    }

    /// Run the provided function if the external version has changed
    /// Tracks version from Versioned or Memoized objects
    pub fn run_if_layout_changed<F>(
        &mut self,
        current_key: K,
        mut f: F,
    ) where
        F: FnMut(),
    {
        let changed = match &self.last_acked {
            Some(k) => *k != current_key,
            None => true,
        };
        if changed {
            f();
            self.last_acked = Some(current_key);
        }
    }
}

// ============================================================================
// Version Keys - Combine all versioned data for change tracking
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StateVersionKey {
    pub graph: u64,
    pub circular_visuals: u64,
    pub label_visibility: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObservableVersionKey {
    pub graph: u64,
    pub bipartite_visuals: u64,
    pub label_visibility: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObservedVersionKey {
    pub graph: u64,
    pub circular_visuals: u64,
    pub label_visibility: u64,
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

// ============================================================================
// State Graph Store
// ============================================================================

#[derive(Clone)]
pub struct StateGraphStore {
    pub graph: Versioned<StateGraphDisplay>,
    pub circular_visuals: Versioned<VisualParams>,
    pub label_visibility: Versioned<bool>,
    layout_reset: LayoutReset<StateVersionKey>,
}

impl StateGraphStore {
    pub fn new(graph: StateGraphDisplay) -> Self {
        Self {
            graph: Versioned::new(graph),
            circular_visuals: Versioned::new(VisualParams::default()),
            label_visibility: Versioned::new(true),
            layout_reset: LayoutReset::new(),
        }
    }

    pub fn version_key(&self) -> StateVersionKey {
        StateVersionKey {
            graph: self.graph.version(),
            circular_visuals: self.circular_visuals.version(),
            label_visibility: self.label_visibility.version(),
        }
    }

    pub fn run_if_layout_changed<F>(&mut self, f: F)
    where
        F: FnMut(),
    {
        let key = self.version_key();
        self.layout_reset.run_if_layout_changed(key, f);
    }
}

// ============================================================================
// Observable Graph Store
// ============================================================================

#[derive(Clone)]
pub struct ObservableGraphStore {
    pub graph: Versioned<ObservableGraphDisplay>,
    pub bipartite_visuals: Versioned<VisualParams>,
    pub label_visibility: Versioned<bool>,
    layout_reset: LayoutReset<ObservableVersionKey>,
}

impl ObservableGraphStore {
    pub fn new(graph: ObservableGraphDisplay) -> Self {
        Self {
            graph: Versioned::new(graph),
            bipartite_visuals: Versioned::new(VisualParams {
                radius: 5.0,
                label_gap: 8.0,
                label_font: 13.0,
            }),
            label_visibility: Versioned::new(true),
            layout_reset: LayoutReset::new(),
        }
    }

    pub fn version_key(&self) -> ObservableVersionKey {
        ObservableVersionKey {
            graph: self.graph.version(),
            bipartite_visuals: self.bipartite_visuals.version(),
            label_visibility: self.label_visibility.version(),
        }
    }

    pub fn run_if_layout_changed<F>(&mut self, f: F)
    where
        F: FnMut(),
    {
        let key = self.version_key();
        self.layout_reset.run_if_layout_changed(key, f);
    }
}

// ============================================================================
// Observed Graph Store
// ============================================================================

#[derive(Clone)]
pub struct ObservedGraphStore {
    pub circular_visuals: Versioned<VisualParams>,
    pub label_visibility: Versioned<bool>,
    layout_reset: LayoutReset<ObservedVersionKey>,
}

impl ObservedGraphStore {
    pub fn new() -> Self {
        Self {
            circular_visuals: Versioned::new(VisualParams::default()),
            label_visibility: Versioned::new(true),
            layout_reset: LayoutReset::new(),
        }
    }

    /// Get version key combining observed graph version (passed in) with visuals
    pub fn version_key(&self, observed_graph_version: u64) -> ObservedVersionKey {
        ObservedVersionKey {
            graph: observed_graph_version,
            circular_visuals: self.circular_visuals.version(),
            label_visibility: self.label_visibility.version(),
        }
    }

    pub fn run_if_layout_changed<F>(
        &mut self,
        observed_graph_version: u64,
        f: F,
    )
    where
        F: FnMut(),
    {
        let key = self.version_key(observed_graph_version);
        self.layout_reset.run_if_layout_changed(key, f);
    }
}

#[derive(Clone)]
pub struct Store {
    // Graph-specific stores
    pub state: StateGraphStore,
    pub observable: ObservableGraphStore,
    pub observed: ObservedGraphStore,

    // Global UI state (not graph-specific)
    pub mode: EditMode,
    pub prev_mode: EditMode,
    pub active_tab: ActiveTab,
    pub dragging_from: Option<(NodeIndex, egui::Pos2)>,
    pub drag_started: bool,
    pub layout_settings: LayoutSettings,

    // Heatmap editing state
    pub heatmap_hovered_cell: Option<(usize, usize)>,
    pub heatmap_editing_cell: Option<(usize, usize)>,
    pub heatmap_edit_buffer: String,

    // Node editing state
    pub weight_editor: NumberEditor,
    pub label_editor: StringEditor,
    pub observed_node_selection: Option<(NodeIndex, bool)>,

    // Global error state
    pub error_message: Option<String>,
}

impl Store {
    pub fn new(
        state_graph: StateGraphDisplay,
        observable_graph: ObservableGraphDisplay,
        _observed_graph: ObservedGraphDisplay,
        layout_settings: LayoutSettings,
    ) -> Self {
        Self {
            state: StateGraphStore::new(state_graph),
            observable: ObservableGraphStore::new(observable_graph),
            observed: ObservedGraphStore::new(),
            mode: EditMode::NodeEditor,
            prev_mode: EditMode::NodeEditor,
            active_tab: ActiveTab::DynamicalSystem,
            dragging_from: None,
            drag_started: false,
            layout_settings,
            heatmap_hovered_cell: None,
            heatmap_editing_cell: None,
            heatmap_edit_buffer: String::new(),
            weight_editor: NumberEditor::new(),
            label_editor: StringEditor::new(),
            observed_node_selection: None,
            error_message: None,
        }
    }

    // Uncached versions (used internally by cache)
    pub fn state_heatmap_uncached(&self) -> HeatmapData {
        compute_generic_heatmap_data(self.state.graph.get())
    }

    pub fn observable_heatmap_uncached(&self) -> HeatmapData {
        compute_observable_heatmap_data(self.observable.graph.get())
    }

    pub fn observed_heatmap_from_graph(
        &self,
        observed: &graph_view::ObservedGraphDisplay,
    ) -> HeatmapData {
        compute_generic_heatmap_data(observed)
    }

    pub fn state_sorted_weights_uncached(&self) -> Vec<f32> {
        collect_sorted_weights_from_display(self.state.graph.get())
    }

    pub fn observable_sorted_weights_uncached(&self) -> Vec<f32> {
        collect_sorted_weights_from_display(
            self.observable.graph.get(),
        )
    }

    // observed_sorted_weights_uncached removed - now collected directly from cached observed_graph
    // This eliminates redundant recalculation of the entire observed graph

    pub fn state_node_weight_stats(&self) -> Vec<(String, f32)> {
        collect_state_node_weights(self.state.graph.get())
    }
}

fn compute_generic_heatmap_data<N, D>(
    graph: &graph_view::GraphDisplay<N, D>,
) -> HeatmapData
where
    N: Clone + HasName,
    D: DisplayNode<N, f32, Directed, DefaultIx>,
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

fn collect_sorted_weights_from_display<N, D>(
    graph: &graph_view::GraphDisplay<N, D>,
) -> Vec<f32>
where
    N: Clone,
    D: DisplayNode<N, f32, Directed, DefaultIx>,
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

    // Normalize weights to probabilities
    let total: f32 = pairs.iter().map(|(_, w)| w).sum();
    if total > 0.0 {
        for (_, weight) in &mut pairs {
            *weight /= total;
        }
    }

    pairs.sort_by(|a, b| a.0.cmp(&b.0));
    pairs
}

pub fn load_graphs_from_path(
    path: &Path,
) -> Result<
    (StateGraphDisplay, ObservableGraphDisplay, LayoutSettings),
    String,
> {
    let state = serialization::load_from_file(path)?;
    let state_graph =
        serialization::serializable_to_graph(&state.dynamical_system);
    let observable_graph =
        serialization::serializable_to_observable_graph(
            &state.observable,
            &state_graph,
        );

    Ok((
        setup_state_graph_display(&state_graph),
        setup_observable_graph_display(&observable_graph),
        state.layout_settings,
    ))
}

pub fn load_or_create_default_state()
-> (StateGraphDisplay, ObservableGraphDisplay, LayoutSettings) {
    if Path::new(STATE_FILE).exists()
        && let Ok(graphs) =
            load_graphs_from_path(Path::new(STATE_FILE))
    {
        return graphs;
    }

    let state_graph = default_state_graph();
    let observable_graph = default_observable_graph(&state_graph);
    (
        setup_state_graph_display(&state_graph),
        setup_observable_graph_display(&observable_graph),
        LayoutSettings::default(),
    )
}
