mod actions;
mod effects;
mod graph_state;
mod graph_view;
mod heatmap;
mod layout_bipartite;
mod layout_circular;
mod serialization;
mod state;
mod store;

use eframe::egui;
use egui_graphs::{
    DisplayEdge, DisplayNode, Graph, SettingsInteraction,
    SettingsStyle, reset_layout,
};
use graph_state::{
    ObservableNodeType, StateNode,
    calculate_observed_graph_from_observable_display,
};
use graph_view::{
    ObservableGraphDisplay, ObservableGraphView, ObservedGraphView,
    StateGraphView, setup_graph_display,
};
use layout_bipartite::LayoutStateBipartite;
use layout_circular::{
    LayoutCircular, LayoutStateCircular, SortOrder, SpacingConfig,
};
use petgraph::stable_graph::NodeIndex;
use petgraph::visit::{EdgeRef, IntoEdgeReferences};
use state::State;
use store::{ActiveTab, EditMode};

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

// ------------------------------------------------------------------

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "Graph Editor",
        options,
        Box::new(|_cc| {
            let (graph, observable_graph) =
                store::load_or_create_default_state();

            let observed_graph_raw =
                calculate_observed_graph_from_observable_display(
                    &observable_graph,
                );
            let observed_graph =
                setup_graph_display(&observed_graph_raw);

            let store = store::Store::new(
                graph,
                observable_graph,
                observed_graph,
            );

            Ok(Box::new(State::new(store)))
        }),
    )
}

// ------------------------------------------------------------------

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

// Helper function to create histogram data from node weights
fn create_weight_histogram<N, E, Ty, Ix, Dn, De>(
    graph: &Graph<N, E, Ty, Ix, Dn, De>,
    get_weight: impl Fn(&N) -> f32,
    get_name: impl Fn(&N) -> String,
) -> (Vec<egui_plot::Bar>, Vec<String>)
where
    N: Clone,
    E: Clone,
    Ty: petgraph::EdgeType,
    Ix: petgraph::graph::IndexType,
    Dn: DisplayNode<N, E, Ty, Ix>,
    De: DisplayEdge<N, E, Ty, Ix, Dn>,
{
    // Collect nodes with their names and weights
    let mut nodes: Vec<(String, f32)> = graph
        .nodes_iter()
        .map(|(_, node)| {
            let payload = node.payload();
            (get_name(payload), get_weight(payload))
        })
        .collect();

    // Sort by name
    nodes.sort_by(|a, b| a.0.cmp(&b.0));

    // Separate names for labels
    let names: Vec<String> =
        nodes.iter().map(|(name, _)| name.clone()).collect();

    // Create bars
    let bars = nodes
        .into_iter()
        .enumerate()
        .map(|(i, (name, weight))| {
            egui_plot::Bar::new(i as f64, weight as f64)
                .name(name)
                .width(0.8)
        })
        .collect();

    (bars, names)
}

impl State {
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
    fn get_settings_interaction(
        &self,
        mode: EditMode,
    ) -> SettingsInteraction {
        match mode {
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
                    self.dispatch(actions::Action::AddStateEdge {
                        source_idx: source_node,
                        target_idx: target_node,
                        weight: 1.0,
                    });
                }
            }
            self.dispatch(actions::Action::SetDraggingFrom {
                node_idx: None,
                position: None,
            });
            self.dispatch(actions::Action::SetDragStarted {
                started: false,
            });
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
                self.dispatch(
                    actions::Action::RemoveStateEdgeByIndex {
                        edge_idx: clicked_edge,
                    },
                );
            }
            // If no edges or different edge clicked, library handles
            // selection automatically
        }
    }

    // Edge creation for observable graph with Source->Destination constraint
    fn handle_observable_edge_creation(
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
                        self.dispatch(
                            actions::Action::AddObservableEdge {
                                source_idx: source_node,
                                target_idx: target_node,
                                weight: 1.0,
                            },
                        );
                    }
                    // Silently ignore invalid edge attempts (Dest->Source, Source->Source, Dest->Dest)
                }
            }
            self.dispatch(actions::Action::SetDraggingFrom {
                node_idx: None,
                position: None,
            });
            self.dispatch(actions::Action::SetDragStarted {
                started: false,
            });
        }

        arrow_coords
    }

    // Edge deletion for observable graph
    fn handle_observable_edge_deletion(
        &mut self,
        pointer: &egui::PointerState,
    ) {
        if pointer.primary_clicked() && self.dragging_from.is_none() {
            let selected_edges: Vec<_> =
                self.observable_graph.selected_edges().to_vec();

            if selected_edges.len() == 1 {
                let clicked_edge = selected_edges[0];
                self.dispatch(
                    actions::Action::RemoveObservableEdgeByIndex {
                        edge_idx: clicked_edge,
                    },
                );
            }
        }
    }
}

