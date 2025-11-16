use eframe::egui;
use petgraph::stable_graph::NodeIndex;

pub type HeatmapData = (
    Vec<String>,           // x_labels
    Vec<String>,           // y_labels
    Vec<Vec<Option<f64>>>, // matrix
    Vec<NodeIndex>,        // x_node_indices
    Vec<NodeIndex>,        // y_node_indices
);

// Color scale configuration constants
const COLOR_SCALE_MESH_POINTS: usize = 10;
const COLOR_SCALE_HEIGHT: f32 = 30.0;
const COLOR_SCALE_LABEL_HEIGHT: f32 = 15.0;

#[derive(Debug, Clone)]
pub struct WeightChange {
    pub source_idx: NodeIndex,
    pub target_idx: NodeIndex,
    pub new_weight: f64,
}

pub struct EditingState {
    pub editing_cell: Option<(usize, usize)>,
    pub edit_buffer: String,
}

/// Convert a normalized value [0.0, 1.0] to a Viridis color
fn viridis(t: f64) -> egui::Color32 {
    let c =
        colorous::VIRIDIS.eval_continuous(t.clamp(0.0, 1.0));
    egui::Color32::from_rgb(c.r, c.g, c.b)
}

/// Calculate contrasting text color for given background
/// Returns black for light backgrounds, white for dark backgrounds
fn contrasting_text_color(bg: egui::Color32) -> egui::Color32 {
    // Calculate relative luminance using sRGB
    let r = bg.r() as f64 / 255.0;
    let g = bg.g() as f64 / 255.0;
    let b = bg.b() as f64 / 255.0;

    // Simplified luminance calculation
    let luminance = 0.299 * r + 0.587 * g + 0.114 * b;

    // Use black text for bright backgrounds (luminance > 0.5)
    // Use white text for dark backgrounds
    if luminance > 0.5 {
        egui::Color32::BLACK
    } else {
        egui::Color32::WHITE
    }
}

/// Calculate color interpolation value based on weight position in sorted list
/// Returns value between 0.0 and 1.0 for the Inferno color gradient
/// Handles any weight value by finding where it fits in the sorted list
fn calculate_color_position(
    weight: f64,
    sorted_weights: &[f64],
) -> f64 {
    if sorted_weights.is_empty() {
        return 0.5; // Middle color when no weights
    }

    if sorted_weights.len() == 1 {
        return 0.5; // Middle color when only one weight
    }

    // Find where this weight fits in the sorted list
    // For weights not in the list, interpolate between surrounding indices

    // Find first index where sorted_weights[i] >= weight
    let mut insert_pos = sorted_weights.len();
    for (i, &w) in sorted_weights.iter().enumerate() {
        if w >= weight {
            insert_pos = i;
            break;
        }
    }

    // Handle edge cases
    if insert_pos == 0 {
        // Weight is less than or equal to minimum
        return 0.0;
    }
    if insert_pos >= sorted_weights.len() {
        // Weight is greater than maximum
        return 1.0;
    }

    // Weight falls between sorted_weights[insert_pos - 1] and sorted_weights[insert_pos]
    // Check if weight exactly matches a value
    if (sorted_weights[insert_pos] - weight).abs() < 1e-6 {
        // Find all occurrences of this weight and use middle
        let mut first_idx = insert_pos;
        let mut last_idx = insert_pos;

        // Find first occurrence
        while first_idx > 0
            && (sorted_weights[first_idx - 1] - weight).abs() < 1e-6
        {
            first_idx -= 1;
        }

        // Find last occurrence
        while last_idx < sorted_weights.len() - 1
            && (sorted_weights[last_idx + 1] - weight).abs() < 1e-6
        {
            last_idx += 1;
        }

        let middle_idx = (first_idx + last_idx) / 2;
        return middle_idx as f64 / (sorted_weights.len() - 1) as f64;
    }

    // Weight doesn't match exactly - interpolate position between indices
    let lower_idx = insert_pos - 1;
    let upper_idx = insert_pos;
    let lower_weight = sorted_weights[lower_idx];
    let upper_weight = sorted_weights[upper_idx];

    // Linear interpolation of the index position
    let ratio =
        (weight - lower_weight) / (upper_weight - lower_weight);
    let interpolated_idx = lower_idx as f64 + ratio;

    interpolated_idx / (sorted_weights.len() - 1) as f64
}

