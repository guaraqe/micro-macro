mod actions;
mod cache;
mod effects;
mod graph_state;
mod graph_view;
mod heatmap;
mod layout_bipartite;
mod layout_circular;
mod layout_settings;
mod node_shapes;
mod serialization;
mod state;
mod store;
mod versioned;

use crate::layout_settings::{
    BIPARTITE_LAYER_GAP_RANGE, BIPARTITE_NODE_GAP_RANGE,
    CIRCULAR_BASE_RADIUS_RANGE, EDGE_THICKNESS_MAX_RANGE,
    EDGE_THICKNESS_MIN_RANGE, LABEL_FONT_RANGE, LABEL_GAP_RANGE,
    LOOP_RADIUS_RANGE, NODE_RADIUS_RANGE,
};
use eframe::egui;
use egui_extras::{Size, StripBuilder};
use egui_graphs::{
    DisplayNode, SettingsInteraction, SettingsNavigation,
    SettingsStyle, reset_layout,
};
use graph_state::{
    ObservableNodeType,
    calculate_observed_graph_from_observable_display,
};
use graph_view::{
    ObservableGraphView, ObservedGraphView, StateGraphView,
    set_loop_radius, setup_observed_graph_display,
};
use layout_bipartite::LayoutStateBipartite;
use layout_circular::{LayoutStateCircular, SpacingConfig};
use petgraph::{
    Directed, graph::DefaultIx, stable_graph::NodeIndex,
    visit::EdgeRef,
};
use state::State;
use store::{ActiveTab, EditMode};

// UI Constants
const DRAG_THRESHOLD: f32 = 2.0;
const EDGE_PREVIEW_STROKE_WIDTH: f32 = 2.0;
const EDGE_PREVIEW_COLOR: egui::Color32 =
    egui::Color32::from_rgb(100, 100, 255);
const GRAPH_FIT_PADDING: f32 = 0.75;

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

            let (graph, observable_graph, layout_settings) =
                store::load_or_create_default_state();

            let observed_graph_raw =
                calculate_observed_graph_from_observable_display(
                    &observable_graph,
                );
            let observed_graph =
                setup_observed_graph_display(&observed_graph_raw);

            let store = store::Store::new(
                graph,
                observable_graph,
                observed_graph,
                layout_settings,
            );

            Ok(Box::new(State::new(store)))
        }),
    )
}

// ------------------------------------------------------------------

/// Collect all edge weights from a graph and return them sorted (including duplicates)
/// Always prepends 0.0 to ensure the smallest actual weight doesn't map to minimum thickness
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

    let names: Vec<String> =
        sorted_data.iter().map(|(name, _)| name.clone()).collect();

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
        .element_formatter(Box::new(|bar, _chart| {
            format!("{:.4}", bar.value)
        }));

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
        ui.label(format!(
            "Eff: {:.2} ({:.0}%)",
            chart_data.effective_states, percentage
        ));
    });
}

type NodeConnections = (Vec<(String, f32)>, Vec<(String, f32)>);

impl State {
    fn render_state_validation_panel(
        &mut self,
        ui: &mut egui::Ui,
        errors: &[cache::StateValidationIssue],
    ) {
        if errors.is_empty() {
            return;
        }

        egui::Frame::new()
            .fill(egui::Color32::from_rgb(255, 230, 230))
            .stroke(egui::Stroke::new(
                1.0,
                egui::Color32::from_rgb(200, 60, 60),
            ))
            .inner_margin(egui::Margin::symmetric(8, 8))
            .show(ui, |ui| {
                ui.vertical(|ui| {
                    ui.strong("State Graph Validation Issues");
                    ui.add_space(4.0);
                    for error in errors {
                        let node_idx = match error {
                            cache::StateValidationIssue::NoOutgoingEdges { node, .. } => *node,
                            cache::StateValidationIssue::NoIncomingEdges { node, .. } => *node,
                        };

                        let text = egui::RichText::new(format!("• {}", error))
                            .color(egui::Color32::from_rgb(170, 30, 30));

                        let button = egui::Button::new(text)
                            .fill(egui::Color32::TRANSPARENT)
                            .frame(false);

                        if ui.add(button).clicked() {
                            self.dispatch(actions::Action::SelectStateNode {
                                node_idx,
                                selected: true,
                            });
                            self.dispatch(actions::Action::SetActiveTab {
                                tab: store::ActiveTab::DynamicalSystem,
                            });
                        }
                    }
                });
            });
    }

