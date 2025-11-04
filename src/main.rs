mod layout_circular;
mod layout_bipartite;
mod heatmap;

use eframe::egui;
use egui_graphs::{
    reset_layout, DefaultEdgeShape, DefaultNodeShape, Graph, GraphView,
    SettingsInteraction, SettingsStyle,
};
use layout_circular::{LayoutCircular, LayoutStateCircular, SortOrder, SpacingConfig};
use layout_bipartite::{LayoutBipartite, LayoutStateBipartite};
use petgraph::Directed;
use petgraph::graph::DefaultIx;
use petgraph::stable_graph::{EdgeIndex, NodeIndex, StableGraph};
use petgraph::visit::{EdgeRef, IntoEdgeReferences};
use serde::{Deserialize, Serialize};
use std::path::Path;
// UI Constants
const DRAG_THRESHOLD: f32 = 2.0;
const EDGE_PREVIEW_STROKE_WIDTH: f32 = 2.0;
const EDGE_PREVIEW_COLOR: egui::Color32 =
    egui::Color32::from_rgb(100, 100, 255);

type MyGraphView<'a> = GraphView<
    'a,
    NodeData,
    (),
    Directed,
    DefaultIx,
    DefaultNodeShape,
    DefaultEdgeShape,
    LayoutStateCircular,
    LayoutCircular,
>;

type MappingGraphView<'a> = GraphView<
    'a,
    MappingNodeData,
    (),
    Directed,
    DefaultIx,
    DefaultNodeShape,
    DefaultEdgeShape,
    LayoutStateBipartite,
    LayoutBipartite,
>;

#[derive(Clone)]
struct NodeData {
    name: String,
}

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum NodeType {
    Source,
    Destination,
}

#[derive(Clone)]
pub struct MappingNodeData {
    pub name: String,
    pub node_type: NodeType,
}


// ------------------------------------------------------------------
// Serialization structures
// ------------------------------------------------------------------

#[derive(Serialize, Deserialize)]
struct SerializableNode {
    name: String,
}

#[derive(Serialize, Deserialize)]
struct SerializableEdge {
    source: usize,
    target: usize,
}

#[derive(Serialize, Deserialize)]
struct SerializableGraphState {
    nodes: Vec<SerializableNode>,
    edges: Vec<SerializableEdge>,
}

#[derive(Serialize, Deserialize)]
struct SerializableMappingNode {
    name: String,
    node_type: NodeType,
}

#[derive(Serialize, Deserialize)]
struct SerializableObservableState {
    nodes: Vec<SerializableMappingNode>,
    edges: Vec<SerializableEdge>,
}

#[derive(Serialize, Deserialize)]
struct SerializableState {
    dynamical_system: SerializableGraphState,
    observable: SerializableObservableState,
}

// ------------------------------------------------------------------
// Layout Configuration - Customize circular layout behavior here
// ------------------------------------------------------------------

/// Example: Default configuration (alphabetical sorting, auto-scaling radius)
#[allow(dead_code)]
fn default_layout_config() -> LayoutCircular {
    LayoutCircular::default()
}

/// Example: Reverse alphabetical sorting
#[allow(dead_code)]
fn reverse_alphabetical_config() -> LayoutCircular {
    LayoutCircular::default()
        .with_sort_order(SortOrder::ReverseAlphabetical)
}

/// Example: No sorting (preserve insertion order)
#[allow(dead_code)]
fn no_sort_config() -> LayoutCircular {
    LayoutCircular::default()
        .without_sorting()
}

/// Example: Custom spacing - larger circle
#[allow(dead_code)]
fn large_circle_config() -> LayoutCircular {
    LayoutCircular::default()
        .with_spacing(SpacingConfig::default().with_fixed_radius(300.0))
}

/// Example: Custom spacing - tighter packing
#[allow(dead_code)]
fn tight_packing_config() -> LayoutCircular {
    LayoutCircular::default()
        .with_spacing(
            SpacingConfig::default()
                .with_base_radius(30.0)
                .with_radius_per_node(3.0)
        )
}

/// Example: Combined configuration - reverse sort with large circle
#[allow(dead_code)]
fn combined_config() -> LayoutCircular {
    LayoutCircular::default()
        .with_sort_order(SortOrder::ReverseAlphabetical)
        .with_spacing(SpacingConfig::default().with_fixed_radius(250.0))
}

// ------------------------------------------------------------------
// Initialization helpers
// ------------------------------------------------------------------

fn setup_graph(g: &StableGraph<NodeData, ()>) -> Graph<NodeData> {
    let mut graph: Graph<NodeData, (), Directed, DefaultIx, DefaultNodeShape, DefaultEdgeShape> = Graph::from(g);
    // Set labels and size for all nodes
    for (idx, node) in g.node_indices().zip(g.node_weights())
    {
        if let Some(graph_node) = graph.node_mut(idx) {
            graph_node.set_label(node.name.clone());
            // Reduce node size to 75% of default
            graph_node.display_mut().radius *= 0.75;
        }
    }
    // Clear labels for all edges
    for edge_idx in g.edge_indices() {
        clear_edge_label(&mut graph, edge_idx);
    }
    graph
}

fn setup_mapping_graph(mg: &StableGraph<MappingNodeData, ()>) -> Graph<MappingNodeData> {
    let mut mapping_graph: Graph<MappingNodeData, (), Directed, DefaultIx, DefaultNodeShape, DefaultEdgeShape> = Graph::from(mg);
    // Set labels and size for all nodes
    for (idx, node) in mg.node_indices().zip(mg.node_weights())
    {
        if let Some(graph_node) = mapping_graph.node_mut(idx) {
            graph_node.set_label(node.name.clone());
            graph_node.display_mut().radius *= 0.75;
        }
    }
    // Clear labels for all edges
    for edge_idx in mg.edge_indices() {
        clear_edge_label(&mut mapping_graph, edge_idx);
    }
    mapping_graph
}

