mod graph_state;
mod graph_view;
mod heatmap;
mod layout_bipartite;
mod layout_circular;
mod serialization;

use eframe::egui;
use egui_graphs::{
    DefaultNodeShape, DisplayEdge, DisplayNode, Graph,
    SettingsInteraction, SettingsStyle, reset_layout,
};
use graph_state::{
    ObservableNode, ObservableNodeType, StateNode,
    calculate_observed_graph_from_observable_display,
    default_observable_graph, default_state_graph,
};
use graph_view::{
    ObservableGraphDisplay, ObservableGraphView,
    ObservedGraphDisplay, ObservedGraphView, StateGraphDisplay,
    StateGraphView, WeightedEdgeShape, setup_graph_display,
};
use layout_bipartite::LayoutStateBipartite;
use layout_circular::{
    LayoutCircular, LayoutStateCircular, SortOrder, SpacingConfig,
};
use petgraph::Directed;
use petgraph::graph::DefaultIx;
use petgraph::stable_graph::NodeIndex;
use petgraph::visit::{EdgeRef, IntoEdgeReferences};
use petgraph::{EdgeType, stable_graph::IndexType};

// UI Constants
const DRAG_THRESHOLD: f32 = 2.0;
const EDGE_PREVIEW_STROKE_WIDTH: f32 = 2.0;
const EDGE_PREVIEW_COLOR: egui::Color32 =
    egui::Color32::from_rgb(100, 100, 255);

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
    LayoutCircular::default().without_sorting()
}

/// Example: Custom spacing - larger circle
#[allow(dead_code)]
fn large_circle_config() -> LayoutCircular {
    LayoutCircular::default().with_spacing(
        SpacingConfig::default().with_fixed_radius(300.0),
    )
}

/// Example: Custom spacing - tighter packing
#[allow(dead_code)]
fn tight_packing_config() -> LayoutCircular {
    LayoutCircular::default().with_spacing(
        SpacingConfig::default()
            .with_base_radius(30.0)
            .with_radius_per_node(3.0),
    )
}

/// Example: Combined configuration - reverse sort with large circle
#[allow(dead_code)]
fn combined_config() -> LayoutCircular {
    LayoutCircular::default()
        .with_sort_order(SortOrder::ReverseAlphabetical)
        .with_spacing(
            SpacingConfig::default().with_fixed_radius(250.0),
        )
}

// ------------------------------------------------------------------
// Initialization helpers
// ------------------------------------------------------------------

fn load_graphs_from_path(
    path: &std::path::Path,
) -> Result<(StateGraphDisplay, ObservableGraphDisplay), String> {
    let state = serialization::load_from_file(path)?;

    let g =
        serialization::serializable_to_graph(&state.dynamical_system);
    let mg = serialization::serializable_to_observable_graph(
        &state.observable,
        &g,
    );

    Ok((setup_graph_display(&g), setup_graph_display(&mg)))
}

fn load_or_create_default_state()
-> (StateGraphDisplay, ObservableGraphDisplay) {
    const STATE_FILE: &str = "state.json";

    if std::path::Path::new(STATE_FILE).exists() {
        // Try to load from state.json
        match load_graphs_from_path(std::path::Path::new(STATE_FILE))
        {
            Ok(graphs) => return graphs,
            Err(e) => {
                eprintln!(
                    "Error loading state.json: {}. Using default state.",
                    e
                );
            }
        }
    }

    // Fall back to default state
    let g = default_state_graph();
    let mg = default_observable_graph(&g);
    (setup_graph_display(&g), setup_graph_display(&mg))
}

// ------------------------------------------------------------------

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "Graph Editor",
        options,
        Box::new(|_cc| {
            let (graph, observable_graph) =
                load_or_create_default_state();

            let observed_graph_raw =
                calculate_observed_graph_from_observable_display(
                    &observable_graph,
                );
            let observed_graph =
                setup_graph_display(&observed_graph_raw);

            Ok(Box::new(GraphEditor {
                state_graph: graph,
                observable_graph,
                observed_graph,
                mode: EditMode::NodeEditor,
                prev_mode: EditMode::NodeEditor,
                active_tab: ActiveTab::DynamicalSystem,
                dragging_from: None,
                drag_started: false,
                show_labels: true,
                layout_reset_needed: false,
                mapping_layout_reset_needed: false,
                observed_layout_reset_needed: true,
                heatmap_hovered_cell: None,
                heatmap_editing_cell: None,
                heatmap_edit_buffer: String::new(),
                error_message: None,
            }))
        }),
    )
}

fn set_node_name<
    Ty: EdgeType,
    Ix: IndexType,
    Dn: DisplayNode<StateNode, f32, Ty, Ix>,
    De: DisplayEdge<StateNode, f32, Ty, Ix, Dn>,
>(
    graph: &mut Graph<StateNode, f32, Ty, Ix, Dn, De>,
    node_idx: NodeIndex<Ix>,
    name: String,
) {
    if let Some(node) = graph.node_mut(node_idx) {
        node.payload_mut().name = name.clone();
        node.set_label(name);
    }
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
    ObservedDynamics,
}