impl eframe::App for State {
    fn update(
        &mut self,
        ctx: &egui::Context,
        _frame: &mut eframe::Frame,
    ) {
        // Flush action queue at the beginning of update
        self.flush_actions();

        let mut active_tab = self.active_tab;

        // Menu bar at the very top
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::MenuBar::new().ui(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Save").clicked() {
                        ui.close();
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("JSON", &["json"])
                            .save_file()
                        {
                            self.dispatch(
                                actions::Action::SaveToFile { path },
                            );
                        }
                    }

                    if ui.button("Load").clicked() {
                        ui.close();
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("JSON", &["json"])
                            .pick_file()
                        {
                            self.dispatch(
                                actions::Action::LoadFromFile {
                                    path,
                                },
                            );
                        }
                    }
                });
            });
        });

        // Tab navigation below menu bar
        egui::TopBottomPanel::top("tabs").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(
                    &mut active_tab,
                    ActiveTab::DynamicalSystem,
                    "Dynamical System",
                );
                ui.selectable_value(
                    &mut active_tab,
                    ActiveTab::ObservableEditor,
                    "Observable Editor",
                );
                ui.selectable_value(
                    &mut active_tab,
                    ActiveTab::ObservedDynamics,
                    "Observed Dynamics",
                );
            });
        });

        if active_tab != self.active_tab {
            self.dispatch(actions::Action::SetActiveTab {
                tab: active_tab,
            });
        }

        // Detect Ctrl key to switch modes
        let ctrl_pressed = ctx.input(|i| i.modifiers.ctrl);
        let desired_mode = if ctrl_pressed {
            EditMode::EdgeEditor
        } else {
            EditMode::NodeEditor
        };

        let mut frame_mode = self.mode;
        if desired_mode != self.mode {
            self.dispatch(actions::Action::SetEditMode {
                mode: desired_mode,
            });
            frame_mode = desired_mode;
        }

        // Render the appropriate view based on active tab
        match active_tab {
            ActiveTab::DynamicalSystem => {
                self.render_dynamical_system_tab(ctx, frame_mode)
            }
            ActiveTab::ObservableEditor => {
                self.render_observable_editor_tab(ctx, frame_mode)
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
                        self.dispatch(
                            actions::Action::ClearErrorMessage,
                        );
                    }
                });
        }

        // Flush effects at the end of update
        self.flush_effects();
    }
}