fn load_or_create_default_state() -> (Graph<NodeData>, Graph<MappingNodeData>) {
    const STATE_FILE: &str = "state.json";

    if std::path::Path::new(STATE_FILE).exists() {
        // Try to load from state.json
        match std::fs::read_to_string(STATE_FILE) {
            Ok(json_str) => {
                match serde_json::from_str::<SerializableState>(&json_str) {
                    Ok(state) => {
                        // Successfully loaded state
                        let g = serializable_to_graph(&state.dynamical_system);
                        let mg = serializable_to_mapping_graph(&state.observable);
                        return (setup_graph(&g), setup_mapping_graph(&mg));
                    }
                    Err(e) => {
                        eprintln!("Error parsing state.json: {}. Using default state.", e);
                    }
                }
            }
            Err(e) => {
                eprintln!("Error reading state.json: {}. Using default state.", e);
            }
        }
    }

    // Fall back to default state
    let g = generate_graph();
    let mg = generate_mapping_graph(&g);
    (setup_graph(&g), setup_mapping_graph(&mg))
}

// ------------------------------------------------------------------

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "Graph Editor",
        options,
        Box::new(|_cc| {
            let (graph, mapping_graph) = load_or_create_default_state();

            Ok(Box::new(GraphEditor {
                g: graph,
                mapping_g: mapping_graph,
                mode: EditMode::NodeEditor,
                prev_mode: EditMode::NodeEditor,
                active_tab: ActiveTab::DynamicalSystem,
                dragging_from: None,
                drag_started: false,
                show_labels: true,
                layout_reset_needed: false,
                mapping_layout_reset_needed: false,
                heatmap_hovered_cell: None,
                error_message: None,
            }))
        }),
    )
}

fn clear_edge_label<N: Clone>(
    graph: &mut Graph<N>,
    edge_idx: EdgeIndex,
) {
    if let Some(edge) = graph.edge_mut(edge_idx) {
        edge.set_label(String::new());
    }
}

fn set_node_name(
    graph: &mut Graph<NodeData>,
    node_idx: NodeIndex,
    name: String,
) {
    if let Some(node) = graph.node_mut(node_idx) {
        node.payload_mut().name = name.clone();
        node.set_label(name);
    }
}

fn generate_graph() -> StableGraph<NodeData, ()> {
    let mut g = StableGraph::new();

    let a = g.add_node(NodeData {
        name: format!("Node {}", 0),
    });
    let b = g.add_node(NodeData {
        name: format!("Node {}", 1),
    });
    let c = g.add_node(NodeData {
        name: format!("Node {}", 2),
    });

    g.add_edge(a, b, ());
    g.add_edge(b, c, ());
    g.add_edge(c, a, ());

    g
}

fn generate_mapping_graph(source_graph: &StableGraph<NodeData, ()>) -> StableGraph<MappingNodeData, ()> {
    let mut g = StableGraph::new();

    // Add Source nodes mirroring the dynamical system
    for node in source_graph.node_weights() {
        g.add_node(MappingNodeData {
            name: node.name.clone(),
            node_type: NodeType::Source,
        });
    }

    // Add two default Destination nodes
    g.add_node(MappingNodeData {
        name: String::from("Value 0"),
        node_type: NodeType::Destination,
    });
    g.add_node(MappingNodeData {
        name: String::from("Value 1"),
        node_type: NodeType::Destination,
    });

    g
}

// ------------------------------------------------------------------
// Serialization conversion functions
// ------------------------------------------------------------------

fn graph_to_serializable(graph: &Graph<NodeData>) -> SerializableGraphState {
    let stable_graph = graph.g();
    let mut nodes = Vec::new();
    let mut node_index_map = std::collections::HashMap::new();

    // Collect nodes and build index mapping using the nodes_iter from Graph
    for (new_idx, (node_idx, node)) in graph.nodes_iter().enumerate() {
        nodes.push(SerializableNode {
            name: node.payload().name.clone(),
        });
        node_index_map.insert(node_idx, new_idx);
    }

    // Collect edges using petgraph's edge_references
    let mut edges = Vec::new();
    for edge in stable_graph.edge_references() {
        edges.push(SerializableEdge {
            source: node_index_map[&edge.source()],
            target: node_index_map[&edge.target()],
        });
    }

    SerializableGraphState { nodes, edges }
}

fn serializable_to_graph(state: &SerializableGraphState) -> StableGraph<NodeData, ()> {
    let mut g = StableGraph::new();
    let mut node_indices = Vec::new();

    // Add nodes
    for node in &state.nodes {
        let idx = g.add_node(NodeData {
            name: node.name.clone(),
        });
        node_indices.push(idx);
    }

    // Add edges
    for edge in &state.edges {
        g.add_edge(node_indices[edge.source], node_indices[edge.target], ());
    }

    g
}

fn mapping_graph_to_serializable(graph: &Graph<MappingNodeData>) -> SerializableObservableState {
    let stable_graph = graph.g();
    let mut nodes = Vec::new();
    let mut node_index_map = std::collections::HashMap::new();

    // Collect nodes and build index mapping using the nodes_iter from Graph
    for (new_idx, (node_idx, node)) in graph.nodes_iter().enumerate() {
        nodes.push(SerializableMappingNode {
            name: node.payload().name.clone(),
            node_type: node.payload().node_type,
        });
        node_index_map.insert(node_idx, new_idx);
    }

    // Collect edges using petgraph's edge_references
    let mut edges = Vec::new();
    for edge in stable_graph.edge_references() {
        edges.push(SerializableEdge {
            source: node_index_map[&edge.source()],
            target: node_index_map[&edge.target()],
        });
    }

    SerializableObservableState { nodes, edges }
}