struct GraphEditor {
    state_graph: Graph<
        StateNode,
        f32,
        Directed,
        DefaultIx,
        DefaultNodeShape,
        WeightedEdgeShape,
    >,
    observable_graph: Graph<
        ObservableNode,
        f32,
        Directed,
        DefaultIx,
        DefaultNodeShape,
        WeightedEdgeShape,
    >,
    observed_graph: ObservedGraphDisplay,
    mode: EditMode,
    prev_mode: EditMode,
    active_tab: ActiveTab,
    dragging_from: Option<(NodeIndex, egui::Pos2)>,
    drag_started: bool,
    show_labels: bool,
    layout_reset_needed: bool,
    mapping_layout_reset_needed: bool,
    observed_layout_reset_needed: bool,
    heatmap_hovered_cell: Option<(usize, usize)>,
    heatmap_editing_cell: Option<(usize, usize)>,
    heatmap_edit_buffer: String,
    error_message: Option<String>,
}

// Type alias for heatmap data return type
type HeatmapData = (
    Vec<String>,                            // x_labels
    Vec<String>,                            // y_labels
    Vec<Vec<Option<f32>>>,                  // matrix
    Vec<petgraph::stable_graph::NodeIndex>, // x_node_indices
    Vec<petgraph::stable_graph::NodeIndex>, // y_node_indices
);

/// Collect all edge weights from a graph and return them sorted (including duplicates)
/// Always prepends 0.0 to ensure the smallest actual weight doesn't map to minimum thickness
fn collect_sorted_weights<N>(
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

    // Prepend 0.0 to the list so smallest actual weight doesn't get thinnest edge
    weights.insert(0, 0.0);

    weights
}

fn apply_weight_change_to_graph<N>(
    graph: &mut graph_view::GraphDisplay<N>,
    change: heatmap::WeightChange,
) where
    N: Clone,
    N: graph_state::HasName,
{
    let src = change.source_idx;
    let tgt = change.target_idx;

    if change.new_weight == 0.0 {
        // Remove edge
        if let Some(edge_idx) = graph.g().find_edge(src, tgt) {
            graph.remove_edge(edge_idx);
        }
    } else {
        // Add or update edge
        if let Some(edge_idx) = graph.g().find_edge(src, tgt) {
            // Update existing edge weight
            if let Some(edge) = graph.edge_mut(edge_idx) {
                *edge.payload_mut() = change.new_weight;
            }
        } else {
            // Add new edge
            graph.add_edge_with_label(
                src,
                tgt,
                change.new_weight,
                String::new(),
            );
        }
    }
}

fn build_heatmap_data<N>(
    graph: &graph_view::GraphDisplay<N>,
) -> HeatmapData
where
    N: Clone,
    N: graph_state::HasName,
{
    // Get all nodes with their labels
    let mut nodes: Vec<_> = graph
        .nodes_iter()
        .map(|(idx, node)| (idx, node.payload().name()))
        .collect();

    // Sort alphabetically by label
    nodes.sort_by(|a, b| a.1.cmp(&b.1));

    if nodes.is_empty() {
        return (vec![], vec![], vec![], vec![], vec![]);
    }

    let labels: Vec<String> =
        nodes.iter().map(|(_, name)| name.clone()).collect();
    let node_count = labels.len();

    // Build index map: NodeIndex -> position in sorted list
    let mut index_map = std::collections::HashMap::new();
    for (pos, (idx, _)) in nodes.iter().enumerate() {
        index_map.insert(*idx, pos);
    }

    // Build node_indices array: position -> NodeIndex
    let node_indices: Vec<petgraph::stable_graph::NodeIndex> =
        nodes.iter().map(|(idx, _)| *idx).collect();

    // Build adjacency matrix: matrix[y][x] = Some(weight) if edge from y to x
    // Sources (y-axis/rows), Targets (x-axis/columns)
    let mut matrix = vec![vec![None; node_count]; node_count];

    // Iterate over all edges in the graph
    let stable_g = graph.g();
    for edge_ref in stable_g.edge_references() {
        let source_idx = edge_ref.source();
        let target_idx = edge_ref.target();
        let weight = *edge_ref.weight().payload();

        if let (Some(&source_pos), Some(&target_pos)) =
            (index_map.get(&source_idx), index_map.get(&target_idx))
        {
            // matrix[source_row][target_col] = weight
            matrix[source_pos][target_pos] = Some(weight);
        }
    }

    // x_labels = targets (columns), y_labels = sources (rows)
    // x_node_indices = targets, y_node_indices = sources
    (
        labels.clone(),
        labels,
        matrix,
        node_indices.clone(),
        node_indices,
    )
}

