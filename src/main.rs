mod layout_circular;
mod heatmap;

use eframe::egui;
use egui_graphs::{
    reset_layout, DefaultEdgeShape, DefaultNodeShape, Graph, GraphView,
    SettingsInteraction, SettingsStyle,
};
use layout_circular::{LayoutCircular, LayoutStateCircular, SortOrder, SpacingConfig};
use petgraph::Directed;
use petgraph::graph::DefaultIx;
use petgraph::stable_graph::{EdgeIndex, NodeIndex, StableGraph};
use petgraph::visit::EdgeRef;
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

#[derive(Clone)]
struct NodeData {
    name: String,
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

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "Graph Editor",
        options,
        Box::new(|_cc| {
            let g = generate_graph();
            let mut graph: Graph<NodeData, (), Directed, DefaultIx, DefaultNodeShape, DefaultEdgeShape> = Graph::from(&g);
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
            Ok(Box::new(GraphEditor {
                g: graph,
                mode: EditMode::NodeEditor,
                prev_mode: EditMode::NodeEditor,
                dragging_from: None,
                drag_started: false,
                show_labels: true,
                layout_reset_needed: false,
                heatmap_hovered_cell: None,
            }))
        }),
    )
}

fn clear_edge_label(
    graph: &mut Graph<NodeData>,
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

// ------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq)]
enum EditMode {
    NodeEditor,
    EdgeEditor,
}

struct GraphEditor {
    g: Graph<NodeData>,
    mode: EditMode,
    prev_mode: EditMode,
    dragging_from: Option<(NodeIndex, egui::Pos2)>,
    drag_started: bool,
    show_labels: bool,
    layout_reset_needed: bool,
    heatmap_hovered_cell: Option<(usize, usize)>,
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
}

impl eframe::App for GraphEditor {
    fn update(
        &mut self,
        ctx: &egui::Context,
        _frame: &mut eframe::Frame,
    ) {
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
                            }
                            if ui.button("🗑").clicked() {
                                self.g.remove_node(node_idx);
                                self.layout_reset_needed = true;
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

        // Update previous mode for next frame
        self.prev_mode = self.mode;
    }
}