fn serializable_to_mapping_graph(state: &SerializableObservableState) -> StableGraph<MappingNodeData, ()> {
    let mut g = StableGraph::new();
    let mut node_indices = Vec::new();

    // Add nodes
    for node in &state.nodes {
        let idx = g.add_node(MappingNodeData {
            name: node.name.clone(),
            node_type: node.node_type,
        });
        node_indices.push(idx);
    }

    // Add edges
    for edge in &state.edges {
        g.add_edge(node_indices[edge.source], node_indices[edge.target], ());
    }

    g
}

// ------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq)]
enum EditMode {
    NodeEditor,
    EdgeEditor,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum ActiveTab {
    DynamicalSystem,
    ObservableEditor,
}

struct GraphEditor {
    g: Graph<NodeData>,
    mapping_g: Graph<MappingNodeData>,
    mode: EditMode,
    prev_mode: EditMode,
    active_tab: ActiveTab,
    dragging_from: Option<(NodeIndex, egui::Pos2)>,
    drag_started: bool,
    show_labels: bool,
    layout_reset_needed: bool,
    mapping_layout_reset_needed: bool,
    heatmap_hovered_cell: Option<(usize, usize)>,
    error_message: Option<String>,
}

impl GraphEditor {
    // Build adjacency matrix and sorted node labels for heatmap
    fn build_heatmap_data(&self) -> (Vec<String>, Vec<String>, Vec<Vec<bool>>) {
        // Get all nodes with their labels
        let mut nodes: Vec<_> = self
            .g
            .nodes_iter()
            .map(|(idx, node)| (idx, node.payload().name.clone()))
            .collect();

        // Sort alphabetically by label
        nodes.sort_by(|a, b| a.1.cmp(&b.1));

        if nodes.is_empty() {
            return (vec![], vec![], vec![]);
        }

        let labels: Vec<String> = nodes.iter().map(|(_, name)| name.clone()).collect();
        let node_count = labels.len();

        // Build index map: NodeIndex -> position in sorted list
        let mut index_map = std::collections::HashMap::new();
        for (pos, (idx, _)) in nodes.iter().enumerate() {
            index_map.insert(*idx, pos);
        }

        // Build adjacency matrix: matrix[y][x] = true if edge from x to y
        let mut matrix = vec![vec![false; node_count]; node_count];

        // Iterate over all edges in the graph
        for (node_idx, _) in nodes.iter() {
            for edge_ref in self.g.edges_directed(*node_idx, petgraph::Direction::Outgoing) {
                let source_idx = edge_ref.source();
                let target_idx = edge_ref.target();

                if let (Some(&x_pos), Some(&y_pos)) =
                    (index_map.get(&source_idx), index_map.get(&target_idx)) {
                    matrix[y_pos][x_pos] = true;
                }
            }
        }

        (labels.clone(), labels, matrix)
    }

    // Returns (incoming_nodes, outgoing_nodes) for a given node
    fn get_node_connections(
        &self,
        node_idx: NodeIndex,
    ) -> (Vec<String>, Vec<String>) {
        let incoming: Vec<String> = self
            .g
            .edges_directed(node_idx, petgraph::Direction::Incoming)
            .map(|edge_ref| {
                let other_idx = edge_ref.source();
                self.g
                    .node(other_idx)
                    .map(|n| n.payload().name.clone())
                    .unwrap_or_else(|| String::from("???"))
            })
            .collect();

        let outgoing: Vec<String> = self
            .g
            .edges_directed(node_idx, petgraph::Direction::Outgoing)
            .map(|edge_ref| {
                let other_idx = edge_ref.target();
                self.g
                    .node(other_idx)
                    .map(|n| n.payload().name.clone())
                    .unwrap_or_else(|| String::from("???"))
            })
            .collect();

        (incoming, outgoing)
    }

    // Returns interaction settings based on current mode
    fn get_settings_interaction(&self) -> SettingsInteraction {
        match self.mode {
            EditMode::NodeEditor => SettingsInteraction::new()
                .with_dragging_enabled(false)
                .with_node_clicking_enabled(true)
                .with_node_selection_enabled(true)
                .with_edge_selection_enabled(true),
            EditMode::EdgeEditor => SettingsInteraction::new()
                .with_dragging_enabled(false)
                .with_edge_clicking_enabled(true)
                .with_edge_selection_enabled(true)
                .with_node_clicking_enabled(true),
        }
    }

    // Returns style settings: controls whether node labels are
    // always visible
    fn get_settings_style(&self) -> SettingsStyle {
        SettingsStyle::new()
            .with_labels_always(self.show_labels)
            .with_node_stroke_hook(|selected, _dragged, _node_color, _current_stroke, _style| {
                if selected {
                    // Elegant blood red for selected nodes
                    egui::Stroke::new(4.0, egui::Color32::from_rgb(180, 50, 60))
                } else {
                    egui::Stroke::new(2.0, egui::Color32::from_rgb(180, 180, 180))
                }
            })
            .with_edge_stroke_hook(|selected, _order, _current_stroke, _style| {
                if selected {
                    egui::Stroke::new(2.0, egui::Color32::from_rgb(120, 120, 120))
                } else {
                    egui::Stroke::new(1.0, egui::Color32::from_rgb(80, 80, 80))
                }
            })
    }