impl State {
    fn render_dynamical_system_tab(
        &mut self,
        ctx: &egui::Context,
        mode: EditMode,
    ) {
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
                    // Dispatch action instead of directly modifying state
                    let node_count = self.state_graph.node_count();
                    let default_name = format!("Node {}", node_count);
                    self.dispatch(actions::Action::AddStateNode {
                        name: default_name,
                        weight: 1.0,
                    });
                    // Related layout/sync work runs immediately inside apply_action
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
                                    self.dispatch(actions::Action::SelectStateNode { node_idx, selected: false });
                                } else {
                                    // Deselect all other nodes first
                                    let all_nodes: Vec<_> = self.state_graph.nodes_iter().map(|(idx, _)| idx).collect();
                                    for idx in all_nodes {
                                        if idx != node_idx {
                                            self.dispatch(actions::Action::SelectStateNode { node_idx: idx, selected: false });
                                        }
                                    }
                                    // Select this node
                                    self.dispatch(actions::Action::SelectStateNode { node_idx, selected: true });
                                }
                            }

                            let response =
                                ui.text_edit_singleline(&mut node_name);
                            if response.changed() {
                                self.dispatch(actions::Action::RenameStateNode {
                                    node_idx,
                                    new_name: node_name.clone(),
                                });
                            }
                            if ui.button("🗑").clicked() {
                                self.dispatch(actions::Action::RemoveStateNode { node_idx });
                            }
                        });

                        // Weight editor
                        ui.horizontal(|ui| {
                            ui.label("Weight:");
                            let current_weight = self.state_graph.node(node_idx)
                                .map(|n| n.payload().weight)
                                .unwrap_or(1.0);
                            let mut weight_str = format!("{:.2}", current_weight);
                            let response = ui.text_edit_singleline(&mut weight_str);
                            if response.changed() {
                                if let Ok(new_weight) =
                                    weight_str.parse::<f32>()
                                {
                                    self.dispatch(
                                        actions::Action::UpdateStateNodeWeight {
                                            node_idx,
                                            new_weight: new_weight
                                                .max(0.0),
                                        },
                                    );
                                }
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

                // Weight histogram at bottom
                ui.separator();
                ui.label("Weight Distribution");
                let (bars, names) = create_weight_histogram(
                    &self.state_graph,
                    |node: &StateNode| node.weight,
                    |node: &StateNode| node.name.clone(),
                );

                // Calculate max weight for proper y-axis range with padding
                let max_weight = bars.iter()
                    .map(|bar| bar.value)
                    .fold(0.0f64, f64::max);
                let y_max = (max_weight * 1.15).ceil(); // Add 15% padding at top and ceiling

                let chart = egui_plot::BarChart::new("weights", bars)
                    .color(egui::Color32::from_rgb(100, 150, 250))
                    .highlight(true)
                    .element_formatter(Box::new(|bar, _chart| {
                        format!("{:.3}", bar.value)
                    }));

                egui_plot::Plot::new("state_weight_histogram")
                    .height(150.0)
                    .show_axes([true, true])
                    .allow_zoom(false)
                    .allow_drag(false)
                    .allow_scroll(false)
                    .show_background(false)
                    .show_grid(false)
                    .include_y(0.0)
                    .include_y(y_max)
                    .x_axis_formatter(move |val, _range| {
                        let idx = val.value as usize;
                        names.get(idx).cloned().unwrap_or_default()
                    })
                    .y_axis_label("Weight")
                    .show(ui, |plot_ui| {
                        plot_ui.bar_chart(chart);
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

                            if new_hover != self.heatmap_hovered_cell {
                                self.dispatch(
                                    actions::Action::SetHeatmapHoveredCell {
                                        cell: new_hover,
                                    },
                                );
                            }

                            let editing_cell = new_editing.editing_cell;
                            if editing_cell != self.heatmap_editing_cell {
                                self.dispatch(
                                    actions::Action::SetHeatmapEditingCell {
                                        cell: editing_cell,
                                    },
                                );
                            }

                            let edit_buffer = new_editing.edit_buffer;
                            if edit_buffer != self.heatmap_edit_buffer {
                                self.dispatch(
                                    actions::Action::SetHeatmapEditBuffer {
                                        buffer: edit_buffer,
                                    },
                                );
                            }

                            // Handle weight changes
                            if let Some(change) = weight_change {
                                self.dispatch(
                                    actions::Action::UpdateStateEdgeWeightFromHeatmap {
                                        source_idx: change.source_idx,
                                        target_idx: change.target_idx,
                                        new_weight: change.new_weight,
                                    },
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
                    if self.state_layout_reset_needed {
                        reset_layout::<LayoutStateCircular>(ui, None);
                        self.dispatch(
                            actions::Action::ClearLayoutResetFlags,
                        );
                    }

                    // Clear edge selections when not in EdgeEditor mode,
                    // before creating GraphView
                    if mode == EditMode::NodeEditor {
                        self.dispatch(
                            actions::Action::ClearEdgeSelections,
                        );
                    }

                    // Update edge thicknesses based on global weight distribution
                    let sorted_weights =
                        collect_sorted_weights(&self.state_graph);
                    graph_view::update_edge_thicknesses(
                        &mut self.state_graph,
                        sorted_weights,
                    );

                    let settings_interaction =
                        self.get_settings_interaction(mode);
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
                            if mode == EditMode::EdgeEditor {
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
                                self.dispatch(
                                    actions::Action::SetDraggingFrom {
                                        node_idx: None,
                                        position: None,
                                    },
                                );
                                self.dispatch(
                                    actions::Action::SetDragStarted {
                                        started: false,
                                    },
                                );
                                self.dispatch(
                                    actions::Action::ClearEdgeSelections,
                                );
                            }
                        },
                    );

                    // Controls and metadata at the bottom
                    ui.with_layout(
                        egui::Layout::bottom_up(egui::Align::LEFT),
                        |ui| {
                            let (mode_text, hint_text) =
                                match mode
                            {
                                EditMode::NodeEditor => (
                                    "Mode: Node Editor",
                                    "Hold Ctrl for Edge Editor",
                                ),
                                EditMode::EdgeEditor
                                => (
                                    "Mode: Edge Editor",
                                    "Release Ctrl for Node Editor",
                                ),
                            };
                            ui.label(hint_text);
                            ui.label(mode_text);
                            let mut show_labels = self.show_labels;
                            ui.checkbox(
                                &mut show_labels,
                                "Show Labels",
                            );
                            if show_labels != self.show_labels {
                                self.dispatch(
                                    actions::Action::SetShowLabels {
                                        show: show_labels,
                                    },
                                );
                            }

                            let mut show_weights = self.show_weights;
                            ui.checkbox(
                                &mut show_weights,
                                "Show Weights",
                            );
                            if show_weights != self.show_weights {
                                self.dispatch(
                                    actions::Action::SetShowWeights {
                                        show: show_weights,
                                    },
                                );
                            }
                            ui.separator();
                        },
                    );
                });
            });
    }

    fn render_observable_editor_tab(
        &mut self,
        ctx: &egui::Context,
        mode: EditMode,
    ) {
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
                        let node_count = self.observable_graph
                            .nodes_iter()
                            .filter(|(_, node)| node.payload().node_type == ObservableNodeType::Destination)
                            .count();
                        let default_name = format!("Value {}", node_count);
                        self.dispatch(actions::Action::AddObservableDestinationNode { name: default_name });
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
                                    if response.changed() {
                                        self.dispatch(
                                            actions::Action::RenameObservableDestinationNode {
                                                node_idx,
                                                new_name: node_name
                                                    .clone(),
                                            },
                                        );
                                    }
                                    if ui.button("🗑").clicked() {
                                        self.dispatch(
                                            actions::Action::RemoveObservableDestinationNode {
                                                node_idx,
                                            },
                                        );
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
                    ui.heading("Observable Heatmap");
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

                            if new_hover != self.heatmap_hovered_cell {
                                self.dispatch(
                                    actions::Action::SetHeatmapHoveredCell {
                                        cell: new_hover,
                                    },
                                );
                            }

                            let editing_cell = new_editing.editing_cell;
                            if editing_cell != self.heatmap_editing_cell {
                                self.dispatch(
                                    actions::Action::SetHeatmapEditingCell {
                                        cell: editing_cell,
                                    },
                                );
                            }

                            let edit_buffer = new_editing.edit_buffer;
                            if edit_buffer != self.heatmap_edit_buffer {
                                self.dispatch(
                                    actions::Action::SetHeatmapEditBuffer {
                                        buffer: edit_buffer,
                                    },
                                );
                            }

                            // Handle weight changes
                            if let Some(change) = weight_change {
                                self.dispatch(
                                    actions::Action::UpdateObservableEdgeWeightFromHeatmap {
                                        source_idx: change.source_idx,
                                        target_idx: change.target_idx,
                                        new_weight: change.new_weight,
                                    },
                                );
                            }
                        },
                    );

                    // Metadata at bottom
                    ui.with_layout(
                        egui::Layout::bottom_up(egui::Align::LEFT),
                        |ui| {
                            ui.label(format!(
                                "Observables: {}",
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
                    if self.observable_layout_reset_needed {
                        reset_layout::<LayoutStateBipartite>(
                            ui, None,
                        );
                        self.dispatch(
                            actions::Action::ClearLayoutResetFlags,
                        );
                    }

                    // Clear edge selections when not in EdgeEditor mode
                    if mode == EditMode::NodeEditor {
                        self.dispatch(actions::Action::ClearObservableEdgeSelections);
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
                        self.get_settings_interaction(mode);
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
                            if mode == EditMode::EdgeEditor
                            {
                                let pointer =
                                    ui.input(|i| i.pointer.clone());

                                // Handle edge creation and draw preview line if needed
                                if let Some((from_pos, to_pos)) = self
                                    .handle_observable_edge_creation(
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

                                self.handle_observable_edge_deletion(
                                    &pointer,
                                );
                            } else {
                                // Reset dragging state and clear selections when not in Edge Editor mode
                                self.dispatch(
                                    actions::Action::SetDraggingFrom {
                                        node_idx: None,
                                        position: None,
                                    },
                                );
                                self.dispatch(
                                    actions::Action::SetDragStarted {
                                        started: false,
                                    },
                                );
                                self.dispatch(
                                    actions::Action::ClearObservableEdgeSelections,
                                );
                            }
                        },
                    );

                    // Controls and metadata at the bottom
                    ui.with_layout(
                        egui::Layout::bottom_up(egui::Align::LEFT),
                        |ui| {
                            let (mode_text, hint_text) = match mode {
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
                            let mut show_labels = self.show_labels;
                            ui.checkbox(
                                &mut show_labels,
                                "Show Labels",
                            );
                            if show_labels != self.show_labels {
                                self.dispatch(
                                    actions::Action::SetShowLabels {
                                        show: show_labels,
                                    },
                                );
                            }
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

                                // Display weight (read-only)
                                ui.horizontal(|ui| {
                                    ui.label("Weight:");
                                    let weight = self.observed_graph.node(node_idx)
                                        .map(|n| n.payload().weight)
                                        .unwrap_or(0.0);
                                    ui.label(format!("{:.4}", weight));
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

                    // Weight histogram at bottom
                    ui.separator();
                    ui.label("Weight Distribution");
                    let (bars, names) = create_weight_histogram(
                        &self.observed_graph,
                        |node: &graph_state::ObservedNode| node.weight,
                        |node: &graph_state::ObservedNode| node.name.clone(),
                    );

                    // Calculate max weight for proper y-axis range with padding
                    let max_weight = bars.iter()
                        .map(|bar| bar.value)
                        .fold(0.0f64, f64::max);
                    let y_max = (max_weight * 1.15).ceil(); // Add 15% padding at top and ceiling

                    let chart = egui_plot::BarChart::new("weights", bars)
                        .color(egui::Color32::from_rgb(250, 150, 100))
                        .highlight(true)
                        .element_formatter(Box::new(|bar, _chart| {
                            format!("{:.3}", bar.value)
                        }));

                    egui_plot::Plot::new("observed_weight_histogram")
                        .height(150.0)
                        .show_axes([true, true])
                        .allow_zoom(false)
                        .allow_drag(false)
                        .allow_scroll(false)
                        .show_background(false)
                        .show_grid(false)
                        .include_y(0.0)
                        .include_y(y_max)
                        .x_axis_formatter(move |val, _range| {
                            let idx = val.value as usize;
                            names.get(idx).cloned().unwrap_or_default()
                        })
                        .y_axis_label("Weight")
                        .show(ui, |plot_ui| {
                            plot_ui.bar_chart(chart);
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

                            if new_hover != self.heatmap_hovered_cell {
                                self.dispatch(actions::Action::SetHeatmapHoveredCell { cell: new_hover });
                            }
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
                        self.dispatch(
                            actions::Action::ClearLayoutResetFlags,
                        );
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
                            let mut show_labels = self.show_labels;
                            ui.checkbox(
                                &mut show_labels,
                                "Show Labels",
                            );
                            if show_labels != self.show_labels {
                                self.dispatch(
                                    actions::Action::SetShowLabels {
                                        show: show_labels,
                                    },
                                );
                            }
                            let mut show_weights = self.show_weights;
                            ui.checkbox(
                                &mut show_weights,
                                "Show Weights",
                            );
                            if show_weights != self.show_weights {
                                self.dispatch(
                                    actions::Action::SetShowWeights {
                                        show: show_weights,
                                    },
                                );
                            }
                            ui.separator();
                        },
                    );
                });
            });
    }
}