/// Render a horizontal color scale showing the Inferno gradient with uniformly spaced weight values
fn render_color_scale(
    ui: &mut egui::Ui,
    sorted_weights: &[f64],
    scale_width: f32,
) {
    if sorted_weights.is_empty() {
        return;
    }

    // Get min and max weights for uniform spacing
    let min_weight = sorted_weights[0];
    let max_weight = sorted_weights[sorted_weights.len() - 1];

    if (max_weight - min_weight).abs() < 1e-6 {
        return; // All weights are the same, don't show scale
    }

    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing = egui::Vec2::splat(0.0);

        // Create mesh for gradient bar
        let (rect, _response) = ui.allocate_exact_size(
            egui::Vec2::new(scale_width, COLOR_SCALE_HEIGHT),
            egui::Sense::hover(),
        );
        let rect_pos = rect.min;

        let mut mesh = egui::Mesh::default();

        // Create vertices for horizontal gradient with uniform weight spacing
        for i in 0..COLOR_SCALE_MESH_POINTS {
            let t = i as f32 / (COLOR_SCALE_MESH_POINTS - 1) as f32;
            let x = rect_pos.x + t * scale_width;

            // Calculate uniform weight value at this position
            let weight = min_weight + (t as f64) * (max_weight - min_weight);

            // Get color for this weight value by looking up position in sorted list
            let color_t =
                calculate_color_position(weight, sorted_weights);
            let color = viridis(color_t);

            // Top vertex
            mesh.colored_vertex(egui::pos2(x, rect_pos.y), color);

            // Bottom vertex
            mesh.colored_vertex(
                egui::pos2(x, rect_pos.y + COLOR_SCALE_HEIGHT),
                color,
            );
        }

        // Create triangle strip indices
        for i in 0..(COLOR_SCALE_MESH_POINTS - 1) {
            let base = (i * 2) as u32;
            // First triangle
            mesh.add_triangle(base, base + 1, base + 2);
            // Second triangle
            mesh.add_triangle(base + 1, base + 3, base + 2);
        }

        ui.painter().add(egui::Shape::mesh(mesh));

        // Add tick marks and labels at 5 uniformly spaced weight positions
        let label_positions: [f32; 5] = [0.0, 0.25, 0.5, 0.75, 1.0];

        ui.allocate_space(egui::Vec2::new(
            scale_width,
            COLOR_SCALE_LABEL_HEIGHT,
        ));

        for &pos in &label_positions {
            let x = rect_pos.x + pos * scale_width;

            // Calculate uniform weight value at this position
            let weight = min_weight + (pos as f64) * (max_weight - min_weight);

            // Draw tick mark
            let tick_top = rect_pos.y + COLOR_SCALE_HEIGHT;
            let tick_bottom = tick_top + 5.0;
            ui.painter().line_segment(
                [egui::pos2(x, tick_top), egui::pos2(x, tick_bottom)],
                egui::Stroke::new(1.0, egui::Color32::DARK_GRAY),
            );

            // Draw label
            let text = format!("{:.1}", weight);
            let font_id = egui::FontId::proportional(9.0);
            let label_y = tick_bottom + 2.0;
            ui.painter().text(
                egui::pos2(x, label_y),
                egui::Align2::CENTER_TOP,
                text,
                font_id,
                egui::Color32::DARK_GRAY,
            );
        }
    });
}