    // Drag-to-create edge workflow: click on source node, drag
    // to target node, release to create edge. Returns arrow
    // coordinates for preview drawing during drag.
    fn handle_edge_creation(
        &mut self,
        pointer: &egui::PointerState,
    ) -> Option<(egui::Pos2, egui::Pos2)> {
        // Start potential drag from a node
        if pointer.primary_pressed()
            && let Some(hovered) = self.g.hovered_node()
                && let Some(press_pos) = pointer.interact_pos() {
                    self.dragging_from = Some((hovered, press_pos));
                    self.drag_started = false;
                }

        // Detect if mouse has moved (drag started)
        if pointer.primary_down() && self.dragging_from.is_some()
            && pointer.delta().length() > DRAG_THRESHOLD {
                self.drag_started = true;
            }

        // Determine if preview arrow should be drawn
        let arrow_coords = if self.drag_started {
            if let Some((_src_idx, from_pos)) = self.dragging_from
            {
                pointer.hover_pos().map(|to_pos| (from_pos, to_pos))
            } else {
                None
            }
        } else {
            None
        };

        // Handle mouse release - create edge if dragged
        if pointer.primary_released() {
            if let Some((source_node, _pos)) = self.dragging_from
                && self.drag_started {
                    // Drag completed - create edge if hovering different node
                    if let Some(target_node) = self.g.hovered_node()
                        && source_node != target_node {
                            let edge_idx = self.g.add_edge(
                                source_node,
                                target_node,
                                (),
                            );
                            // Clear edge label to hide it
                            clear_edge_label(&mut self.g, edge_idx);
                        }
                }
            self.dragging_from = None;
            self.drag_started = false;
        }

        arrow_coords
    }

    // Two-click edge deletion: first click selects, second click
    // deletes. Uses graph library's selection state.
    fn handle_edge_deletion(
        &mut self,
        pointer: &egui::PointerState,
    ) {
        if pointer.primary_clicked() && self.dragging_from.is_none()
        {
            let selected_edges: Vec<_> =
                self.g.selected_edges().to_vec();

            // If exactly one edge is selected and clicked again, delete
            // it
            if selected_edges.len() == 1 {
                let clicked_edge = selected_edges[0];
                self.g.remove_edge(clicked_edge);
            }
            // If no edges or different edge clicked, library handles
            // selection automatically
        }
    }

    // Edge creation for mapping graph with Source->Destination constraint
    fn handle_mapping_edge_creation(
        &mut self,
        pointer: &egui::PointerState,
    ) -> Option<(egui::Pos2, egui::Pos2)> {
        // Start potential drag from a node
        if pointer.primary_pressed()
            && let Some(hovered) = self.mapping_g.hovered_node()
                && let Some(press_pos) = pointer.interact_pos() {
                    self.dragging_from = Some((hovered, press_pos));
                    self.drag_started = false;
                }

        // Detect if mouse has moved (drag started)
        if pointer.primary_down() && self.dragging_from.is_some()
            && pointer.delta().length() > DRAG_THRESHOLD {
                self.drag_started = true;
            }

        // Determine if preview arrow should be drawn
        let arrow_coords = if self.drag_started {
            if let Some((_src_idx, from_pos)) = self.dragging_from
            {
                pointer.hover_pos().map(|to_pos| (from_pos, to_pos))
            } else {
                None
            }
        } else {
            None
        };

        // Handle mouse release - create edge if dragged
        if pointer.primary_released() {
            if let Some((source_node, _pos)) = self.dragging_from
                && self.drag_started {
                    // Drag completed - create edge if hovering different node
                    if let Some(target_node) = self.mapping_g.hovered_node()
                        && source_node != target_node {
                            // Check node types: only allow Source -> Destination
                            let source_type = self.mapping_g.node(source_node)
                                .map(|n| n.payload().node_type);
                            let target_type = self.mapping_g.node(target_node)
                                .map(|n| n.payload().node_type);

                            if let (Some(NodeType::Source), Some(NodeType::Destination)) = (source_type, target_type) {
                                let edge_idx = self.mapping_g.add_edge(
                                    source_node,
                                    target_node,
                                    (),
                                );
                                // Clear edge label to hide it
                                clear_edge_label(&mut self.mapping_g, edge_idx);
                            }
                            // Silently ignore invalid edge attempts (Dest->Source, Source->Source, Dest->Dest)
                        }
                }
            self.dragging_from = None;
            self.drag_started = false;
        }

        arrow_coords
    }

    // Edge deletion for mapping graph
    fn handle_mapping_edge_deletion(
        &mut self,
        pointer: &egui::PointerState,
    ) {
        if pointer.primary_clicked() && self.dragging_from.is_none()
        {
            let selected_edges: Vec<_> =
                self.mapping_g.selected_edges().to_vec();

            if selected_edges.len() == 1 {
                let clicked_edge = selected_edges[0];
                self.mapping_g.remove_edge(clicked_edge);
            }
        }
    }

    fn save_to_file(&self, path: &Path) -> Result<(), String> {
        let state = SerializableState {
            dynamical_system: graph_to_serializable(&self.g),
            observable: mapping_graph_to_serializable(&self.mapping_g),
        };

        let json = serde_json::to_string_pretty(&state)
            .map_err(|e| format!("Failed to serialize state: {}", e))?;

        std::fs::write(path, json)
            .map_err(|e| format!("Failed to write file: {}", e))?;

        Ok(())
    }