    fn render_observable_validation_panel(
        &mut self,
        ui: &mut egui::Ui,
        errors: &[cache::ObservableValidationIssue],
    ) {
        if errors.is_empty() {
            return;
        }

        egui::Frame::new()
            .fill(egui::Color32::from_rgb(255, 230, 230))
            .stroke(egui::Stroke::new(
                1.0,
                egui::Color32::from_rgb(200, 60, 60),
            ))
            .inner_margin(egui::Margin::symmetric(8, 8))
            .show(ui, |ui| {
                ui.vertical(|ui| {
                    ui.strong("Observable Graph Validation Issues");
                    ui.add_space(4.0);
                    for error in errors {
                        let node_idx = match error {
                            cache::ObservableValidationIssue::SourceNoOutgoingEdges { node, .. } => *node,
                            cache::ObservableValidationIssue::DestinationNoIncomingEdges { node, .. } => *node,
                        };

                        let text = egui::RichText::new(format!("• {}", error))
                            .color(egui::Color32::from_rgb(170, 30, 30));

                        let button = egui::Button::new(text)
                            .fill(egui::Color32::TRANSPARENT)
                            .frame(false);

                        if ui.add(button).clicked() {
                            self.dispatch(actions::Action::SelectObservableNode {
                                node_idx,
                                selected: true,
                            });
                            self.dispatch(actions::Action::SetActiveTab {
                                tab: store::ActiveTab::ObservableEditor,
                            });
                        }
                    }
                });
            });
    }

