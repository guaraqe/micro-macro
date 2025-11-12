mod actions;
mod cache;
mod effects;
mod graph_state;
mod graph_view;
mod heatmap;
mod layout_bipartite;
mod layout_circular;
mod serialization;
mod state;
mod store;
mod versioned;

use eframe::egui;
use egui_graphs::{SettingsInteraction, SettingsStyle, reset_layout};
use egui_extras::{StripBuilder, Size};
use graph_state::{
    ObservableNodeType,
    calculate_observed_graph_from_observable_display,
};
use graph_view::{
    ObservableGraphView, ObservedGraphView, StateGraphView,
    setup_graph_display,
};
use layout_bipartite::LayoutStateBipartite;
use layout_circular::{
    LayoutCircular, LayoutStateCircular, SortOrder, SpacingConfig,
};
use petgraph::stable_graph::NodeIndex;
use petgraph::visit::EdgeRef;
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
        Box::new(|cc| {
            // Set light theme
            cc.egui_ctx.set_visuals(egui::Visuals::light());

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

/// Collect all edge weights from a graph and return them sorted (including duplicates)
/// Always prepends 0.0 to ensure the smallest actual weight doesn't map to minimum thickness
// Helper function to create histogram data from node weights
fn create_weight_histogram(
    data: &[(String, f32)],
) -> (Vec<egui_plot::Bar>, Vec<String>) {
    if data.is_empty() {
        return (Vec::new(), Vec::new());
    }

    let mut nodes: Vec<(String, f32)> = data.to_vec();
    nodes.sort_by(|a, b| a.0.cmp(&b.0));

    let names: Vec<String> =
        nodes.iter().map(|(name, _)| name.clone()).collect();

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

/// Create probability bars from raw data, normalizing by total weight
/// Render a probability distribution histogram with statistics
/// Takes a Prob distribution and displays it as a bar chart with entropy and effective states
fn render_probability_chart(
    ui: &mut egui::Ui,
    plot_id: &str,
    title: &str,
    chart_data: &cache::ProbabilityChart,
    color: egui::Color32,
    height: f32,
) {
    ui.label(title);

    // Extract data from ProbabilityChart for plotting
    let data: Vec<(String, f64)> = chart_data
        .distribution
        .enumerate()
        .map(|(node_idx, value)| {
            let label = chart_data
                .labels
                .get(&node_idx)
                .cloned()
                .unwrap_or_else(|| format!("Node {:?}", node_idx));
            (label, value)
        })
        .collect();

    // Sort by label for consistent display
    let mut sorted_data = data.clone();
    sorted_data.sort_by(|a, b| a.0.cmp(&b.0));

    let names: Vec<String> = sorted_data.iter().map(|(name, _)| name.clone()).collect();

    let bars: Vec<egui_plot::Bar> = sorted_data
        .into_iter()
        .enumerate()
        .map(|(i, (name, prob))| {
            egui_plot::Bar::new(i as f64, prob)
                .name(name)
                .width(0.8)
                .fill(color)
        })
        .collect();

    let chart = egui_plot::BarChart::new(plot_id, bars)
        .color(color)
        .highlight(true)
        .element_formatter(Box::new(|bar, _chart| format!("{:.4}", bar.value)));

    egui_plot::Plot::new(plot_id)
        .height(height)
        .show_axes([true, true])
        .allow_zoom(false)
        .allow_drag(false)
        .allow_scroll(false)
        .show_x(false)
        .show_background(false)
        .show_grid(false)
        .include_y(0.0)
        .include_y(1.0)
        .x_axis_formatter(move |val, _range| {
            let idx = val.value as usize;
            names.get(idx).cloned().unwrap_or_default()
        })
        .y_axis_label("Probability")
        .show(ui, |plot_ui| {
            plot_ui.bar_chart(chart);
        });

    // Display statistics below the plot
    let num_states = data.len() as f64;
    let percentage = if num_states > 0.0 {
        (chart_data.effective_states / num_states) * 100.0
    } else {
        0.0
    };

    ui.horizontal(|ui| {
        ui.label(format!("Entropy: {:.4}", chart_data.entropy));
        ui.separator();
        ui.label(format!("Eff: {:.2} ({:.0}%)", chart_data.effective_states, percentage));
    });
}

impl State {
    // Returns (incoming_connections, outgoing_connections) for a given node in any graph
    // Each connection is (node_name, edge_weight)
    fn get_connections<N>(
        graph: &graph_view::GraphDisplay<N>,
        node_idx: NodeIndex,
    ) -> (Vec<(String, f32)>, Vec<(String, f32)>)
    where
        N: Clone + graph_state::HasName,
    {
        let incoming: Vec<(String, f32)> = graph
            .edges_directed(node_idx, petgraph::Direction::Incoming)
            .map(|edge_ref| {
                let other_idx = edge_ref.source();
                let node_name = graph
                    .node(other_idx)
                    .map(|n| n.payload().name())
                    .unwrap_or_else(|| String::from("???"));
                let weight = *edge_ref.weight().payload();
                (node_name, weight)
            })
            .collect();

        let outgoing: Vec<(String, f32)> = graph
            .edges_directed(node_idx, petgraph::Direction::Outgoing)
            .map(|edge_ref| {
                let other_idx = edge_ref.target();
                let node_name = graph
                    .node(other_idx)
                    .map(|n| n.payload().name())
                    .unwrap_or_else(|| String::from("???"));
                let weight = *edge_ref.weight().payload();
                (node_name, weight)
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
            .with_labels_always(self.store.show_labels)
            .with_node_stroke_hook(
                |selected,
                 _dragged,
                 _node_color,
                 _current_stroke,
                 _style| {
                    if selected {
                        // Red for selected nodes (light theme)
                        egui::Stroke::new(
                            4.0,
                            egui::Color32::from_rgb(200, 60, 70),
                        )
                    } else {
                        egui::Stroke::new(
                            2.0,
                            egui::Color32::from_rgb(80, 80, 80),
                        )
                    }
                },
            )
            .with_edge_stroke_hook(
                |selected, _order, current_stroke, _style| {
                    // Use the width from current_stroke (which comes from WeightedEdgeShape)
                    // but change color based on selection (light theme colors)
                    if selected {
                        egui::Stroke::new(
                            current_stroke.width,
                            egui::Color32::from_rgb(100, 100, 100),
                        )
                    } else {
                        egui::Stroke::new(
                            current_stroke.width,
                            egui::Color32::from_rgb(140, 140, 140),
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
            && let Some(hovered) =
                self.store.state_graph.get().hovered_node()
            && let Some(press_pos) = pointer.interact_pos()
        {
            self.store.dragging_from = Some((hovered, press_pos));
            self.store.drag_started = false;
        }

        // Detect if mouse has moved (drag started)
        if pointer.primary_down()
            && self.store.dragging_from.is_some()
            && pointer.delta().length() > DRAG_THRESHOLD
        {
            self.store.drag_started = true;
        }

        // Determine if preview arrow should be drawn
        let arrow_coords = if self.store.drag_started {
            if let Some((_src_idx, from_pos)) =
                self.store.dragging_from
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
            if let Some((source_node, _pos)) =
                self.store.dragging_from
                && self.store.drag_started
            {
                // Drag completed - create edge if hovering different node
                if let Some(target_node) =
                    self.store.state_graph.get().hovered_node()
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
        if pointer.primary_clicked()
            && self.store.dragging_from.is_none()
        {
            let selected_edges: Vec<_> = self
                .store
                .state_graph
                .get()
                .selected_edges()
                .to_vec();

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
                self.store.observable_graph.get().hovered_node()
            && let Some(press_pos) = pointer.interact_pos()
        {
            self.store.dragging_from = Some((hovered, press_pos));
            self.store.drag_started = false;
        }

        // Detect if mouse has moved (drag started)
        if pointer.primary_down()
            && self.store.dragging_from.is_some()
            && pointer.delta().length() > DRAG_THRESHOLD
        {
            self.store.drag_started = true;
        }

        // Determine if preview arrow should be drawn
        let arrow_coords = if self.store.drag_started {
            if let Some((_src_idx, from_pos)) =
                self.store.dragging_from
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
            if let Some((source_node, _pos)) =
                self.store.dragging_from
                && self.store.drag_started
            {
                // Drag completed - create edge if hovering different node
                if let Some(target_node) =
                    self.store.observable_graph.get().hovered_node()
                    && source_node != target_node
                {
                    // Check node types: only allow Source -> Destination
                    let source_type = self
                        .store
                        .observable_graph
                        .get()
                        .node(source_node)
                        .map(|n| n.payload().node_type);
                    let target_type = self
                        .store
                        .observable_graph
                        .get()
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
        if pointer.primary_clicked()
            && self.store.dragging_from.is_none()
        {
            let selected_edges: Vec<_> = self
                .store
                .observable_graph
                .get()
                .selected_edges()
                .to_vec();

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

        let mut active_tab = self.store.active_tab;

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

        if active_tab != self.store.active_tab {
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

        let mut frame_mode = self.store.mode;
        if desired_mode != self.store.mode {
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
        if let Some(error) = self.store.error_message.clone() {
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
        // Left panel takes the space it needs (min/max width)
        egui::SidePanel::left("left_panel")
            .min_width(250.0)
            .max_width(400.0)
            .default_width(320.0)
            .resizable(true)
            .frame(egui::Frame::side_top_panel(&ctx.style()).inner_margin(8.0))
            .show(ctx, |ui| {
            ui.vertical(|ui| {
                // Panel name
                ui.heading("Nodes");
                ui.separator();

                // Controls
                if ui.button("Add Node").clicked() {
                    // Dispatch action instead of directly modifying state
                    let node_count = self.store.state_graph.get().node_count();
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
                        .store
                        .state_graph
                        .get()
                        .nodes_iter()
                        .map(|(idx, node)| {
                            (idx, node.payload().name.clone())
                        })
                        .collect();

                    for (node_idx, node_name) in nodes {
                        let is_selected = self
                            .store
                            .state_graph
                            .get()
                            .node(node_idx)
                            .map(|n| n.selected())
                            .unwrap_or(false);
                        let all_nodes: Vec<_> = self
                            .store
                            .state_graph
                            .get()
                            .nodes_iter()
                            .map(|(idx, _)| idx)
                            .collect();

                        ui.horizontal(|ui| {
                            // Collapsible arrow button
                            self.selection_widget(
                                ui,
                                node_idx,
                                is_selected,
                                |idx, selected| {
                                    actions::Action::SelectStateNode {
                                        node_idx: idx,
                                        selected,
                                    }
                                },
                                all_nodes,
                            );

                            self.label_editor(
                                ui,
                                node_idx,
                                node_name,
                                |idx, value| actions::Action::UpdateStateNodeLabelEditor { node_idx: idx, value },
                                |idx, new_name| actions::Action::RenameStateNode { node_idx: idx, new_name },
                            );

                            if ui.button("🗑").clicked() {
                                self.dispatch(actions::Action::RemoveStateNode { node_idx });
                            }
                        });

                        // Weight editor
                        self.weight_editor(ui, node_idx);

                        // Only show connection info if this node is selected
                        if is_selected {
                            let (incoming, outgoing) =
                                Self::get_connections(
                                    self.store.state_graph.get(),
                                    node_idx,
                                );
                            Self::connections_widget(ui, incoming, outgoing);
                        }
                    }
                });

                // Metadata at bottom
                ui.with_layout(
                    egui::Layout::bottom_up(egui::Align::LEFT),
                    |ui| {
                        ui.label(format!("Nodes: {}", self.store.state_graph.get().node_count()));
                        ui.separator();
                    },
                );
            });
        });

        // Calculate middle Viridis color for histograms
        let viridis_mid = {
            let c = colorous::VIRIDIS.eval_continuous(0.5);
            egui::Color32::from_rgb(c.r, c.g, c.b)
        };

        // Bottom panel for histograms - takes ~25% of screen height
        let total_height = ctx.input(|i| i.screen_rect().height());
        let histogram_height = (total_height * 0.25).max(180.0);

        egui::TopBottomPanel::bottom("histogram_panel")
            .exact_height(histogram_height)
            .frame(egui::Frame::side_top_panel(&ctx.style()).inner_margin(8.0))
            .show(ctx, |ui| {
                // Use StripBuilder for proper proportional layout in histogram strip
                StripBuilder::new(ui)
                    .size(Size::remainder().at_least(200.0)) // Weight histogram
                    .size(Size::remainder().at_least(200.0)) // Equilibrium histogram
                    .size(Size::remainder().at_least(150.0)) // Stats
                    .horizontal(|mut strip| {
                        // Weight histogram
                        strip.cell(|ui| {
                            ui.vertical(|ui| {
                                let state_data = self.cache.state_data.get(&self.store);
                                let plot_height = ui.available_height() - 30.0;

                                render_probability_chart(
                                    ui,
                                    "state_weight_histogram",
                                    "Weight Distribution",
                                    &state_data.weight_distribution,
                                    viridis_mid,
                                    plot_height,
                                );
                            });
                        });

                        // Equilibrium histogram
                        strip.cell(|ui| {
                            ui.vertical(|ui| {
                                let state_data = self.cache.state_data.get(&self.store);
                                let plot_height = ui.available_height() - 30.0;

                                render_probability_chart(
                                    ui,
                                    "state_equilibrium_histogram",
                                    "State Equilibrium Distribution",
                                    &state_data.equilibrium_distribution,
                                    viridis_mid,
                                    plot_height,
                                );
                            });
                        });

                        // Stats section
                        strip.cell(|ui| {
                        ui.label("Statistics");
                        let state_data = self.cache.state_data.get(&self.store);
                        ui.label(format!("Entropy rate: {:.4}", state_data.entropy_rate));
                        ui.label(format!("Balance dev: {:.4}", state_data.detailed_balance_deviation));
                        ui.separator();
                        let selected = self.store.state_selection();
                        ui.label(format!("Selected: {}", selected.len()));
                        ui.label(format!("Edges: {}", self.store.state_graph.get().edge_count()));
                        ui.label(format!("Nodes: {}", self.store.state_graph.get().node_count()));
                        });
                    });
            });

        // Central panel split horizontally: graph on left, heatmap on right
        egui::CentralPanel::default()
            .frame(egui::Frame::central_panel(&ctx.style()).inner_margin(8.0))
            .show(ctx, |ui| {
                // Use StripBuilder for proper 50/50 split
                StripBuilder::new(ui)
                    .size(Size::remainder()) // Graph - 50%
                    .size(Size::remainder()) // Heatmap - 50%
                    .horizontal(|mut strip| {
                        // Left: Graph
                        strip.cell(|ui| {
                        ui.heading("Graph");
                        ui.separator();

                        // Reset layout if state graph version changed
                        let state_version = self.store.state_graph.version();
                        self.store.state_layout_reset.run_if_version_changed(
                            state_version,
                            || {
                                reset_layout::<LayoutStateCircular>(ui, None);
                            },
                        );

                        // Clear edge selections when not in EdgeEditor mode
                        if mode == EditMode::NodeEditor {
                            self.dispatch(actions::Action::ClearEdgeSelections);
                        }

                        // Update edge thicknesses
                        let sorted_weights = self.cache.state_data.get(&self.store).sorted_weights.clone();
                        graph_view::update_edge_thicknesses(
                            self.store.state_graph.get_mut(),
                            sorted_weights,
                        );

                        let settings_interaction = self.get_settings_interaction(mode);
                        let settings_style = self.get_settings_style();

                        // Graph takes most of available space, leaving room for controls
                        ui.add(
                            &mut StateGraphView::new(self.store.state_graph.get_mut())
                                .with_interactions(&settings_interaction)
                                .with_styles(&settings_style),
                        );

                        // Edge editing functionality
                        if mode == EditMode::EdgeEditor {
                            let pointer = ui.input(|i| i.pointer.clone());
                            if let Some((from_pos, to_pos)) = self.handle_edge_creation(&pointer) {
                                ui.painter().line_segment(
                                    [from_pos, to_pos],
                                    egui::Stroke::new(EDGE_PREVIEW_STROKE_WIDTH, EDGE_PREVIEW_COLOR),
                                );
                            }
                            self.handle_edge_deletion(&pointer);
                        } else {
                            self.dispatch(actions::Action::SetDraggingFrom {
                                node_idx: None,
                                position: None,
                            });
                            self.dispatch(actions::Action::SetDragStarted { started: false });
                            self.dispatch(actions::Action::ClearEdgeSelections);
                        }

                        // Controls at bottom
                        let (mode_text, hint_text) = match mode {
                            EditMode::NodeEditor => ("Mode: Node Editor", "Hold Ctrl for Edge Editor"),
                            EditMode::EdgeEditor => ("Mode: Edge Editor", "Release Ctrl for Node Editor"),
                        };
                        ui.label(hint_text);
                        ui.label(mode_text);
                        ui.horizontal(|ui| {
                            let mut show_labels = self.store.show_labels;
                            ui.checkbox(&mut show_labels, "Show Labels");
                            if show_labels != self.store.show_labels {
                                self.dispatch(actions::Action::SetShowLabels { show: show_labels });
                            }
                            let mut show_weights = self.store.show_weights;
                            ui.checkbox(&mut show_weights, "Show Weights");
                            if show_weights != self.store.show_weights {
                                self.dispatch(actions::Action::SetShowWeights { show: show_weights });
                            }
                        });
                        });

                        // Right: Heatmap
                        strip.cell(|ui| {
                        ui.heading("Heatmap");
                        ui.separator();

                        // Build heatmap data
                        let (x_labels, y_labels, matrix, x_node_indices, y_node_indices) =
                            self.cache.state_data.get(&self.store).heatmap.clone();

                        let editing_state = heatmap::EditingState {
                            editing_cell: self.store.heatmap_editing_cell,
                            edit_buffer: self.store.heatmap_edit_buffer.clone(),
                        };

                        let (new_hover, new_editing, weight_change) = heatmap::show_heatmap(
                            ui,
                            &x_labels,
                            &y_labels,
                            &matrix,
                            &x_node_indices,
                            &y_node_indices,
                            self.store.heatmap_hovered_cell,
                            editing_state,
                        );

                        if new_hover != self.store.heatmap_hovered_cell {
                            self.dispatch(actions::Action::SetHeatmapHoveredCell { cell: new_hover });
                        }

                        if new_editing.editing_cell != self.store.heatmap_editing_cell {
                            self.dispatch(actions::Action::SetHeatmapEditingCell {
                                cell: new_editing.editing_cell,
                            });
                        }

                        if new_editing.edit_buffer != self.store.heatmap_edit_buffer {
                            self.dispatch(actions::Action::SetHeatmapEditBuffer {
                                buffer: new_editing.edit_buffer,
                            });
                        }

                        if let Some(change) = weight_change {
                            self.dispatch(actions::Action::UpdateStateEdgeWeightFromHeatmap {
                                source_idx: change.source_idx,
                                target_idx: change.target_idx,
                                new_weight: change.new_weight,
                            });
                        }
                        });
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
                        let node_count = self.store.observable_graph
                            .get()
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
                                .store.observable_graph
                                .get()
                                .nodes_iter()
                                .filter(|(_, node)| node.payload().node_type == ObservableNodeType::Destination)
                                .map(|(idx, node)| (idx, node.payload().name.clone()))
                                .collect();

                            for (node_idx, node_name) in dest_nodes {
                                let is_selected = self
                                    .store.observable_graph
                                    .get()
                                    .node(node_idx)
                                    .map(|n| n.selected())
                                    .unwrap_or(false);
                                let all_nodes: Vec<_> = self
                                    .store
                                    .observable_graph
                                    .get()
                                    .nodes_iter()
                                    .map(|(idx, _)| idx)
                                    .collect();

                                ui.horizontal(|ui| {
                                    // Collapsible arrow button
                                    self.selection_widget(
                                        ui,
                                        node_idx,
                                        is_selected,
                                        |idx, selected| {
                                            actions::Action::SelectObservableNode {
                                                node_idx: idx,
                                                selected,
                                            }
                                        },
                                        all_nodes,
                                    );

                                    self.label_editor(
                                        ui,
                                        node_idx,
                                        node_name,
                                        |idx, value| actions::Action::UpdateObservableDestinationNodeLabelEditor { node_idx: idx, value },
                                        |idx, new_name| actions::Action::RenameObservableDestinationNode { node_idx: idx, new_name },
                                    );

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
                                    let (incoming, _outgoing) =
                                        Self::get_connections(
                                            self.store.observable_graph.get(),
                                            node_idx,
                                        );
                                    Self::connections_widget(ui, incoming, vec![]);
                                }
                            }
                        });

                    // Metadata at bottom
                    ui.with_layout(
                        egui::Layout::bottom_up(egui::Align::LEFT),
                        |ui| {
                            let dest_count = self
                                .store
                                .observable_graph
                                .get()
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
                            ) = self.cache.observable_data.get(&self.store).heatmap.clone();

                            // Display heatmap with editing support
                            let editing_state =
                                heatmap::EditingState {
                                    editing_cell: self
                                        .store.heatmap_editing_cell,
                                    edit_buffer: self
                                        .store.heatmap_edit_buffer
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
                                self.store.heatmap_hovered_cell,
                                editing_state,
                            );

                            if new_hover != self.store.heatmap_hovered_cell {
                                self.dispatch(
                                    actions::Action::SetHeatmapHoveredCell {
                                        cell: new_hover,
                                    },
                                );
                            }

                            let editing_cell = new_editing.editing_cell;
                            if editing_cell != self.store.heatmap_editing_cell {
                                self.dispatch(
                                    actions::Action::SetHeatmapEditingCell {
                                        cell: editing_cell,
                                    },
                                );
                            }

                            let edit_buffer = new_editing.edit_buffer;
                            if edit_buffer != self.store.heatmap_edit_buffer {
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
                            let selected = self.store.observable_selection();
                            ui.label(format!(
                                "Selected: {}",
                                selected.len()
                            ));
                            ui.label(format!(
                                "Observables: {}",
                                self.store.observable_graph.get().edge_count()
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

                    // Reset layout if observable graph version changed
                    let observable_version = self.store.observable_graph.version();
                    self.store.observable_layout_reset.run_if_version_changed(
                        observable_version,
                        || {
                            reset_layout::<LayoutStateBipartite>(ui, None);
                        },
                    );

                    // Clear edge selections when not in EdgeEditor mode
                    if mode == EditMode::NodeEditor {
                        self.dispatch(actions::Action::ClearObservableEdgeSelections);
                    }

                    // Update edge thicknesses based on global weight distribution
                    let sorted_weights =
                        self.cache.observable_data.get(&self.store).sorted_weights.clone();
                    graph_view::update_edge_thicknesses(
                        self.store.observable_graph.get_mut(),
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
                                    self.store.observable_graph.get_mut(),
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
                            let mut show_labels = self.store.show_labels;
                            ui.checkbox(
                                &mut show_labels,
                                "Show Labels",
                            );
                            if show_labels != self.store.show_labels {
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
        self.store.ensure_observed_graph_fresh();
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
                            // Get nodes data first
                            let observed_data = self.cache.observed_data.get(&self.store);
                            let nodes: Vec<_> = observed_data
                                .graph
                                .nodes_iter()
                                .map(|(idx, node)| {
                                    (idx, node.payload().name.clone(), node.selected())
                                })
                                .collect();

                            let all_nodes: Vec<_> = nodes.iter().map(|(idx, _, _)| *idx).collect();

                            for (node_idx, node_name, is_selected) in nodes {
                                ui.horizontal(|ui| {
                                    // Collapsible arrow - now functional!
                                    self.selection_widget(
                                        ui,
                                        node_idx,
                                        is_selected,
                                        |idx, selected| {
                                            actions::Action::SelectObservedNode {
                                                node_idx: idx,
                                                selected,
                                            }
                                        },
                                        all_nodes.clone(),
                                    );

                                    // Display name as label (read-only)
                                    ui.label(&node_name);
                                });

                                // Display weight (read-only)
                                ui.horizontal(|ui| {
                                    ui.label("Weight:");
                                    let observed_data = self.cache.observed_data.get(&self.store);
                                    let weight = observed_data
                                        .graph
                                        .node(node_idx)
                                        .map(|n| n.payload().weight)
                                        .unwrap_or(0.0);
                                    ui.label(format!("{:.4}", weight));
                                });

                                // Show connections when selected
                                if is_selected {
                                    let observed_data = self.cache.observed_data.get(&self.store);
                                    let (incoming, outgoing) =
                                        Self::get_connections(
                                            &observed_data.graph,
                                            node_idx,
                                        );
                                    Self::connections_widget(ui, incoming, outgoing);
                                }
                            }
                        });

                    // Apply any pending observed node selection
                    if let Some((node_idx, selected)) = self.store.observed_node_selection.take() {
                        let observed_data = self.cache.observed_data.get_mut(&self.store);
                        if let Some(node) = observed_data.graph.node_mut(node_idx) {
                            node.set_selected(selected);
                        }
                    }

                    // Weight histogram at bottom
                    ui.separator();
                    ui.label("Weight Distribution");
                    let observed_stats = self.store.observed_node_weight_stats();
                    let (bars, names) =
                        create_weight_histogram(&observed_stats);

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
                        .y_axis_label("Probability")
                        .show(ui, |plot_ui| {
                            plot_ui.bar_chart(chart);
                        });

                    // Observed Equilibrium (from state) with statistics
                    let observed_data = self.cache.observed_data.get(&self.store);

                    render_probability_chart(
                        ui,
                        "observed_equilibrium_from_state",
                        "Observed Equilibrium (State × Observable)",
                        &observed_data.equilibrium_from_state,
                        egui::Color32::from_rgb(250, 150, 100),
                        150.0,
                    );

                    // Metadata at bottom
                    ui.with_layout(
                        egui::Layout::bottom_up(egui::Align::LEFT),
                        |ui| {
                            let observed_data = self.cache.observed_data.get(&self.store);
                            ui.label(format!("Values: {}", observed_data.graph.node_count()));
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
                            let observed_data = self.cache.observed_data.get(&self.store);
                            let (
                                x_labels,
                                y_labels,
                                matrix,
                                x_node_indices,
                                y_node_indices,
                            ) = observed_data.heatmap.clone();

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
                                self.store.heatmap_hovered_cell,
                                editing_state,
                            );

                            if new_hover != self.store.heatmap_hovered_cell {
                                self.dispatch(actions::Action::SetHeatmapHoveredCell { cell: new_hover });
                            }
                            // Ignore editing and weight changes (read-only)
                        },
                    );

                    // Calculated Observed Equilibrium with statistics
                    let observed_data = self.cache.observed_data.get(&self.store);

                    render_probability_chart(
                        ui,
                        "observed_equilibrium_calculated",
                        "Calculated Observed Equilibrium",
                        &observed_data.equilibrium_calculated,
                        egui::Color32::from_rgb(250, 150, 100),
                        150.0,
                    );

                    // Metadata and statistics at bottom
                    ui.with_layout(
                        egui::Layout::bottom_up(egui::Align::LEFT),
                        |ui| {
                            let observed_data = self.cache.observed_data.get(&self.store);
                            ui.label(format!(
                                "Entropy rate: {:.4}",
                                observed_data.entropy_rate
                            ));
                            ui.label(format!(
                                "Detailed balance deviation: {:.4}",
                                observed_data.detailed_balance_deviation
                            ));
                            ui.separator();
                            ui.label(format!(
                                "Edges: {}",
                                observed_data.graph.edge_count()
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

                    // Get settings first to avoid borrowing issues
                    let settings_interaction =
                        SettingsInteraction::new()
                            .with_dragging_enabled(false)
                            .with_node_clicking_enabled(true)
                            .with_node_selection_enabled(true);
                    let settings_style = self.get_settings_style();

                    // Get observed version before mutable borrow
                    let observed_version =
                        self.cache.observed_data.version();

                    // Get observed data (graph + weights) from unified cache
                    let observed_data =
                        self.cache.observed_data.get_mut(&self.store);

                    // Reset layout if the observed graph was recalculated (version changed)
                    self.store
                        .observed_layout_reset
                        .run_if_version_changed(
                            observed_version,
                            || {
                                reset_layout::<LayoutStateCircular>(
                                    ui, None,
                                );
                            },
                        );

                    graph_view::update_edge_thicknesses(
                        &mut observed_data.graph,
                        observed_data.sorted_weights.clone(),
                    );

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
                                    &mut observed_data.graph,
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
                            let mut show_labels =
                                self.store.show_labels;
                            ui.checkbox(
                                &mut show_labels,
                                "Show Labels",
                            );
                            if show_labels != self.store.show_labels {
                                self.dispatch(
                                    actions::Action::SetShowLabels {
                                        show: show_labels,
                                    },
                                );
                            }
                            let mut show_weights =
                                self.store.show_weights;
                            ui.checkbox(
                                &mut show_weights,
                                "Show Weights",
                            );
                            if show_weights != self.store.show_weights
                            {
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

    fn weight_editor(
        &mut self,
        ui: &mut egui::Ui,
        node_idx: NodeIndex,
    ) {
        // Weight editor
        ui.horizontal(|ui| {
            ui.label("Weight:");
            let current_weight = self
                .store
                .state_graph
                .get()
                .node(node_idx)
                .map(|n| n.payload().weight)
                .unwrap_or(1.0);
            let is_focused =
                self.store.weight_editor.node() == Some(node_idx);
            let mut weight_str = if is_focused {
                self.store.weight_editor.value()
            } else {
                current_weight.to_string()
            };
            let response = ui.text_edit_singleline(&mut weight_str);
            if response.gained_focus() || response.changed() {
                self.dispatch(
                    actions::Action::UpdateStateNodeWeightEditor {
                        node_idx,
                        value: weight_str,
                    },
                );
            };
            if response.lost_focus()
                && let Ok(new_weight) =
                    self.store.weight_editor.parse()
            {
                self.dispatch(
                    actions::Action::UpdateStateNodeWeight {
                        node_idx,
                        new_weight: new_weight.max(0.0),
                    },
                );
            }
        });
    }

    fn label_editor(
        &mut self,
        ui: &mut egui::Ui,
        node_idx: NodeIndex,
        current_label: String,
        on_update: impl FnOnce(NodeIndex, String) -> actions::Action,
        on_commit: impl FnOnce(NodeIndex, String) -> actions::Action,
    ) {
        let is_focused =
            self.store.label_editor.node() == Some(node_idx);
        let mut name_str = if is_focused {
            self.store.label_editor.value()
        } else {
            current_label
        };
        let response = ui.text_edit_singleline(&mut name_str);
        if response.gained_focus() || response.changed() {
            self.dispatch(on_update(node_idx, name_str.clone()));
        }
        if response.lost_focus() {
            let new_name = self.store.label_editor.value();
            self.dispatch(on_commit(node_idx, new_name));
        }
    }

    fn connections_widget(
        ui: &mut egui::Ui,
        incoming: Vec<(String, f32)>,
        outgoing: Vec<(String, f32)>,
    ) {
        if !incoming.is_empty() {
            ui.label(format!("Incoming ({}):", incoming.len()));
            for (name, weight) in incoming {
                ui.label(format!("  ⬅ {} ({:.3})", name, weight));
            }
        }

        if !outgoing.is_empty() {
            ui.label(format!("Outgoing ({}):", outgoing.len()));
            for (name, weight) in outgoing {
                ui.label(format!("  ➡ {} ({:.3})", name, weight));
            }
        }
    }

    fn selection_widget(
        &mut self,
        ui: &mut egui::Ui,
        node_idx: NodeIndex,
        is_selected: bool,
        on_select: impl Fn(NodeIndex, bool) -> actions::Action,
        all_node_indices: Vec<NodeIndex>,
    ) {
        let arrow = if is_selected { "⏷" } else { "⏵" };
        if ui.small_button(arrow).clicked() {
            if is_selected {
                self.dispatch(on_select(node_idx, false));
            } else {
                // Deselect all other nodes first
                for idx in all_node_indices {
                    if idx != node_idx {
                        self.dispatch(on_select(idx, false));
                    }
                }
                // Select this node
                self.dispatch(on_select(node_idx, true));
            }
        }
    }
}