    fn load_from_file(&mut self, path: &Path) -> Result<(), String> {
        let json_str = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read file: {}", e))?;

        let state: SerializableState = serde_json::from_str(&json_str)
            .map_err(|e| format!("Failed to parse JSON: {}", e))?;

        // Convert to StableGraph first, then setup with proper display properties
        let g = serializable_to_graph(&state.dynamical_system);
        let mg = serializable_to_mapping_graph(&state.observable);

        self.g = setup_graph(&g);
        self.mapping_g = setup_mapping_graph(&mg);

        // Reset layouts to display new graphs
        self.layout_reset_needed = true;
        self.mapping_layout_reset_needed = true;

        Ok(())
    }
}

impl eframe::App for GraphEditor {
    fn update(
        &mut self,
        ctx: &egui::Context,
        _frame: &mut eframe::Frame,
    ) {
        // Menu bar at the very top
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::MenuBar::new().ui(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Save").clicked() {
                        ui.close();
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("JSON", &["json"])
                            .save_file()
                            && let Err(e) = self.save_to_file(&path)
                        {
                            self.error_message = Some(e);
                        }
                    }

                    if ui.button("Load").clicked() {
                        ui.close();
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("JSON", &["json"])
                            .pick_file()
                            && let Err(e) = self.load_from_file(&path)
                        {
                            self.error_message = Some(e);
                        }
                    }
                });
            });
        });

        // Tab navigation below menu bar
        egui::TopBottomPanel::top("tabs").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.active_tab, ActiveTab::DynamicalSystem, "Dynamical System");
                ui.selectable_value(&mut self.active_tab, ActiveTab::ObservableEditor, "Observable Editor");
            });
        });

        // Detect Ctrl key to switch modes
        let ctrl_pressed = ctx.input(|i| i.modifiers.ctrl);
        self.mode = if ctrl_pressed {
            EditMode::EdgeEditor
        } else {
            EditMode::NodeEditor
        };

        // Clear edge selection state when transitioning from EdgeEditor
        // to NodeEditor. Must happen before GraphView is created.
        if self.prev_mode == EditMode::EdgeEditor
            && self.mode == EditMode::NodeEditor
        {
            self.g.set_selected_edges(Vec::new());
        }

        // Render the appropriate view based on active tab
        match self.active_tab {
            ActiveTab::DynamicalSystem => self.render_dynamical_system_tab(ctx),
            ActiveTab::ObservableEditor => self.render_observable_editor_tab(ctx),
        }

        // Display error dialog if there's an error message
        if let Some(error) = self.error_message.clone() {
            egui::Window::new("Error")
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.label(&error);
                    if ui.button("OK").clicked() {
                        self.error_message = None;
                    }
                });
        }

        // Update previous mode for next frame
        self.prev_mode = self.mode;
    }
}

impl GraphEditor {
    // Build heatmap data for mapping graph: Sources (x-axis), Destinations (y-axis)
    fn build_mapping_heatmap_data(&self) -> (Vec<String>, Vec<String>, Vec<Vec<bool>>) {
        // Get Source nodes (columns/x-axis)
        let mut source_nodes: Vec<_> = self
            .mapping_g
            .nodes_iter()
            .filter(|(_, node)| node.payload().node_type == NodeType::Source)
            .map(|(idx, node)| (idx, node.payload().name.clone()))
            .collect();

        // Get Destination nodes (rows/y-axis)
        let mut dest_nodes: Vec<_> = self
            .mapping_g
            .nodes_iter()
            .filter(|(_, node)| node.payload().node_type == NodeType::Destination)
            .map(|(idx, node)| (idx, node.payload().name.clone()))
            .collect();

        // Sort alphabetically
        source_nodes.sort_by(|a, b| a.1.cmp(&b.1));
        dest_nodes.sort_by(|a, b| a.1.cmp(&b.1));

        if source_nodes.is_empty() || dest_nodes.is_empty() {
            return (vec![], vec![], vec![]);
        }

        let x_labels: Vec<String> = source_nodes.iter().map(|(_, name)| name.clone()).collect();
        let y_labels: Vec<String> = dest_nodes.iter().map(|(_, name)| name.clone()).collect();

        // Build index maps
        let mut source_index_map = std::collections::HashMap::new();
        for (pos, (idx, _)) in source_nodes.iter().enumerate() {
            source_index_map.insert(*idx, pos);
        }

        let mut dest_index_map = std::collections::HashMap::new();
        for (pos, (idx, _)) in dest_nodes.iter().enumerate() {
            dest_index_map.insert(*idx, pos);
        }

        // Build adjacency matrix: matrix[y][x] = true if edge from Source x to Destination y
        let mut matrix = vec![vec![false; source_nodes.len()]; dest_nodes.len()];

        // Iterate over all edges
        for (source_idx, _) in source_nodes.iter() {
            for edge_ref in self.mapping_g.edges_directed(*source_idx, petgraph::Direction::Outgoing) {
                let src = edge_ref.source();
                let tgt = edge_ref.target();

                if let (Some(&x_pos), Some(&y_pos)) =
                    (source_index_map.get(&src), dest_index_map.get(&tgt)) {
                    matrix[y_pos][x_pos] = true;
                }
            }
        }

        (x_labels, y_labels, matrix)
    }

    // Synchronize mapping graph Source nodes with dynamical system nodes
    fn sync_source_nodes(&mut self) {
        // Get current dynamical system nodes
        let dyn_nodes: Vec<(NodeIndex, String)> = self
            .g
            .nodes_iter()
            .map(|(idx, node)| (idx, node.payload().name.clone()))
            .collect();

        // Get current Source nodes in mapping graph
        let source_nodes: Vec<(NodeIndex, String)> = self
            .mapping_g
            .nodes_iter()
            .filter(|(_, node)| node.payload().node_type == NodeType::Source)
            .map(|(idx, node)| (idx, node.payload().name.clone()))
            .collect();

        // Build a map of Source nodes by name for quick lookup
        let source_map: std::collections::HashMap<String, NodeIndex> = source_nodes
            .iter()
            .map(|(idx, name)| (name.clone(), *idx))
            .collect();

        // Add missing Source nodes
        for (_, dyn_name) in &dyn_nodes {
            if !source_map.contains_key(dyn_name) {
                let new_idx = self.mapping_g.add_node(MappingNodeData {
                    name: dyn_name.clone(),
                    node_type: NodeType::Source,
                });
                if let Some(node) = self.mapping_g.node_mut(new_idx) {
                    node.set_label(dyn_name.clone());
                    node.display_mut().radius *= 0.75;
                }
            }
        }

        // Remove Source nodes that no longer exist in dynamical system
        let dyn_names: std::collections::HashSet<String> = dyn_nodes
            .iter()
            .map(|(_, name)| name.clone())
            .collect();

        for (source_idx, source_name) in source_nodes {
            if !dyn_names.contains(&source_name) {
                self.mapping_g.remove_node(source_idx);
            }
        }

        // Update names of Source nodes (in case of renames)
        for (_, dyn_name) in &dyn_nodes {
            if let Some(&source_idx) = source_map.get(dyn_name)
                && let Some(source_node) = self.mapping_g.node_mut(source_idx)
                    && source_node.payload().name != *dyn_name {
                        source_node.payload_mut().name = dyn_name.clone();
                        source_node.set_label(dyn_name.clone());
                    }
        }
    }