/// Render a heatmap visualization of a directed graph's adjacency matrix with inline editing
/// Returns (new_hovered_cell, new_editing_state, optional_weight_change)
#[allow(clippy::too_many_arguments)]
pub fn show_heatmap(
    ui: &mut egui::Ui,
    x_labels: &[String],
    y_labels: &[String],
    matrix: &[Vec<Option<f64>>],
    x_node_indices: &[NodeIndex], // Maps x position (columns) to target NodeIndex
    y_node_indices: &[NodeIndex], // Maps y position (rows) to source NodeIndex
    prev_hovered_cell: Option<(usize, usize)>,
    editing_state: EditingState,
) -> (Option<(usize, usize)>, EditingState, Option<WeightChange>) {
    if x_labels.is_empty() || y_labels.is_empty() {
        ui.label("No nodes to display");
        return (None, editing_state, None);
    }

    // Collect all non-zero weights for color interpolation
    // Missing edges (None values) are rendered as empty cells
    let mut sorted_weights: Vec<f64> = matrix
        .iter()
        .flat_map(|row| row.iter())
        .filter_map(|&w| w)
        .filter(|&w| w > 0.0)
        .collect();

    sorted_weights.sort_by(|a, b| {
        a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal)
    });

    // Prepend 0.0 to the sorted list for color scaling
    sorted_weights.insert(0, 0.0);

    let available_rect = ui.available_rect_before_wrap();
    let spacing = 2.0;
    let label_height = 20.0;
    let label_width = 60.0;

    let available_width =
        available_rect.width() - label_width - spacing;
    let available_height = available_rect.height()
        - label_height
        - spacing
        - 10.0
        - COLOR_SCALE_HEIGHT
        - COLOR_SCALE_LABEL_HEIGHT
        - 5.0;

    let cell_width = available_width / x_labels.len() as f32;
    let cell_height = available_height / y_labels.len() as f32;
    let cell_size = cell_width.min(cell_height).max(10.0);

    let new_hovered_cell =
        std::cell::RefCell::new(None::<(usize, usize)>);
    let mut new_editing_cell = editing_state.editing_cell;
    let mut new_edit_buffer = editing_state.edit_buffer;
    let mut weight_change = None;

    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing = egui::Vec2::ZERO;

        // Top row: X-axis labels (column headers)
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = egui::Vec2::ZERO;
            ui.add_space(label_width);

            for (x_idx, label) in x_labels.iter().enumerate() {
                let is_highlighted = prev_hovered_cell.map(|(hx, _)| hx == x_idx).unwrap_or(false);
                let text_color = if is_highlighted {
                    egui::Color32::from_rgb(255, 255, 255)
                } else {
                    ui.style().visuals.text_color()
                };

                ui.add_sized(
                    [cell_size, label_height],
                    egui::Label::new(
                        egui::RichText::new(label.as_str())
                            .color(text_color)
                            .size(10.0)
                    )
                );
            }
        });

        for y_idx in 0..y_labels.len() {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing = egui::Vec2::ZERO;

                let is_y_highlighted = prev_hovered_cell.map(|(_, hy)| hy == y_idx).unwrap_or(false);
                let text_color = if is_y_highlighted {
                    egui::Color32::from_rgb(255, 255, 255)
                } else {
                    ui.style().visuals.text_color()
                };

                ui.add_sized(
                    [label_width, cell_size],
                    egui::Label::new(
                        egui::RichText::new(y_labels[y_idx].as_str())
                            .color(text_color)
                            .size(10.0)
                    )
                );

                for (x_idx, weight_opt) in matrix[y_idx].iter().enumerate() {
                    let is_editing = new_editing_cell == Some((x_idx, y_idx));

                    if is_editing {
                        // Determine background color based on weight value;
                        // treat None as zero weight drawn via Viridis.
                        let cell_color = {
                            let weight = weight_opt.unwrap_or(0.0);
                            let t = calculate_color_position(
                                weight,
                                &sorted_weights,
                            );
                            viridis(t)
                        };

                        // Render text edit widget
                        let (rect, _) = ui.allocate_exact_size(
                            egui::Vec2::new(cell_size, cell_size),
                            egui::Sense::click(),
                        );

                        // Draw background for editing cell
                        ui.painter().rect_filled(
                            rect,
                            0.0,
                            cell_color,
                        );

                        // Calculate centered position for text edit
                        let text_height = 14.0_f32.min(cell_size * 0.8); // Ensure text height fits in cell
                        let y_offset = (cell_size - text_height) / 2.0;
                        let padding = 4.0_f32.min(cell_size * 0.1);
                        let inner_rect = egui::Rect::from_min_size(
                            egui::pos2(rect.min.x + padding, rect.min.y + y_offset.max(0.0)),
                            egui::vec2((cell_size - 2.0 * padding).max(1.0), text_height.max(1.0)),
                        );

                        let mut child_ui = ui.new_child(
                            egui::UiBuilder::new()
                                .max_rect(inner_rect)
                                .layout(egui::Layout::centered_and_justified(egui::Direction::TopDown))
                        );

                        // Use style override to make text edit background transparent and remove borders
                        child_ui.style_mut().visuals.extreme_bg_color = egui::Color32::TRANSPARENT;
                        child_ui.style_mut().visuals.widgets.inactive.bg_fill = egui::Color32::TRANSPARENT;
                        child_ui.style_mut().visuals.widgets.hovered.bg_fill = egui::Color32::TRANSPARENT;
                        child_ui.style_mut().visuals.widgets.active.bg_fill = egui::Color32::TRANSPARENT;
                        child_ui.style_mut().visuals.widgets.inactive.bg_stroke = egui::Stroke::NONE;
                        child_ui.style_mut().visuals.widgets.hovered.bg_stroke = egui::Stroke::NONE;
                        child_ui.style_mut().visuals.widgets.active.bg_stroke = egui::Stroke::NONE;

                        let text_edit = egui::TextEdit::singleline(&mut new_edit_buffer)
                            .desired_width(cell_size - 8.0)
                            .font(egui::FontId::proportional(10.0))
                            .horizontal_align(egui::Align::Center);

                        let te_response = child_ui.add(text_edit);
                        te_response.request_focus();

                        {
                            let enter_pressed = ui.input(|i| i.key_pressed(egui::Key::Enter));
                            let tab_pressed = ui.input(|i| i.key_pressed(egui::Key::Tab));
                            let escape_pressed = ui.input(|i| i.key_pressed(egui::Key::Escape));

                            // Handle Enter key - commit and exit editing
                            if enter_pressed {
                                if let Ok(parsed_weight) = new_edit_buffer.parse::<f64>() {
                                    weight_change = Some(WeightChange {
                                        source_idx: y_node_indices[y_idx],
                                        target_idx: x_node_indices[x_idx],
                                        new_weight: parsed_weight,
                                    });
                                }
                                new_editing_cell = None;
                                new_edit_buffer.clear();
                            }
                            // Handle Tab key - commit and move to next cell
                            else if tab_pressed {
                                if let Ok(parsed_weight) = new_edit_buffer.parse::<f64>() {
                                    weight_change = Some(WeightChange {
                                        source_idx: y_node_indices[y_idx],
                                        target_idx: x_node_indices[x_idx],
                                        new_weight: parsed_weight,
                                    });
                                }

                                // Move to next cell (left to right, then down)
                                let next_x = if x_idx + 1 < x_labels.len() {
                                    x_idx + 1
                                } else {
                                    0
                                };
                                let next_y = if next_x == 0 && y_idx > 0 {
                                    y_idx - 1
                                } else {
                                    y_idx
                                };

                                new_editing_cell = Some((next_x, next_y));
                                new_edit_buffer = matrix[next_y][next_x]
                                    .map(|w| w.to_string())
                                    .unwrap_or_default();
                            }
                            // Handle Escape key or clicking outside - cancel editing
                            else if escape_pressed || (te_response.lost_focus() && !te_response.has_focus()) {
                                new_editing_cell = None;
                                new_edit_buffer.clear();
                            }
                        }
                    } else {
                        // Normal cell rendering; None draws as zero weight.
                        let cell_color = {
                            let weight = weight_opt.unwrap_or(0.0);
                            let t = calculate_color_position(
                                weight,
                                &sorted_weights,
                            );
                            viridis(t)
                        };

                        let (rect, response) = ui.allocate_exact_size(
                            egui::Vec2::new(cell_size, cell_size),
                            egui::Sense::click(),
                        );

                        if response.hovered() {
                            *new_hovered_cell.borrow_mut() = Some((x_idx, y_idx));
                        }

                        // Start editing on click
                        if response.clicked() {
                            new_editing_cell = Some((x_idx, y_idx));
                            new_edit_buffer = weight_opt
                                .map(|w| w.to_string())
                                .unwrap_or_default();
                        }

                        let is_hovered = response.hovered();
                        let final_color = if is_hovered {
                            egui::Color32::from_rgb(
                                cell_color.r().saturating_add(40),
                                cell_color
                                    .g()
                                    .saturating_add(40),
                                cell_color
                                    .b()
                                    .saturating_add(40),
                            )
                        } else {
                            cell_color
                        };

                        ui.painter().rect_filled(rect, 0.0, final_color);
                        ui.painter().rect_stroke(
                            rect,
                            0.0,
                            egui::Stroke::new(0.5, egui::Color32::from_gray(40)),
                            egui::epaint::StrokeKind::Outside,
                        );

                        if let Some(weight) = weight_opt
                            && *weight > 0.0 {
                                let text = format!("{:.1}", weight);
                                let font_id = egui::FontId::proportional(9.0);
                                // Use contrasting text color based on background
                                let text_color = contrasting_text_color(cell_color);
                                ui.painter().text(
                                    rect.center(),
                                    egui::Align2::CENTER_CENTER,
                                    text,
                                    font_id,
                                    text_color,
                                );
                            }
                    }
                }
            });
        }

        // Add spacing before color scale
        ui.add_space(10.0);

        // Color scale row - aligned with heatmap matrix
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = egui::Vec2::ZERO;
            ui.add_space(label_width); // Align with heatmap left edge

            // Scale width matches heatmap matrix width
            let scale_width = cell_size * x_labels.len() as f32;
            render_color_scale(ui, &sorted_weights, scale_width);
        });
    });

    let new_editing_state = EditingState {
        editing_cell: new_editing_cell,
        edit_buffer: new_edit_buffer,
    };

    let hovered = *new_hovered_cell.borrow();
    (hovered, new_editing_state, weight_change)
}