    // Returns (incoming_connections, outgoing_connections) for a given node in any graph
    // Each connection is (node_name, edge_weight)
    fn get_connections<N, D>(
        graph: &graph_view::GraphDisplay<N, D>,
        node_idx: NodeIndex,
    ) -> NodeConnections
    where
        N: Clone + graph_state::HasName,
        D: DisplayNode<N, f32, Directed, DefaultIx>,
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
    fn get_settings_style(
        &self,
        labels_always: bool,
    ) -> SettingsStyle {
        SettingsStyle::new()
            .with_labels_always(labels_always)
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

    fn get_settings_navigation(&self) -> SettingsNavigation {
        SettingsNavigation::new()
            .with_fit_to_screen_enabled(true)
            .with_fit_to_screen_padding(GRAPH_FIT_PADDING)
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
                self.store.state.graph.get().hovered_node()
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
                    self.store.state.graph.get().hovered_node()
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
                .state.graph
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
                self.store.observable.graph.get().hovered_node()
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
                    self.store.observable.graph.get().hovered_node()
                    && source_node != target_node
                {
                    // Check node types: only allow Source -> Destination
                    let source_type = self
                        .store
                        .observable.graph
                        .get()
                        .node(source_node)
                        .map(|n| n.payload().node_type);
                    let target_type = self
                        .store
                        .observable.graph
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
                .observable.graph
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
        // Left panel fixed at 25% of the screen width
        let screen_width = ctx.viewport_rect().width().max(1.0);
        let left_panel_width = screen_width * 0.25;

        egui::SidePanel::left("left_panel")
            .exact_width(left_panel_width)
            .resizable(false)
            .frame(egui::Frame::side_top_panel(&ctx.style()).inner_margin(8.0))
            .show(ctx, |panel_ui| {
                egui::TopBottomPanel::bottom("state_left_footer")
                    .resizable(false)
                    .frame(egui::Frame::NONE)
                    .show_inside(panel_ui, |ui| {
                        let validation_errors = self.cache.state_data.get(&self.store).validation_errors.clone();
                        self.render_state_validation_panel(
                            ui,
                            &validation_errors,
                        );
                        ui.add_space(6.0);
                        self.layout_settings_panel(
                            ui,
                            ActiveTab::DynamicalSystem,
                        );
                    });
            egui::CentralPanel::default()
                .frame(egui::Frame::NONE)
                .show_inside(panel_ui, |ui| {
                ui.vertical(|ui| {
                // Panel name
                ui.heading("Nodes");
                ui.separator();

                // Controls
                if ui.button("Add Node").clicked() {
                    // Dispatch action instead of directly modifying state
                    let node_count = self.store.state.graph.get().node_count();
                    let default_name = format!("State {}", node_count + 1);
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
                        .state.graph
                        .get()
                        .nodes_iter()
                        .map(|(idx, node)| {
                            (idx, node.payload().name.clone())
                        })
                        .collect();

                    for (node_idx, node_name) in nodes {
                        let is_selected = self
                            .store
                            .state.graph
                            .get()
                            .node(node_idx)
                            .map(|n| n.selected())
                            .unwrap_or(false);
                        let all_nodes: Vec<_> = self
                            .store
                            .state.graph
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

                            ui.add_space(ui.spacing().item_spacing.x);

                            let delete_clicked = ui
                                .with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        let clicked = ui.button("🗑").clicked();
                                        ui.add_space(ui.spacing().item_spacing.x);
                                        self.label_editor(
                                            ui,
                                            node_idx,
                                            node_name,
                                            |idx, value| actions::Action::UpdateStateNodeLabelEditor { node_idx: idx, value },
                                            |idx, new_name| actions::Action::RenameStateNode { node_idx: idx, new_name },
                                        );
                                        clicked
                                    },
                                )
                                .inner;

                            if delete_clicked {
                                self.dispatch(actions::Action::RemoveStateNode { node_idx });
                            }
                        });

                        // Weight editor
                        self.weight_editor(ui, node_idx);

                        // Only show connection info if this node is selected
                        if is_selected {
                            let (incoming, outgoing) =
                                Self::get_connections(
                                    self.store.state.graph.get(),
                                    node_idx,
                                );
                            Self::connections_widget(ui, incoming, outgoing);
                        }
                    }
                });
            });
        });
        });

        // Calculate middle Viridis color for histograms
        let viridis_mid = {
            let c = colorous::VIRIDIS.eval_continuous(0.5);
            egui::Color32::from_rgb(c.r, c.g, c.b)
        };

        // Bottom panel for histograms - takes ~25% of screen height
        let total_height = ctx.available_rect().height();
        let histogram_height = (total_height * 0.25).max(180.0);

        egui::TopBottomPanel::bottom("histogram_panel")
            .exact_height(histogram_height)
            .frame(
                egui::Frame::side_top_panel(&ctx.style())
                    .inner_margin(8.0),
            )
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
                                let state_data = self
                                    .cache
                                    .state_data
                                    .get(&self.store);
                                let plot_height =
                                    ui.available_height() - 30.0;

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
                                let state_data = self
                                    .cache
                                    .state_data
                                    .get(&self.store);

                                if let Some(ref equilibrium_distribution) = state_data.equilibrium_distribution {
                                    let plot_height =
                                        ui.available_height() - 30.0;

                                    render_probability_chart(
                                        ui,
                                        "state_equilibrium_histogram",
                                        "State Equilibrium Distribution",
                                        equilibrium_distribution,
                                        viridis_mid,
                                        plot_height,
                                    );
                                } else {
                                    ui.heading("State Equilibrium Distribution");
                                    ui.separator();
                                    ui.label("Requires valid state graph");
                                }
                            });
                        });

                        // Stats section
                        strip.cell(|ui| {
                            ui.label("Statistics");
                            let state_data = self
                                .cache
                                .state_data
                                .get(&self.store);

                            if let (Some(entropy_rate), Some(detailed_balance_deviation)) =
                                (state_data.entropy_rate, state_data.detailed_balance_deviation) {
                                ui.label(format!(
                                    "Entropy rate: {:.4}",
                                    entropy_rate
                                ));
                                ui.label(format!(
                                    "Balance dev: {:.4}",
                                    detailed_balance_deviation
                                ));
                            } else {
                                ui.label("Entropy rate: N/A");
                                ui.label("Balance dev: N/A");
                            }
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

                        let tab_settings = self
                            .store
                            .layout_settings
                            .dynamical_system
                            .clone();
                        // Update visual parameters if they changed
                        let new_visuals = node_shapes::VisualParams {
                            radius: tab_settings.visuals.node_radius,
                            label_gap: tab_settings.visuals.label_gap,
                            label_font: tab_settings.visuals.label_font_size,
                        };
                        if self.store.state.circular_visuals.get() != &new_visuals {
                            self.store.state.circular_visuals.set(new_visuals);
                        }
                        if self.store.state.label_visibility.get() != &tab_settings.visuals.show_labels {
                            self.store.state.label_visibility.set(tab_settings.visuals.show_labels);
                        }

                        // Sync visual params from Store to node_shapes globals
                        let visuals = self.store.state.circular_visuals.get();
                        node_shapes::set_circular_visual_params(
                            visuals.radius,
                            visuals.label_gap,
                            visuals.label_font,
                        );
                        node_shapes::set_label_visibility(*self.store.state.label_visibility.get());

                        graph_view::set_edge_thickness_bounds(
                            tab_settings.edges.min_width,
                            tab_settings.edges.max_width,
                        );
                        set_loop_radius(
                            tab_settings.layout.loop_radius,
                        );

                        // Reset layout if graph or visual params changed
                        let order = self.cache.state_data.get(&self.store).order.clone();
                        let base_radius = tab_settings.layout.base_radius;
                        let visuals = *self.store.state.circular_visuals.get();
                        let label_visibility = *self.store.state.label_visibility.get();

                        self.store.state.run_if_layout_changed(
                            || {
                                let spacing = SpacingConfig::default().with_fixed_radius(base_radius);
                                layout_circular::set_pending_layout(order.clone(), spacing, visuals, label_visibility);
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
                            self.store.state.graph.get_mut(),
                            sorted_weights,
                        );

                        let settings_interaction = self.get_settings_interaction(mode);
                        let settings_style = self
                            .get_settings_style(
                                tab_settings.visuals.show_labels,
                            );
                        let settings_navigation =
                            self.get_settings_navigation();

                        // Graph takes most of available space, leaving room for controls
                        ui.add(
                            &mut StateGraphView::new(self.store.state.graph.get_mut())
                                .with_interactions(&settings_interaction)
                                .with_navigations(&settings_navigation)
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
        // Left panel fixed at 25% of the screen width to match the dynamical tab
        let screen_width = ctx.viewport_rect().width().max(1.0);
        let left_panel_width = screen_width * 0.25;

        // Remaining panels still use a 3-way split for the rest of the layout
        let available_width = ctx.available_rect().width();
        let right_panel_width = available_width / 3.0;

        // Left panel: Destination node management
        egui::SidePanel::left("observable_left_panel")
            .exact_width(left_panel_width)
            .resizable(false)
            .frame(egui::Frame::side_top_panel(&ctx.style()).inner_margin(8.0))
            .show(ctx, |panel_ui| {
                egui::TopBottomPanel::bottom("observable_left_footer")
                    .resizable(false)
                    .frame(egui::Frame::NONE)
                    .show_inside(panel_ui, |ui| {
                        let validation_errors = self.cache.observable_data.get(&self.store).validation_errors.clone();
                        self.render_observable_validation_panel(
                            ui,
                            &validation_errors,
                        );
                        ui.add_space(6.0);
                        self.layout_settings_panel(
                            ui,
                            ActiveTab::ObservableEditor,
                        );
                    });
                egui::CentralPanel::default()
                    .frame(egui::Frame::NONE)
                    .show_inside(panel_ui, |ui| {
                        ui.vertical(|ui| {
                    ui.heading("Observable Values");
                    ui.separator();

                    // Add Destination button
                    if ui.button("Add Value").clicked() {
                        let node_count = self.store.observable.graph
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
                                .store.observable.graph
                                .get()
                                .nodes_iter()
                                .filter(|(_, node)| node.payload().node_type == ObservableNodeType::Destination)
                                .map(|(idx, node)| (idx, node.payload().name.clone()))
                                .collect();

                            for (node_idx, node_name) in dest_nodes {
                                let is_selected = self
                                    .store.observable.graph
                                    .get()
                                    .node(node_idx)
                                    .map(|n| n.selected())
                                    .unwrap_or(false);
                                let all_nodes: Vec<_> = self
                                    .store
                                    .observable.graph
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

                                    ui.add_space(ui.spacing().item_spacing.x);

                                    let delete_clicked = ui
                                        .with_layout(
                                            egui::Layout::right_to_left(
                                                egui::Align::Center,
                                            ),
                                            |ui| {
                                                let clicked = ui.button("🗑").clicked();
                                                ui.add_space(
                                                    ui.spacing().item_spacing.x,
                                                );
                                                self.label_editor(
                                                    ui,
                                                    node_idx,
                                                    node_name,
                                                    |idx, value| actions::Action::UpdateObservableDestinationNodeLabelEditor { node_idx: idx, value },
                                                    |idx, new_name| actions::Action::RenameObservableDestinationNode { node_idx: idx, new_name },
                                                );
                                                clicked
                                            },
                                        )
                                        .inner;

                                    if delete_clicked {
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
                                            self.store.observable.graph.get(),
                                            node_idx,
                                        );
                                    Self::connections_widget(ui, incoming, vec![]);
                                }
                            }
                        });
                });
            });
            });

        // Right panel: Heatmap
        egui::SidePanel::right("observable_right_panel")
            .exact_width(right_panel_width)
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

                    let tab_settings = self
                        .store
                        .layout_settings
                        .observable_editor
                        .clone();
                    // Update visual parameters if they changed
                    let new_visuals = node_shapes::VisualParams {
                        radius: tab_settings.visuals.node_radius,
                        label_gap: tab_settings.visuals.label_gap,
                        label_font: tab_settings.visuals.label_font_size,
                    };
                    if self.store.observable.bipartite_visuals.get() != &new_visuals {
                        self.store.observable.bipartite_visuals.set(new_visuals);
                    }
                    if self.store.observable.label_visibility.get() != &tab_settings.visuals.show_labels {
                        self.store.observable.label_visibility.set(tab_settings.visuals.show_labels);
                    }

                    // Sync visual params from Store to node_shapes globals
                    let visuals = self.store.observable.bipartite_visuals.get();
                    node_shapes::set_bipartite_visual_params(
                        visuals.radius,
                        visuals.label_gap,
                        visuals.label_font,
                    );
                    node_shapes::set_label_visibility(*self.store.observable.label_visibility.get());

                    graph_view::set_edge_thickness_bounds(
                        tab_settings.edges.min_width,
                        tab_settings.edges.max_width,
                    );

                    // Reset layout if graph or visual params changed
                    let visuals = *self.store.observable.bipartite_visuals.get();
                    let label_visibility = *self.store.observable.label_visibility.get();

                    self.store.observable.run_if_layout_changed(
                        || {
                            let spacing = layout_bipartite::BipartiteSpacingConfig {
                                node_gap: tab_settings.layout.node_gap,
                                layer_gap: tab_settings.layout.layer_gap,
                            };
                            layout_bipartite::set_pending_layout(spacing, visuals, label_visibility);
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
                        self.store.observable.graph.get_mut(),
                        sorted_weights,
                    );

                    let settings_interaction =
                        self.get_settings_interaction(mode);
                    let settings_style = self
                        .get_settings_style(
                            tab_settings.visuals.show_labels,
                        );
                    let settings_navigation =
                        self.get_settings_navigation();

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
                                    self.store.observable.graph.get_mut(),
                                )
                                .with_interactions(
                                    &settings_interaction,
                                )
                                .with_navigations(
                                    &settings_navigation,
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
                    ui.separator();
                });
            });
    }

    fn render_observed_dynamics_tab(&mut self, ctx: &egui::Context) {
        let screen_width = ctx.viewport_rect().width().max(1.0);
        let left_panel_width = screen_width * 0.25;

        let total_height = ctx.available_rect().height();
        let histogram_height = (total_height * 0.25).max(180.0);
        let observed_color = egui::Color32::from_rgb(250, 150, 100);

        egui::SidePanel::left("observed_left_panel")
            .exact_width(left_panel_width)
            .resizable(false)
            .frame(egui::Frame::side_top_panel(&ctx.style()).inner_margin(8.0))
            .show(ctx, |panel_ui| {
                egui::TopBottomPanel::bottom("observed_left_footer")
                    .resizable(false)
                    .frame(egui::Frame::NONE)
                    .show_inside(panel_ui, |ui| {
                        let state_validation_errors = self.cache.state_data.get(&self.store).validation_errors.clone();
                        let observable_validation_errors = self.cache.observable_data.get(&self.store).validation_errors.clone();

                        self.render_state_validation_panel(
                            ui,
                            &state_validation_errors,
                        );
                        self.render_observable_validation_panel(
                            ui,
                            &observable_validation_errors,
                        );

                        ui.add_space(6.0);
                        self.layout_settings_panel(
                            ui,
                            ActiveTab::ObservedDynamics,
                        );
                    });

                egui::CentralPanel::default()
                    .frame(egui::Frame::NONE)
                    .show_inside(panel_ui, |ui| {
                        ui.heading("Observed Values");
                        ui.separator();

                        let list_height = ui.available_height() - 40.0;
                        egui::ScrollArea::vertical()
                            .max_height(list_height)
                            .show(ui, |ui| {
                                let nodes: Vec<_> = {
                                    let observed_data =
                                        self.cache.observed_data.get(&self.store);
                                    observed_data
                                        .graph
                                        .nodes_iter()
                                        .map(|(idx, node)| {
                                            (
                                                idx,
                                                node.payload().name.clone(),
                                                node.payload().weight,
                                                node.selected(),
                                            )
                                        })
                                        .collect()
                                };

                                let all_nodes: Vec<_> =
                                    nodes.iter().map(|(idx, _, _, _)| *idx).collect();

                                for (node_idx, node_name, weight, is_selected) in nodes {
                                    let connection_data = if is_selected {
                                        let observed_data =
                                            self.cache.observed_data.get(&self.store);
                                        Some(Self::get_connections(
                                            &observed_data.graph,
                                            node_idx,
                                        ))
                                    } else {
                                        None
                                    };

                                    ui.horizontal(|ui| {
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
                                        ui.label(&node_name);
                                    });

                                    ui.horizontal(|ui| {
                                        ui.label("Weight:");
                                        ui.label(format!("{:.4}", weight));
                                    });

                                    if let Some((incoming, outgoing)) = connection_data {
                                        Self::connections_widget(
                                            ui,
                                            incoming,
                                            outgoing,
                                        );
                                    }
                                }
                            });

                        if let Some((node_idx, selected)) =
                            self.store.observed_node_selection.take()
                        {
                            let observed_data =
                                self.cache.observed_data.get_mut(&self.store);
                            if let Some(node) =
                                observed_data.graph.node_mut(node_idx)
                            {
                                node.set_selected(selected);
                            }
                        }
                    });
            });

        egui::TopBottomPanel::bottom("observed_histogram_panel")
            .exact_height(histogram_height)
            .frame(
                egui::Frame::side_top_panel(&ctx.style())
                    .inner_margin(8.0),
            )
            .show(ctx, |ui| {
                let observed_data = self.cache.observed_data.get(&self.store);
                StripBuilder::new(ui)
                    .size(Size::remainder().at_least(200.0))
                    .size(Size::remainder().at_least(200.0))
                    .size(Size::remainder().at_least(200.0))
                    .size(Size::remainder().at_least(150.0))
                    .horizontal(|mut strip| {
                        strip.cell(|ui| {
                            ui.vertical(|ui| {
                                let plot_height = ui.available_height() - 30.0;
                                render_probability_chart(
                                    ui,
                                    "observed_weight_distribution",
                                    "Observed Weight Distribution",
                                    &observed_data.weight_distribution,
                                    observed_color,
                                    plot_height,
                                );
                            });
                        });

                        strip.cell(|ui| {
                            ui.vertical(|ui| {
                                if let Some(ref equilibrium_from_state) = observed_data.equilibrium_from_state {
                                    let plot_height = ui.available_height() - 30.0;
                                    render_probability_chart(
                                        ui,
                                        "observed_equilibrium_from_state",
                                        "Observed Equilibrium",
                                        equilibrium_from_state,
                                        observed_color,
                                        plot_height,
                                    );
                                } else {
                                    ui.heading("Observed Equilibrium");
                                    ui.separator();
                                    ui.label("Requires valid state and observable graphs");
                                }
                            });
                        });

                        strip.cell(|ui| {
                            ui.vertical(|ui| {
                                if let Some(ref equilibrium_calculated) = observed_data.equilibrium_calculated {
                                    let plot_height = ui.available_height() - 30.0;
                                    render_probability_chart(
                                        ui,
                                        "observed_equilibrium_calculated",
                                        "Calculated Equilibrium",
                                        equilibrium_calculated,
                                        observed_color,
                                        plot_height,
                                    );
                                } else {
                                    ui.heading("Calculated Equilibrium");
                                    ui.separator();
                                    ui.label("Requires valid state and observable graphs");
                                }
                            });
                        });
                        strip.cell(|ui| {
                            ui.vertical(|ui| {
                                if let (Some(entropy_rate), Some(detailed_balance_deviation)) =
                                    (observed_data.entropy_rate, observed_data.detailed_balance_deviation) {
                                    ui.label(format!(
                                        "Entropy rate: {:.4}",
                                        entropy_rate
                                    ));
                                    ui.label(format!(
                                        "Detailed balance deviation: {:.4}",
                                        detailed_balance_deviation
                                    ));
                                } else {
                                    ui.label("Entropy rate: N/A");
                                    ui.label("Detailed balance deviation: N/A");
                                }
                            });
                        });
                    });
            });

        egui::CentralPanel::default()
            .frame(
                egui::Frame::central_panel(&ctx.style())
                    .inner_margin(8.0),
            )
            .show(ctx, |ui| {
                StripBuilder::new(ui)
                    .size(Size::remainder())
                    .size(Size::remainder())
                    .horizontal(|mut strip| {
                        strip.cell(|ui| {
                            ui.heading("Observed Graph");
                            ui.separator();

                            let tab_settings = self
                                .store
                                .layout_settings
                                .observed_dynamics
                                .clone();
                            // Update visual parameters if they changed
                            let new_visuals = node_shapes::VisualParams {
                                radius: tab_settings.visuals.node_radius,
                                label_gap: tab_settings.visuals.label_gap,
                                label_font: tab_settings.visuals.label_font_size,
                            };
                            if self.store.observed.circular_visuals.get() != &new_visuals {
                                self.store.observed.circular_visuals.set(new_visuals);
                            }
                            if self.store.observed.label_visibility.get() != &tab_settings.visuals.show_labels {
                                self.store.observed.label_visibility.set(tab_settings.visuals.show_labels);
                            }

                            // Sync visual params from Store to node_shapes globals
                            let visuals = self.store.observed.circular_visuals.get();
                            node_shapes::set_circular_visual_params(
                                visuals.radius,
                                visuals.label_gap,
                                visuals.label_font,
                            );
                            node_shapes::set_label_visibility(*self.store.observed.label_visibility.get());

                            graph_view::set_edge_thickness_bounds(
                                tab_settings.edges.min_width,
                                tab_settings.edges.max_width,
                            );
                            set_loop_radius(tab_settings.layout.loop_radius);

                            let settings_interaction =
                                SettingsInteraction::new()
                                    .with_dragging_enabled(false)
                                    .with_node_clicking_enabled(true)
                                    .with_node_selection_enabled(true);
                            let settings_style = self.get_settings_style(
                                tab_settings.visuals.show_labels,
                            );
                            let settings_navigation =
                                self.get_settings_navigation();

                            let observed_version =
                                self.cache.observed_data.version();
                            let observed_data =
                                self.cache.observed_data.get_mut(&self.store);
                            let order = observed_data.order.clone();
                            let base_radius = tab_settings.layout.base_radius;
                            let visuals = *self.store.observed.circular_visuals.get();
                            let label_visibility = *self.store.observed.label_visibility.get();

                            self.store
                                .observed
                                .run_if_layout_changed(
                                    observed_version,
                                    || {
                                        let spacing = SpacingConfig::default().with_fixed_radius(base_radius);
                                        layout_circular::set_pending_layout(order.clone(), spacing, visuals, label_visibility);
                                        reset_layout::<LayoutStateCircular>(ui, None);
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
                                        .with_navigations(
                                            &settings_navigation,
                                        )
                                        .with_styles(&settings_style),
                                    );
                                },
                            );

                        });

                        strip.cell(|ui| {
                            ui.heading("Observed Heatmap");
                            ui.separator();

                            let (
                                x_labels,
                                y_labels,
                                matrix,
                                x_node_indices,
                                y_node_indices,
                            ) = self
                                .cache
                                .observed_data
                                .get(&self.store)
                                .heatmap
                                .clone();

                            let editing_state = heatmap::EditingState {
                                editing_cell: None,
                                edit_buffer: String::new(),
                            };

                            let (new_hover, _, _) = heatmap::show_heatmap(
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
                        });
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
                .state.graph
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

            let response = ui
                .with_layout(
                    egui::Layout::right_to_left(egui::Align::Center),
                    |ui| {
                        ui.add_space(ui.spacing().item_spacing.x);
                        let text_width =
                            ui.available_width().max(80.0);
                        ui.add(
                            egui::TextEdit::singleline(
                                &mut weight_str,
                            )
                            .desired_width(text_width),
                        )
                    },
                )
                .inner;
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
        let text_width = ui.available_width().max(80.0);
        let response = ui.add(
            egui::TextEdit::singleline(&mut name_str)
                .desired_width(text_width),
        );
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

    fn layout_settings_panel(
        &mut self,
        ui: &mut egui::Ui,
        tab: ActiveTab,
    ) {
        egui::CollapsingHeader::new("Layout settings")
            .default_open(false)
            .show(ui, |ui| match tab {
                ActiveTab::DynamicalSystem => {
                    let settings = self
                        .store
                        .layout_settings
                        .dynamical_system
                        .clone();
                    self.render_circular_layout_controls(
                        ui, tab, settings,
                    );
                }
                ActiveTab::ObservedDynamics => {
                    let settings = self
                        .store
                        .layout_settings
                        .observed_dynamics
                        .clone();
                    self.render_circular_layout_controls(
                        ui, tab, settings,
                    );
                }
                ActiveTab::ObservableEditor => {
                    let settings = self
                        .store
                        .layout_settings
                        .observable_editor
                        .clone();
                    self.render_bipartite_layout_controls(
                        ui, tab, settings,
                    );
                }
            });

        if matches!(
            tab,
            ActiveTab::DynamicalSystem | ActiveTab::ObservableEditor
        ) {
            ui.add_space(4.0);
            ui.label("Hold Ctrl for Edge Editor");
            ui.label("Release Ctrl for Node Editor");
        }
    }

    fn render_circular_layout_controls(
        &mut self,
        ui: &mut egui::Ui,
        tab: ActiveTab,
        settings: layout_settings::CircularTabLayoutSettings,
    ) {
        ui.label("Node visuals");
        self.layout_slider(
            ui,
            tab,
            "Node radius",
            settings.visuals.node_radius,
            NODE_RADIUS_RANGE,
            actions::LayoutSettingChange::NodeRadius,
            false,
        );
        self.layout_slider(
            ui,
            tab,
            "Label gap",
            settings.visuals.label_gap,
            LABEL_GAP_RANGE,
            actions::LayoutSettingChange::LabelGap,
            false,
        );
        self.layout_slider(
            ui,
            tab,
            "Label font size",
            settings.visuals.label_font_size,
            LABEL_FONT_RANGE,
            actions::LayoutSettingChange::LabelFontSize,
            false,
        );
        self.show_label_toggle(ui, tab, settings.visuals.show_labels);

        ui.separator();
        ui.label("Edges");
        self.layout_slider(
            ui,
            tab,
            "Min width",
            settings.edges.min_width,
            EDGE_THICKNESS_MIN_RANGE,
            actions::LayoutSettingChange::EdgeMinWidth,
            false,
        );
        self.layout_slider(
            ui,
            tab,
            "Max width",
            settings.edges.max_width,
            EDGE_THICKNESS_MAX_RANGE,
            actions::LayoutSettingChange::EdgeMaxWidth,
            false,
        );
        self.layout_slider(
            ui,
            tab,
            "Self-loop radius",
            settings.layout.loop_radius,
            LOOP_RADIUS_RANGE,
            actions::LayoutSettingChange::LoopRadius,
            false,
        );

        ui.separator();
        ui.label("Layout");
        self.layout_slider(
            ui,
            tab,
            "Ring radius",
            settings.layout.base_radius,
            CIRCULAR_BASE_RADIUS_RANGE,
            actions::LayoutSettingChange::CircularBaseRadius,
            true,
        );
    }

    fn render_bipartite_layout_controls(
        &mut self,
        ui: &mut egui::Ui,
        tab: ActiveTab,
        settings: layout_settings::BipartiteTabLayoutSettings,
    ) {
        ui.label("Node visuals");
        self.layout_slider(
            ui,
            tab,
            "Node radius",
            settings.visuals.node_radius,
            NODE_RADIUS_RANGE,
            actions::LayoutSettingChange::NodeRadius,
            false,
        );
        self.layout_slider(
            ui,
            tab,
            "Label gap",
            settings.visuals.label_gap,
            LABEL_GAP_RANGE,
            actions::LayoutSettingChange::LabelGap,
            false,
        );
        self.layout_slider(
            ui,
            tab,
            "Label font size",
            settings.visuals.label_font_size,
            LABEL_FONT_RANGE,
            actions::LayoutSettingChange::LabelFontSize,
            false,
        );
        self.show_label_toggle(ui, tab, settings.visuals.show_labels);

        ui.separator();
        ui.label("Edges");
        self.layout_slider(
            ui,
            tab,
            "Min width",
            settings.edges.min_width,
            EDGE_THICKNESS_MIN_RANGE,
            actions::LayoutSettingChange::EdgeMinWidth,
            false,
        );
        self.layout_slider(
            ui,
            tab,
            "Max width",
            settings.edges.max_width,
            EDGE_THICKNESS_MAX_RANGE,
            actions::LayoutSettingChange::EdgeMaxWidth,
            false,
        );

        ui.separator();
        ui.label("Layout");
        self.layout_slider(
            ui,
            tab,
            "Layer gap",
            settings.layout.layer_gap,
            BIPARTITE_LAYER_GAP_RANGE,
            actions::LayoutSettingChange::BipartiteLayerGap,
            true,
        );
        self.layout_slider(
            ui,
            tab,
            "Node spacing",
            settings.layout.node_gap,
            BIPARTITE_NODE_GAP_RANGE,
            actions::LayoutSettingChange::BipartiteNodeGap,
            true,
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn layout_slider(
        &mut self,
        ui: &mut egui::Ui,
        tab: ActiveTab,
        label: &str,
        value: f32,
        range: layout_settings::SliderRange,
        change: impl Fn(f32) -> actions::LayoutSettingChange,
        requires_layout_reset: bool,
    ) {
        let mut slider_value = value;
        let response = ui.add(
            egui::Slider::new(
                &mut slider_value,
                range.min..=range.max,
            )
            .text(label)
            .step_by(range.step as f64),
        );
        if response.changed() {
            self.dispatch(actions::Action::UpdateLayoutSetting {
                tab,
                change: change(slider_value),
            });
            if requires_layout_reset {
                self.reset_layout_for_tab(ui, tab);
            }
        }
    }

    fn show_label_toggle(
        &mut self,
        ui: &mut egui::Ui,
        tab: ActiveTab,
        value: bool,
    ) {
        let mut show_labels = value;
        if ui.checkbox(&mut show_labels, "Show labels").changed() {
            self.dispatch(actions::Action::UpdateLayoutSetting {
                tab,
                change: actions::LayoutSettingChange::ShowLabels(
                    show_labels,
                ),
            });
        }
    }

    fn reset_layout_for_tab(
        &self,
        ui: &mut egui::Ui,
        tab: ActiveTab,
    ) {
        match tab {
            ActiveTab::ObservableEditor => {
                reset_layout::<LayoutStateBipartite>(ui, None);
            }
            ActiveTab::DynamicalSystem
            | ActiveTab::ObservedDynamics => {
                reset_layout::<LayoutStateCircular>(ui, None);
            }
        }
    }
}