    fn render_dynamical_system_tab(&mut self, ctx: &egui::Context) {
        // Calculate exact 1/3 split for all three panels
        let available_width = ctx.available_rect().width();
        let panel_width = available_width / 3.0;

        egui::SidePanel::left("left_panel")
            .exact_width(panel_width)
            .frame(egui::Frame::side_top_panel(&ctx.style()).inner_margin(8.0))
            .show(ctx, |ui| {
            ui.vertical(|ui| {
                // Panel name
                ui.heading("Nodes");
                ui.separator();

                // Controls
                if ui.button("Add Node").clicked() {
                    let node_idx = self.g.add_node(NodeData {
                        name: String::new(),
                    });
                    let default_name =
                        format!("Node {}", node_idx.index());
                    set_node_name(&mut self.g, node_idx, default_name);
                    // Set size to 75% of default
                    if let Some(node) = self.g.node_mut(node_idx) {
                        node.display_mut().radius *= 0.75;
                    }
                    self.layout_reset_needed = true;
                    self.sync_source_nodes();
                }

                // Contents - node list
                let available_height = ui.available_height() - 40.0; // Reserve space for bottom metadata
                egui::ScrollArea::vertical()
                    .max_height(available_height)
                    .show(ui, |ui| {
                    let nodes: Vec<_> = self
                        .g
                        .nodes_iter()
                        .map(|(idx, node)| {
                            (idx, node.payload().name.clone())
                        })
                        .collect();

                    for (node_idx, mut node_name) in nodes {
                        let is_selected = self
                            .g
                            .node(node_idx)
                            .map(|n| n.selected())
                            .unwrap_or(false);

                        ui.horizontal(|ui| {
                            // Collapsible arrow button
                            let arrow = if is_selected { "▼" } else { "▶" };
                            if ui.small_button(arrow).clicked() {
                                // Toggle selection
                                if is_selected {
                                    // Deselect this node
                                    if let Some(node) = self.g.node_mut(node_idx) {
                                        node.set_selected(false);
                                    }
                                } else {
                                    // Deselect all other nodes first
                                    let all_nodes: Vec<_> = self.g.nodes_iter().map(|(idx, _)| idx).collect();
                                    for idx in all_nodes {
                                        if let Some(node) = self.g.node_mut(idx) {
                                            node.set_selected(false);
                                        }
                                    }
                                    // Select this node
                                    if let Some(node) = self.g.node_mut(node_idx) {
                                        node.set_selected(true);
                                    }
                                }
                            }

                            let response =
                                ui.text_edit_singleline(&mut node_name);
                            if response.changed() {
                                set_node_name(
                                    &mut self.g,
                                    node_idx,
                                    node_name,
                                );
                                self.layout_reset_needed = true;
                                self.sync_source_nodes();
                            }
                            if ui.button("🗑").clicked() {
                                self.g.remove_node(node_idx);
                                self.layout_reset_needed = true;
                                self.sync_source_nodes();
                            }
                        });

                        // Only show connection info if this node is selected
                        if is_selected {

                            let (incoming, outgoing) =
                                self.get_node_connections(node_idx);

                            ui.label(format!(
                                "Incoming ({}):",
                                incoming.len()
                            ));
                            if incoming.is_empty() {
                                ui.label("  None");
                            } else {
                                for name in incoming {
                                    ui.label(format!("  ← {}", name));
                                }
                            }

                            ui.label(format!(
                                "Outgoing ({}):",
                                outgoing.len()
                            ));
                            if outgoing.is_empty() {
                                ui.label("  None");
                            } else {
                                for name in outgoing {
                                    ui.label(format!("  → {}", name));
                                }
                            }
                        }
                    }
                });

                // Metadata at bottom
                ui.with_layout(
                    egui::Layout::bottom_up(egui::Align::LEFT),
                    |ui| {
                        ui.label(format!("Nodes: {}", self.g.node_count()));
                        ui.separator();
                    },
                );
            });
        });

        // Right panel for heatmap (1/3 width)
        egui::SidePanel::right("right_panel")
            .exact_width(panel_width)
            .frame(egui::Frame::side_top_panel(&ctx.style()).inner_margin(8.0))
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    // Panel name
                    ui.heading("Heatmap");
                    ui.separator();

                    // Contents - heatmap
                    let available_height = ui.available_height() - 40.0; // Reserve space for bottom metadata
                    ui.allocate_ui_with_layout(
                        egui::Vec2::new(ui.available_width(), available_height),
                        egui::Layout::top_down(egui::Align::Center),
                        |ui| {
                            // Build heatmap data
                            let (x_labels, y_labels, matrix) = self.build_heatmap_data();

                            // Display heatmap and get new hover state
                            self.heatmap_hovered_cell = heatmap::show_heatmap(
                                ui,
                                &x_labels,
                                &y_labels,
                                &matrix,
                                self.heatmap_hovered_cell,
                            );
                        },
                    );

                    // Metadata at bottom
                    ui.with_layout(
                        egui::Layout::bottom_up(egui::Align::LEFT),
                        |ui| {
                            ui.label(format!("Edges: {}", self.g.edge_count()));
                            ui.separator();
                        },
                    );
                });
            });

        egui::CentralPanel::default()
            .frame(egui::Frame::central_panel(&ctx.style()).inner_margin(8.0))
            .show(ctx, |ui| {
            ui.vertical(|ui| {
                // Heading at the top
                ui.heading("Graph");
                ui.separator();

                // Reset layout if needed
                if self.layout_reset_needed {
                    reset_layout::<LayoutStateCircular>(ui, None);
                    self.layout_reset_needed = false;
                }

                // Clear edge selections when not in EdgeEditor mode,
                // before creating GraphView
                if self.mode == EditMode::NodeEditor {
                    self.g.set_selected_edges(Vec::new());
                }

                let settings_interaction = self.get_settings_interaction();
                let settings_style = self.get_settings_style();

                // Allocate remaining space for the graph
                let available_height = ui.available_height() - 60.0; // Reserve space for bottom instructions
                ui.allocate_ui_with_layout(
                    egui::Vec2::new(ui.available_width(), available_height),
                    egui::Layout::top_down(egui::Align::Center),
                    |ui| {
                        ui.add(
                            &mut MyGraphView::new(&mut self.g)
                                .with_interactions(&settings_interaction)
                                .with_styles(&settings_style),
                        );

                        // Edge editing functionality (only in Edge Editor mode)
                        if self.mode == EditMode::EdgeEditor {
                            let pointer = ui.input(|i| i.pointer.clone());

                            // Handle edge creation and draw preview line if needed
                            if let Some((from_pos, to_pos)) =
                                self.handle_edge_creation(&pointer)
                            {
                                ui.painter().line_segment(
                                    [from_pos, to_pos],
                                    egui::Stroke::new(
                                        EDGE_PREVIEW_STROKE_WIDTH,
                                        EDGE_PREVIEW_COLOR,
                                    ),
                                );
                            }

                            self.handle_edge_deletion(&pointer);
                        } else {
                            // Reset dragging state and clear selections when not in Edge Editor mode
                            self.dragging_from = None;
                            self.drag_started = false;
                            self.g.set_selected_edges(Vec::new());
                        }
                    },
                );

                // Controls and metadata at the bottom
                ui.with_layout(
                    egui::Layout::bottom_up(egui::Align::LEFT),
                    |ui| {
                        let (mode_text, hint_text) = match self.mode {
                            EditMode::NodeEditor => (
                                "Mode: Node Editor",
                                "Hold Ctrl for Edge Editor",
                            ),
                            EditMode::EdgeEditor => (
                                "Mode: Edge Editor",
                                "Release Ctrl for Node Editor",
                            ),
                        };
                        ui.label(hint_text);
                        ui.label(mode_text);
                        ui.checkbox(&mut self.show_labels, "Show Labels");
                        ui.separator();
                    },
                );
            });
        });
    }

    fn render_observable_editor_tab(&mut self, ctx: &egui::Context) {
        // Calculate exact 1/3 split for all three panels
        let available_width = ctx.available_rect().width();
        let panel_width = available_width / 3.0;

        // Left panel: Destination node management
        egui::SidePanel::left("observable_left_panel")
            .exact_width(panel_width)
            .frame(egui::Frame::side_top_panel(&ctx.style()).inner_margin(8.0))
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    ui.heading("Observable Values");
                    ui.separator();

                    // Add Destination button
                    if ui.button("Add Value").clicked() {
                        let node_idx = self.mapping_g.add_node(MappingNodeData {
                            name: String::new(),
                            node_type: NodeType::Destination,
                        });
                        let default_name = format!("Value {}", node_idx.index());
                        if let Some(node) = self.mapping_g.node_mut(node_idx) {
                            node.payload_mut().name = default_name.clone();
                            node.set_label(default_name);
                            node.display_mut().radius *= 0.75;
                        }
                    }

                    // Contents - Destination node list
                    let available_height = ui.available_height() - 40.0;
                    egui::ScrollArea::vertical()
                        .max_height(available_height)
                        .show(ui, |ui| {
                            // Collect Destination nodes
                            let dest_nodes: Vec<_> = self
                                .mapping_g
                                .nodes_iter()
                                .filter(|(_, node)| node.payload().node_type == NodeType::Destination)
                                .map(|(idx, node)| (idx, node.payload().name.clone()))
                                .collect();

                            for (node_idx, mut node_name) in dest_nodes {
                                let is_selected = self
                                    .mapping_g
                                    .node(node_idx)
                                    .map(|n| n.selected())
                                    .unwrap_or(false);

                                ui.horizontal(|ui| {
                                    // Collapsible arrow button
                                    let arrow = if is_selected { "▼" } else { "▶" };
                                    if ui.small_button(arrow).clicked() {
                                        // Toggle selection
                                        if is_selected {
                                            if let Some(node) = self.mapping_g.node_mut(node_idx) {
                                                node.set_selected(false);
                                            }
                                        } else {
                                            // Deselect all other nodes first
                                            let all_nodes: Vec<_> = self.mapping_g.nodes_iter().map(|(idx, _)| idx).collect();
                                            for idx in all_nodes {
                                                if let Some(node) = self.mapping_g.node_mut(idx) {
                                                    node.set_selected(false);
                                                }
                                            }
                                            // Select this node
                                            if let Some(node) = self.mapping_g.node_mut(node_idx) {
                                                node.set_selected(true);
                                            }
                                        }
                                    }

                                    let response = ui.text_edit_singleline(&mut node_name);
                                    if response.changed()
                                        && let Some(node) = self.mapping_g.node_mut(node_idx) {
                                            node.payload_mut().name = node_name.clone();
                                            node.set_label(node_name);
                                        }
                                    if ui.button("🗑").clicked() {
                                        self.mapping_g.remove_node(node_idx);
                                    }
                                });

                                // Show incoming Source nodes when selected
                                if is_selected {
                                    let incoming_sources: Vec<String> = self
                                        .mapping_g
                                        .edges_directed(node_idx, petgraph::Direction::Incoming)
                                        .map(|edge_ref| {
                                            let source_idx = edge_ref.source();
                                            self.mapping_g
                                                .node(source_idx)
                                                .map(|n| n.payload().name.clone())
                                                .unwrap_or_else(|| String::from("???"))
                                        })
                                        .collect();

                                    ui.label(format!("Incoming ({}):", incoming_sources.len()));
                                    if incoming_sources.is_empty() {
                                        ui.label("  None");
                                    } else {
                                        for name in incoming_sources {
                                            ui.label(format!("  ← {}", name));
                                        }
                                    }
                                }
                            }
                        });

                    // Metadata at bottom
                    ui.with_layout(
                        egui::Layout::bottom_up(egui::Align::LEFT),
                        |ui| {
                            let dest_count = self
                                .mapping_g
                                .nodes_iter()
                                .filter(|(_, node)| node.payload().node_type == NodeType::Destination)
                                .count();
                            ui.label(format!("Values: {}", dest_count));
                            ui.separator();
                        },
                    );
                });
            });

        // Right panel: Heatmap
        egui::SidePanel::right("observable_right_panel")
            .exact_width(panel_width)
            .frame(egui::Frame::side_top_panel(&ctx.style()).inner_margin(8.0))
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    ui.heading("Mapping Heatmap");
                    ui.separator();

                    // Contents - heatmap
                    let available_height = ui.available_height() - 40.0;
                    ui.allocate_ui_with_layout(
                        egui::Vec2::new(ui.available_width(), available_height),
                        egui::Layout::top_down(egui::Align::Center),
                        |ui| {
                            // Build heatmap data
                            let (x_labels, y_labels, matrix) = self.build_mapping_heatmap_data();

                            // Display heatmap
                            self.heatmap_hovered_cell = heatmap::show_heatmap(
                                ui,
                                &x_labels,
                                &y_labels,
                                &matrix,
                                self.heatmap_hovered_cell,
                            );
                        },
                    );

                    // Metadata at bottom
                    ui.with_layout(
                        egui::Layout::bottom_up(egui::Align::LEFT),
                        |ui| {
                            ui.label(format!("Mappings: {}", self.mapping_g.edge_count()));
                            ui.separator();
                        },
                    );
                });
            });

        // Center panel: Bipartite graph visualization
        egui::CentralPanel::default()
            .frame(egui::Frame::central_panel(&ctx.style()).inner_margin(8.0))
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    ui.heading("Observable Mapping");
                    ui.separator();

                    // Reset layout if needed
                    if self.mapping_layout_reset_needed {
                        reset_layout::<LayoutStateBipartite>(ui, None);
                        self.mapping_layout_reset_needed = false;
                    }

                    // Clear edge selections when not in EdgeEditor mode
                    if self.mode == EditMode::NodeEditor {
                        self.mapping_g.set_selected_edges(Vec::new());
                    }

                    let settings_interaction = self.get_settings_interaction();
                    let settings_style = self.get_settings_style();

                    // Allocate remaining space for the graph
                    let available_height = ui.available_height() - 60.0;
                    ui.allocate_ui_with_layout(
                        egui::Vec2::new(ui.available_width(), available_height),
                        egui::Layout::top_down(egui::Align::Center),
                        |ui| {
                            ui.add(
                                &mut MappingGraphView::new(&mut self.mapping_g)
                                    .with_interactions(&settings_interaction)
                                    .with_styles(&settings_style),
                            );

                            // Edge editing functionality (only in Edge Editor mode)
                            if self.mode == EditMode::EdgeEditor {
                                let pointer = ui.input(|i| i.pointer.clone());

                                // Handle edge creation and draw preview line if needed
                                if let Some((from_pos, to_pos)) =
                                    self.handle_mapping_edge_creation(&pointer)
                                {
                                    ui.painter().line_segment(
                                        [from_pos, to_pos],
                                        egui::Stroke::new(
                                            EDGE_PREVIEW_STROKE_WIDTH,
                                            EDGE_PREVIEW_COLOR,
                                        ),
                                    );
                                }

                                self.handle_mapping_edge_deletion(&pointer);
                            } else {
                                // Reset dragging state and clear selections when not in Edge Editor mode
                                self.dragging_from = None;
                                self.drag_started = false;
                                self.mapping_g.set_selected_edges(Vec::new());
                            }
                        },
                    );

                    // Controls and metadata at the bottom
                    ui.with_layout(
                        egui::Layout::bottom_up(egui::Align::LEFT),
                        |ui| {
                            let (mode_text, hint_text) = match self.mode {
                                EditMode::NodeEditor => (
                                    "Mode: Node Editor",
                                    "Hold Ctrl for Edge Editor",
                                ),
                                EditMode::EdgeEditor => (
                                    "Mode: Edge Editor",
                                    "Release Ctrl for Node Editor",
                                ),
                            };
                            ui.label(hint_text);
                            ui.label(mode_text);
                            ui.checkbox(&mut self.show_labels, "Show Labels");
                            ui.separator();
                        },
                    );
                });
            });
    }
}
