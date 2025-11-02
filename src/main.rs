mod layout_circular;

use eframe::egui;
use egui_graphs::{
    reset_layout, DefaultEdgeShape, DefaultNodeShape, Graph, GraphView,
    SettingsInteraction, SettingsStyle,
};
use layout_circular::{LayoutCircular, LayoutStateCircular};
use petgraph::Directed;
use petgraph::graph::DefaultIx;
use petgraph::stable_graph::{EdgeIndex, NodeIndex, StableGraph};
use petgraph::visit::EdgeRef;
use std::time::Duration;

// UI Constants
const DRAG_THRESHOLD: f32 = 2.0;
const EDGE_PREVIEW_STROKE_WIDTH: f32 = 2.0;
const EDGE_PREVIEW_COLOR: egui::Color32 =
    egui::Color32::from_rgb(100, 100, 255);
const REPAINT_INTERVAL_MS: u64 = 16; // ~60 FPS

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

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "Graph Editor",
        options,
        Box::new(|_cc| {
            let g = generate_graph();
            let mut graph = Graph::from(&g);
            // Set labels for all nodes
            for (idx, node) in g.node_indices().zip(g.node_weights())
            {
                if let Some(graph_node) = graph.node_mut(idx) {
                    graph_node.set_label(node.name.clone());
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
}

impl GraphEditor {
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
        SettingsStyle::new().with_labels_always(self.show_labels)
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
        let elapsed_time = ctx.input(|i| i.time) as f32;

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

        egui::SidePanel::left("left_panel").show(ctx, |ui| {
            ui.heading("Nodes");

            if ui.button("Add Node").clicked() {
                let node_idx = self.g.add_node(NodeData {
                    name: String::new(),
                });
                let default_name =
                    format!("Node {}", node_idx.index());
                set_node_name(&mut self.g, node_idx, default_name);
                self.layout_reset_needed = true;
            }

            ui.label(format!("Nodes: {}", self.g.node_count()));

            ui.checkbox(&mut self.show_labels, "Show Labels");

            ui.separator();

            egui::ScrollArea::vertical().show(ui, |ui| {
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

            ui.with_layout(
                egui::Layout::bottom_up(egui::Align::LEFT),
                |ui| {
                    ui.label(format!("Time: {:.1} s", elapsed_time));
                    ui.separator();

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
                    ui.label(mode_text);
                    ui.label(hint_text);
                    ui.separator();
                },
            );
        });

        egui::CentralPanel::default().show(ctx, |ui| {
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
        });

        ctx.request_repaint_after(Duration::from_millis(
            REPAINT_INTERVAL_MS,
        ));

        // Update previous mode for next frame
        self.prev_mode = self.mode;
    }
}