fn build_observable_heatmap_data(
    graph: &ObservableGraphDisplay,
) -> HeatmapData {
    // Get source nodes (y-axis/rows) and destination nodes (x-axis/columns)
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

    // Sort alphabetically
    source_nodes.sort_by(|a, b| a.1.cmp(&b.1));
    dest_nodes.sort_by(|a, b| a.1.cmp(&b.1));

    if source_nodes.is_empty() || dest_nodes.is_empty() {
        return (vec![], vec![], vec![], vec![], vec![]);
    }

    // SWAPPED: x_labels = destinations (columns), y_labels = sources (rows)
    let x_labels: Vec<String> =
        dest_nodes.iter().map(|(_, name)| name.clone()).collect();
    let y_labels: Vec<String> =
        source_nodes.iter().map(|(_, name)| name.clone()).collect();

    // Build index maps
    let mut source_index_map = std::collections::HashMap::new();
    for (y_pos, (idx, _)) in source_nodes.iter().enumerate() {
        source_index_map.insert(*idx, y_pos);
    }

    let mut dest_index_map = std::collections::HashMap::new();
    for (x_pos, (idx, _)) in dest_nodes.iter().enumerate() {
        dest_index_map.insert(*idx, x_pos);
    }

    // SWAPPED: x_node_indices = destinations, y_node_indices = sources
    let x_node_indices: Vec<petgraph::stable_graph::NodeIndex> =
        dest_nodes.iter().map(|(idx, _)| *idx).collect();
    let y_node_indices: Vec<petgraph::stable_graph::NodeIndex> =
        source_nodes.iter().map(|(idx, _)| *idx).collect();

    // Build adjacency matrix: matrix[source_row][dest_col] = Some(weight)
    let mut matrix = vec![vec![None; x_labels.len()]; y_labels.len()];

    // Iterate over all edges in the graph
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

impl GraphEditor {
    // Helper to recompute observed graph from current state
    fn recompute_observed_graph(&mut self) {
        let observed_graph_raw =
            calculate_observed_graph_from_observable_display(
                &self.observable_graph,
            );
        self.observed_graph =
            setup_graph_display(&observed_graph_raw);
        self.observed_layout_reset_needed = true;
    }

    // Returns (incoming_nodes, outgoing_nodes) for a given node
    fn get_node_connections(
        &self,
        node_idx: NodeIndex,
    ) -> (Vec<String>, Vec<String>) {
        let incoming: Vec<String> = self
            .state_graph
            .edges_directed(node_idx, petgraph::Direction::Incoming)
            .map(|edge_ref| {
                let other_idx = edge_ref.source();
                self.state_graph
                    .node(other_idx)
                    .map(|n| n.payload().name.clone())
                    .unwrap_or_else(|| String::from("???"))
            })
            .collect();

        let outgoing: Vec<String> = self
            .state_graph
            .edges_directed(node_idx, petgraph::Direction::Outgoing)
            .map(|edge_ref| {
                let other_idx = edge_ref.target();
                self.state_graph
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
            .with_node_stroke_hook(
                |selected,
                 _dragged,
                 _node_color,
                 _current_stroke,
                 _style| {
                    if selected {
                        // Elegant blood red for selected nodes
                        egui::Stroke::new(
                            4.0,
                            egui::Color32::from_rgb(180, 50, 60),
                        )
                    } else {
                        egui::Stroke::new(
                            2.0,
                            egui::Color32::from_rgb(180, 180, 180),
                        )
                    }
                },
            )
            .with_edge_stroke_hook(
                |selected, _order, current_stroke, _style| {
                    // Use the width from current_stroke (which comes from WeightedEdgeShape)
                    // but change color based on selection
                    if selected {
                        egui::Stroke::new(
                            current_stroke.width,
                            egui::Color32::from_rgb(120, 120, 120),
                        )
                    } else {
                        egui::Stroke::new(
                            current_stroke.width,
                            egui::Color32::from_rgb(80, 80, 80),
                        )
                    }
                },
            )
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
            && let Some(hovered) = self.state_graph.hovered_node()
            && let Some(press_pos) = pointer.interact_pos()
        {
            self.dragging_from = Some((hovered, press_pos));
            self.drag_started = false;
        }

        // Detect if mouse has moved (drag started)
        if pointer.primary_down()
            && self.dragging_from.is_some()
            && pointer.delta().length() > DRAG_THRESHOLD
        {
            self.drag_started = true;
        }

        // Determine if preview arrow should be drawn
        let arrow_coords = if self.drag_started {
            if let Some((_src_idx, from_pos)) = self.dragging_from {
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
                && self.drag_started
            {
                // Drag completed - create edge if hovering different node
                if let Some(target_node) =
                    self.state_graph.hovered_node()
                    && source_node != target_node
                {
                    self.state_graph.add_edge_with_label(
                        source_node,
                        target_node,
                        1.0,
                        String::new(),
                    );
                }
            }
            self.dragging_from = None;
            self.drag_started = false;
        }

        arrow_coords
    }

    // Two-click edge deletion: first click selects, second click
    // deletes. Uses graph library's selection state.
    fn handle_edge_deletion(&mut self, pointer: &egui::PointerState) {
        if pointer.primary_clicked() && self.dragging_from.is_none() {
            let selected_edges: Vec<_> =
                self.state_graph.selected_edges().to_vec();

            // If exactly one edge is selected and clicked again, delete
            // it
            if selected_edges.len() == 1 {
                let clicked_edge = selected_edges[0];
                self.state_graph.remove_edge(clicked_edge);
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
            && let Some(hovered) =
                self.observable_graph.hovered_node()
            && let Some(press_pos) = pointer.interact_pos()
        {
            self.dragging_from = Some((hovered, press_pos));
            self.drag_started = false;
        }

        // Detect if mouse has moved (drag started)
        if pointer.primary_down()
            && self.dragging_from.is_some()
            && pointer.delta().length() > DRAG_THRESHOLD
        {
            self.drag_started = true;
        }

        // Determine if preview arrow should be drawn
        let arrow_coords = if self.drag_started {
            if let Some((_src_idx, from_pos)) = self.dragging_from {
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
                && self.drag_started
            {
                // Drag completed - create edge if hovering different node
                if let Some(target_node) =
                    self.observable_graph.hovered_node()
                    && source_node != target_node
                {
                    // Check node types: only allow Source -> Destination
                    let source_type = self
                        .observable_graph
                        .node(source_node)
                        .map(|n| n.payload().node_type);
                    let target_type = self
                        .observable_graph
                        .node(target_node)
                        .map(|n| n.payload().node_type);

                    if let (
                        Some(ObservableNodeType::Source),
                        Some(ObservableNodeType::Destination),
                    ) = (source_type, target_type)
                    {
                        self.observable_graph.add_edge_with_label(
                            source_node,
                            target_node,
                            1.0,
                            String::new(),
                        );
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
        if pointer.primary_clicked() && self.dragging_from.is_none() {
            let selected_edges: Vec<_> =
                self.observable_graph.selected_edges().to_vec();

            if selected_edges.len() == 1 {
                let clicked_edge = selected_edges[0];
                self.observable_graph.remove_edge(clicked_edge);
            }
        }
    }

    fn save_to_file(
        &self,
        path: &std::path::Path,
    ) -> Result<(), String> {
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

    fn load_from_file(
        &mut self,
        path: &std::path::Path,
    ) -> Result<(), String> {
        let (g, mg) = load_graphs_from_path(path)?;

        self.state_graph = g;
        self.observable_graph = mg;
        let observed_graph_raw =
            calculate_observed_graph_from_observable_display(
                &self.observable_graph,
            );
        self.observed_graph =
            setup_graph_display(&observed_graph_raw);

        // Reset layouts to display new graphs
        self.layout_reset_needed = true;
        self.mapping_layout_reset_needed = true;
        self.observed_layout_reset_needed = true;

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
                ui.selectable_value(
                    &mut self.active_tab,
                    ActiveTab::DynamicalSystem,
                    "Dynamical System",
                );
                ui.selectable_value(
                    &mut self.active_tab,
                    ActiveTab::ObservableEditor,
                    "Observable Editor",
                );
                ui.selectable_value(
                    &mut self.active_tab,
                    ActiveTab::ObservedDynamics,
                    "Observed Dynamics",
                );
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
            self.state_graph.set_selected_edges(Vec::new());
        }

        // Render the appropriate view based on active tab
        match self.active_tab {
            ActiveTab::DynamicalSystem => {
                self.render_dynamical_system_tab(ctx)
            }
            ActiveTab::ObservableEditor => {
                self.render_observable_editor_tab(ctx)
            }
            ActiveTab::ObservedDynamics => {
                self.render_observed_dynamics_tab(ctx)
            }
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
    // Synchronize mapping graph Source nodes with dynamical system nodes
    fn sync_source_nodes(&mut self) {
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
        let source_map: std::collections::HashMap<String, NodeIndex> =
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
        let dyn_names: std::collections::HashSet<String> =
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
                    let node_idx = self.state_graph.add_node(StateNode {
                        name: String::new(),
                    });
                    let default_name =
                        format!("Node {}", node_idx.index());
                    set_node_name(&mut self.state_graph, node_idx, default_name);
                    self.layout_reset_needed = true;
                    self.sync_source_nodes();
                    self.recompute_observed_graph();
                }

                // Contents - node list
                let available_height = ui.available_height() - 40.0; // Reserve space for bottom metadata
                egui::ScrollArea::vertical()
                    .max_height(available_height)
                    .show(ui, |ui| {
                    let nodes: Vec<_> = self
                        .state_graph
                        .nodes_iter()
                        .map(|(idx, node)| {
                            (idx, node.payload().name.clone())
                        })
                        .collect();

                    for (node_idx, mut node_name) in nodes {
                        let is_selected = self
                            .state_graph
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
                                    if let Some(node) = self.state_graph.node_mut(node_idx) {
                                        node.set_selected(false);
                                    }
                                } else {
                                    // Deselect all other nodes first
                                    let all_nodes: Vec<_> = self.state_graph.nodes_iter().map(|(idx, _)| idx).collect();
                                    for idx in all_nodes {
                                        if let Some(node) = self.state_graph.node_mut(idx) {
                                            node.set_selected(false);
                                        }
                                    }
                                    // Select this node
                                    if let Some(node) = self.state_graph.node_mut(node_idx) {
                                        node.set_selected(true);
                                    }
                                }
                            }

                            let response =
                                ui.text_edit_singleline(&mut node_name);
                            if response.changed() {
                                set_node_name(
                                    &mut self.state_graph,
                                    node_idx,
                                    node_name,
                                );
                                self.layout_reset_needed = true;
                                self.sync_source_nodes();
                                self.recompute_observed_graph();
                            }
                            if ui.button("🗑").clicked() {
                                self.state_graph.remove_node(node_idx);
                                self.layout_reset_needed = true;
                                self.sync_source_nodes();
                                self.recompute_observed_graph();
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
                        ui.label(format!("Nodes: {}", self.state_graph.node_count()));
                        ui.separator();
                    },
                );
            });
        });

        // Right panel for heatmap (1/3 width)
        egui::SidePanel::right("right_panel")
            .exact_width(panel_width)
            .frame(
                egui::Frame::side_top_panel(&ctx.style())
                    .inner_margin(8.0),
            )
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    // Panel name
                    ui.heading("Heatmap");
                    ui.separator();

                    // Contents - heatmap
                    let available_height =
                        ui.available_height() - 40.0; // Reserve space for bottom metadata
                    ui.allocate_ui_with_layout(
                        egui::Vec2::new(
                            ui.available_width(),
                            available_height,
                        ),
                        egui::Layout::top_down(egui::Align::Center),
                        |ui| {
                            // Build heatmap data
                            let (
                                x_labels,
                                y_labels,
                                matrix,
                                x_node_indices,
                                y_node_indices,
                            ) = build_heatmap_data(&self.state_graph);

                            // Display heatmap with editing support
                            let editing_state =
                                heatmap::EditingState {
                                    editing_cell: self
                                        .heatmap_editing_cell,
                                    edit_buffer: self
                                        .heatmap_edit_buffer
                                        .clone(),
                                };

                            let (
                                new_hover,
                                new_editing,
                                weight_change,
                            ) = heatmap::show_heatmap(
                                ui,
                                &x_labels,
                                &y_labels,
                                &matrix,
                                &x_node_indices,
                                &y_node_indices,
                                self.heatmap_hovered_cell,
                                editing_state,
                            );

                            self.heatmap_hovered_cell = new_hover;
                            self.heatmap_editing_cell =
                                new_editing.editing_cell;
                            self.heatmap_edit_buffer =
                                new_editing.edit_buffer;

                            // Handle weight changes
                            if let Some(change) = weight_change {
                                apply_weight_change_to_graph(
                                    &mut self.state_graph,
                                    change,
                                );
                            }
                        },
                    );

                    // Metadata at bottom
                    ui.with_layout(
                        egui::Layout::bottom_up(egui::Align::LEFT),
                        |ui| {
                            ui.label(format!(
                                "Edges: {}",
                                self.state_graph.edge_count()
                            ));
                            ui.separator();
                        },
                    );
                });
            });

        egui::CentralPanel::default()
            .frame(
                egui::Frame::central_panel(&ctx.style())
                    .inner_margin(8.0),
            )
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
                        self.state_graph
                            .set_selected_edges(Vec::new());
                    }

                    // Update edge thicknesses based on global weight distribution
                    let sorted_weights =
                        collect_sorted_weights(&self.state_graph);
                    graph_view::update_edge_thicknesses(
                        &mut self.state_graph,
                        sorted_weights,
                    );

                    let settings_interaction =
                        self.get_settings_interaction();
                    let settings_style = self.get_settings_style();

                    // Allocate remaining space for the graph
                    let available_height =
                        ui.available_height() - 60.0; // Reserve space for bottom instructions

                    ui.allocate_ui_with_layout(
                        egui::Vec2::new(
                            ui.available_width(),
                            available_height,
                        ),
                        egui::Layout::top_down(egui::Align::Center),
                        |ui| {
                            ui.add(
                                &mut StateGraphView::new(
                                    &mut self.state_graph,
                                )
                                .with_interactions(
                                    &settings_interaction,
                                )
                                .with_styles(&settings_style),
                            );

                            // Edge editing functionality (only in Edge Editor mode)
                            if self.mode == EditMode::EdgeEditor {
                                let pointer =
                                    ui.input(|i| i.pointer.clone());

                                // Handle edge creation and draw preview line if needed
                                if let Some((from_pos, to_pos)) = self
                                    .handle_edge_creation(&pointer)
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
                                self.state_graph
                                    .set_selected_edges(Vec::new());
                            }
                        },
                    );

                    // Controls and metadata at the bottom
                    ui.with_layout(
                        egui::Layout::bottom_up(egui::Align::LEFT),
                        |ui| {
                            let (mode_text, hint_text) = match self
                                .mode
                            {
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
                            ui.checkbox(
                                &mut self.show_labels,
                                "Show Labels",
                            );
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
                        let node_idx = self.observable_graph.add_node(ObservableNode {
                            name: String::new(),
                            node_type: ObservableNodeType::Destination,
                            state_node_idx: None,
                        });
                        let default_name = format!("Value {}", node_idx.index());
                        if let Some(node) = self.observable_graph.node_mut(node_idx) {
                            node.payload_mut().name = default_name.clone();
                            node.set_label(default_name);
                        }
                        self.mapping_layout_reset_needed = true;
                        self.recompute_observed_graph();
                    }

                    // Contents - Destination node list
                    let available_height = ui.available_height() - 40.0;
                    egui::ScrollArea::vertical()
                        .max_height(available_height)
                        .show(ui, |ui| {
                            // Collect Destination nodes
                            let dest_nodes: Vec<_> = self
                                .observable_graph
                                .nodes_iter()
                                .filter(|(_, node)| node.payload().node_type == ObservableNodeType::Destination)
                                .map(|(idx, node)| (idx, node.payload().name.clone()))
                                .collect();

                            for (node_idx, mut node_name) in dest_nodes {
                                let is_selected = self
                                    .observable_graph
                                    .node(node_idx)
                                    .map(|n| n.selected())
                                    .unwrap_or(false);

                                ui.horizontal(|ui| {
                                    // Collapsible arrow button
                                    let arrow = if is_selected { "▼" } else { "▶" };
                                    if ui.small_button(arrow).clicked() {
                                        // Toggle selection
                                        if is_selected {
                                            if let Some(node) = self.observable_graph.node_mut(node_idx) {
                                                node.set_selected(false);
                                            }
                                        } else {
                                            // Deselect all other nodes first
                                            let all_nodes: Vec<_> = self.observable_graph.nodes_iter().map(|(idx, _)| idx).collect();
                                            for idx in all_nodes {
                                                if let Some(node) = self.observable_graph.node_mut(idx) {
                                                    node.set_selected(false);
                                                }
                                            }
                                            // Select this node
                                            if let Some(node) = self.observable_graph.node_mut(node_idx) {
                                                node.set_selected(true);
                                            }
                                        }
                                    }

                                    let response = ui.text_edit_singleline(&mut node_name);
                                    if response.changed()
                                        && let Some(node) = self.observable_graph.node_mut(node_idx) {
                                            node.payload_mut().name = node_name.clone();
                                            node.set_label(node_name);
                                            self.recompute_observed_graph();
                                        }
                                    if ui.button("🗑").clicked() {
                                        self.observable_graph.remove_node(node_idx);
                                        self.mapping_layout_reset_needed = true;
                                        self.recompute_observed_graph();
                                    }
                                });

                                // Show incoming Source nodes when selected
                                if is_selected {
                                    let incoming_sources: Vec<String> = self
                                        .observable_graph
                                        .edges_directed(node_idx, petgraph::Direction::Incoming)
                                        .map(|edge_ref| {
                                            let source_idx = edge_ref.source();
                                            self.observable_graph
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
                                .observable_graph
                                .nodes_iter()
                                .filter(|(_, node)| node.payload().node_type == ObservableNodeType::Destination)
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
            .frame(
                egui::Frame::side_top_panel(&ctx.style())
                    .inner_margin(8.0),
            )
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    ui.heading("Mapping Heatmap");
                    ui.separator();

                    // Contents - heatmap
                    let available_height =
                        ui.available_height() - 40.0;
                    ui.allocate_ui_with_layout(
                        egui::Vec2::new(
                            ui.available_width(),
                            available_height,
                        ),
                        egui::Layout::top_down(egui::Align::Center),
                        |ui| {
                            // Build heatmap data for observable graph (sources as x-axis, destinations as y-axis)
                            let (
                                x_labels,
                                y_labels,
                                matrix,
                                x_node_indices,
                                y_node_indices,
                            ) = build_observable_heatmap_data(
                                &self.observable_graph,
                            );

                            // Display heatmap with editing support
                            let editing_state =
                                heatmap::EditingState {
                                    editing_cell: self
                                        .heatmap_editing_cell,
                                    edit_buffer: self
                                        .heatmap_edit_buffer
                                        .clone(),
                                };

                            let (
                                new_hover,
                                new_editing,
                                weight_change,
                            ) = heatmap::show_heatmap(
                                ui,
                                &x_labels,
                                &y_labels,
                                &matrix,
                                &x_node_indices,
                                &y_node_indices,
                                self.heatmap_hovered_cell,
                                editing_state,
                            );

                            self.heatmap_hovered_cell = new_hover;
                            self.heatmap_editing_cell =
                                new_editing.editing_cell;
                            self.heatmap_edit_buffer =
                                new_editing.edit_buffer;

                            // Handle weight changes
                            if let Some(change) = weight_change {
                                apply_weight_change_to_graph(
                                    &mut self.observable_graph,
                                    change,
                                );
                            }
                        },
                    );

                    // Metadata at bottom
                    ui.with_layout(
                        egui::Layout::bottom_up(egui::Align::LEFT),
                        |ui| {
                            ui.label(format!(
                                "Mappings: {}",
                                self.observable_graph.edge_count()
                            ));
                            ui.separator();
                        },
                    );
                });
            });

        // Center panel: Bipartite graph visualization
        egui::CentralPanel::default()
            .frame(
                egui::Frame::central_panel(&ctx.style())
                    .inner_margin(8.0),
            )
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    ui.heading("Observable Mapping");
                    ui.separator();

                    // Reset layout if needed
                    if self.mapping_layout_reset_needed {
                        reset_layout::<LayoutStateBipartite>(
                            ui, None,
                        );
                        self.mapping_layout_reset_needed = false;
                    }

                    // Clear edge selections when not in EdgeEditor mode
                    if self.mode == EditMode::NodeEditor {
                        self.observable_graph
                            .set_selected_edges(Vec::new());
                    }

                    // Update edge thicknesses based on global weight distribution
                    let sorted_weights = collect_sorted_weights(
                        &self.observable_graph,
                    );
                    graph_view::update_edge_thicknesses(
                        &mut self.observable_graph,
                        sorted_weights,
                    );

                    let settings_interaction =
                        self.get_settings_interaction();
                    let settings_style = self.get_settings_style();

                    // Allocate remaining space for the graph
                    let available_height =
                        ui.available_height() - 60.0;
                    ui.allocate_ui_with_layout(
                        egui::Vec2::new(
                            ui.available_width(),
                            available_height,
                        ),
                        egui::Layout::top_down(egui::Align::Center),
                        |ui| {
                            ui.add(
                                &mut ObservableGraphView::new(
                                    &mut self.observable_graph,
                                )
                                .with_interactions(
                                    &settings_interaction,
                                )
                                .with_styles(&settings_style),
                            );

                            // Edge editing functionality (only in Edge Editor mode)
                            if self.mode == EditMode::EdgeEditor {
                                let pointer =
                                    ui.input(|i| i.pointer.clone());

                                // Handle edge creation and draw preview line if needed
                                if let Some((from_pos, to_pos)) = self
                                    .handle_mapping_edge_creation(
                                        &pointer,
                                    )
                                {
                                    ui.painter().line_segment(
                                        [from_pos, to_pos],
                                        egui::Stroke::new(
                                            EDGE_PREVIEW_STROKE_WIDTH,
                                            EDGE_PREVIEW_COLOR,
                                        ),
                                    );
                                }

                                self.handle_mapping_edge_deletion(
                                    &pointer,
                                );
                            } else {
                                // Reset dragging state and clear selections when not in Edge Editor mode
                                self.dragging_from = None;
                                self.drag_started = false;
                                self.observable_graph
                                    .set_selected_edges(Vec::new());
                            }
                        },
                    );

                    // Controls and metadata at the bottom
                    ui.with_layout(
                        egui::Layout::bottom_up(egui::Align::LEFT),
                        |ui| {
                            let (mode_text, hint_text) = match self
                                .mode
                            {
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
                            ui.checkbox(
                                &mut self.show_labels,
                                "Show Labels",
                            );
                            ui.separator();
                        },
                    );
                });
            });
    }

    fn render_observed_dynamics_tab(&mut self, ctx: &egui::Context) {
        // Calculate exact 1/3 split for all three panels
        let available_width = ctx.available_rect().width();
        let panel_width = available_width / 3.0;

        // Left panel - read-only node list
        egui::SidePanel::left("observed_left_panel")
            .exact_width(panel_width)
            .frame(egui::Frame::side_top_panel(&ctx.style()).inner_margin(8.0))
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    ui.heading("Observed Values");
                    ui.separator();

                    // Contents - node list (read-only, no add button)
                    let available_height = ui.available_height() - 40.0;
                    egui::ScrollArea::vertical()
                        .max_height(available_height)
                        .show(ui, |ui| {
                            let nodes: Vec<_> = self
                                .observed_graph
                                .nodes_iter()
                                .map(|(idx, node)| {
                                    (idx, node.payload().name.clone())
                                })
                                .collect();

                            for (node_idx, node_name) in nodes {
                                let is_selected = self
                                    .observed_graph
                                    .node(node_idx)
                                    .map(|n| n.selected())
                                    .unwrap_or(false);

                                ui.horizontal(|ui| {
                                    // Collapsible arrow
                                    let arrow = if is_selected { "▼" } else { "▶" };
                                    if ui.small_button(arrow).clicked() {
                                        if is_selected {
                                            if let Some(node) = self.observed_graph.node_mut(node_idx) {
                                                node.set_selected(false);
                                            }
                                        } else {
                                            let all_nodes: Vec<_> = self.observed_graph.nodes_iter().map(|(idx, _)| idx).collect();
                                            for idx in all_nodes {
                                                if let Some(node) = self.observed_graph.node_mut(idx) {
                                                    node.set_selected(false);
                                                }
                                            }
                                            if let Some(node) = self.observed_graph.node_mut(node_idx) {
                                                node.set_selected(true);
                                            }
                                        }
                                    }

                                    // Display name as label (read-only)
                                    ui.label(&node_name);
                                });

                                // Show connections when selected
                                if is_selected {
                                    let incoming: Vec<String> = self
                                        .observed_graph
                                        .edges_directed(node_idx, petgraph::Direction::Incoming)
                                        .map(|edge_ref| {
                                            let other_idx = edge_ref.source();
                                            self.observed_graph
                                                .node(other_idx)
                                                .map(|n| n.payload().name.clone())
                                                .unwrap_or_else(|| String::from("???"))
                                        })
                                        .collect();

                                    let outgoing: Vec<String> = self
                                        .observed_graph
                                        .edges_directed(node_idx, petgraph::Direction::Outgoing)
                                        .map(|edge_ref| {
                                            let other_idx = edge_ref.target();
                                            self.observed_graph
                                                .node(other_idx)
                                                .map(|n| n.payload().name.clone())
                                                .unwrap_or_else(|| String::from("???"))
                                        })
                                        .collect();

                                    ui.label(format!("Incoming ({}):", incoming.len()));
                                    if incoming.is_empty() {
                                        ui.label("  None");
                                    } else {
                                        for name in incoming {
                                            ui.label(format!("  ← {}", name));
                                        }
                                    }

                                    ui.label(format!("Outgoing ({}):", outgoing.len()));
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
                            ui.label(format!("Values: {}", self.observed_graph.node_count()));
                            ui.separator();
                        },
                    );
                });
            });

        // Right panel - read-only heatmap
        egui::SidePanel::right("observed_right_panel")
            .exact_width(panel_width)
            .frame(
                egui::Frame::side_top_panel(&ctx.style())
                    .inner_margin(8.0),
            )
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    ui.heading("Observed Dynamics Heatmap");
                    ui.separator();

                    let available_height =
                        ui.available_height() - 40.0;
                    ui.allocate_ui_with_layout(
                        egui::Vec2::new(
                            ui.available_width(),
                            available_height,
                        ),
                        egui::Layout::top_down(egui::Align::Center),
                        |ui| {
                            let (
                                x_labels,
                                y_labels,
                                matrix,
                                x_node_indices,
                                y_node_indices,
                            ) = build_heatmap_data(
                                &self.observed_graph,
                            );

                            // Display heatmap without editing
                            let editing_state =
                                heatmap::EditingState {
                                    editing_cell: None, // Always None for read-only
                                    edit_buffer: String::new(),
                                };

                            let (
                                new_hover,
                                _new_editing,
                                _weight_change,
                            ) = heatmap::show_heatmap(
                                ui,
                                &x_labels,
                                &y_labels,
                                &matrix,
                                &x_node_indices,
                                &y_node_indices,
                                self.heatmap_hovered_cell,
                                editing_state,
                            );

                            self.heatmap_hovered_cell = new_hover;
                            // Ignore editing and weight changes (read-only)
                        },
                    );

                    // Metadata at bottom
                    ui.with_layout(
                        egui::Layout::bottom_up(egui::Align::LEFT),
                        |ui| {
                            ui.label(format!(
                                "Edges: {}",
                                self.observed_graph.edge_count()
                            ));
                            ui.separator();
                        },
                    );
                });
            });

        // Center panel - read-only graph visualization
        egui::CentralPanel::default()
            .frame(
                egui::Frame::central_panel(&ctx.style())
                    .inner_margin(8.0),
            )
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    ui.heading("Observed Graph");
                    ui.separator();

                    // Reset layout if needed
                    if self.observed_layout_reset_needed {
                        reset_layout::<LayoutStateCircular>(ui, None);
                        self.observed_layout_reset_needed = false;
                    }

                    // Update edge thicknesses based on global weight distribution
                    let sorted_weights =
                        collect_sorted_weights(&self.observed_graph);
                    graph_view::update_edge_thicknesses(
                        &mut self.observed_graph,
                        sorted_weights,
                    );

                    let settings_interaction =
                        SettingsInteraction::new()
                            .with_dragging_enabled(false)
                            .with_node_clicking_enabled(true)
                            .with_node_selection_enabled(true);
                    let settings_style = self.get_settings_style();

                    let available_height =
                        ui.available_height() - 60.0;
                    ui.allocate_ui_with_layout(
                        egui::Vec2::new(
                            ui.available_width(),
                            available_height,
                        ),
                        egui::Layout::top_down(egui::Align::Center),
                        |ui| {
                            ui.add(
                                &mut ObservedGraphView::new(
                                    &mut self.observed_graph,
                                )
                                .with_interactions(
                                    &settings_interaction,
                                )
                                .with_styles(&settings_style),
                            );
                        },
                    );

                    // Controls at bottom (read-only, no mode switching)
                    ui.with_layout(
                        egui::Layout::bottom_up(egui::Align::LEFT),
                        |ui| {
                            ui.label("Read-only view");
                            ui.checkbox(
                                &mut self.show_labels,
                                "Show Labels",
                            );
                            ui.separator();
                        },
                    );
                });
            });
    }
}
