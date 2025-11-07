use eframe::egui;

#[derive(Debug, Clone)]
pub struct WeightChange {
    pub x: usize,
    pub y: usize,
    pub new_weight: f32,
}

pub struct EditingState {
    pub editing_cell: Option<(usize, usize)>,
    pub edit_buffer: String,
}

/// Render a heatmap visualization of a directed graph's adjacency matrix with inline editing
/// Returns (new_hovered_cell, new_editing_state, optional_weight_change)
pub fn show_heatmap(
    ui: &mut egui::Ui,
    x_labels: &[String],
    y_labels: &[String],
    matrix: &[Vec<Option<f32>>],
    prev_hovered_cell: Option<(usize, usize)>,
    editing_state: EditingState,
) -> (Option<(usize, usize)>, EditingState, Option<WeightChange>) {
    if x_labels.is_empty() || y_labels.is_empty() {
        ui.label("No nodes to display");
        return (None, editing_state, None);
    }

    let available_rect = ui.available_rect_before_wrap();
    let spacing = 2.0;
    let label_height = 20.0;
    let label_width = 60.0;

    let available_width =
        available_rect.width() - label_width - spacing;
    let available_height =
        available_rect.height() - label_height - spacing;

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

        for y_idx in (0..y_labels.len()).rev() {
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
                        // Determine background color based on whether cell has weight
                        let cell_color = if weight_opt.is_some() {
                            egui::Color32::from_rgb(200, 60, 70)
                        } else {
                            ui.style().visuals.extreme_bg_color
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
                                if let Ok(parsed_weight) = new_edit_buffer.parse::<f32>() {
                                    weight_change = Some(WeightChange {
                                        x: x_idx,
                                        y: y_idx,
                                        new_weight: parsed_weight,
                                    });
                                }
                                new_editing_cell = None;
                                new_edit_buffer.clear();
                            }
                            // Handle Tab key - commit and move to next cell
                            else if tab_pressed {
                                if let Ok(parsed_weight) = new_edit_buffer.parse::<f32>() {
                                    weight_change = Some(WeightChange {
                                        x: x_idx,
                                        y: y_idx,
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
                                    .map(|w| format!("{:.1}", w))
                                    .unwrap_or_default();
                            }
                            // Handle Escape key or clicking outside - cancel editing
                            else if escape_pressed || (te_response.lost_focus() && !te_response.has_focus()) {
                                new_editing_cell = None;
                                new_edit_buffer.clear();
                            }
                        }
                    } else {
                        // Normal cell rendering
                        let cell_color = if weight_opt.is_some() {
                            egui::Color32::from_rgb(200, 60, 70)
                        } else {
                            ui.style().visuals.extreme_bg_color
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
                                .map(|w| format!("{:.1}", w))
                                .unwrap_or_default();
                        }

                        let is_hovered = response.hovered();
                        let final_color = if is_hovered {
                            if weight_opt.is_some() {
                                egui::Color32::from_rgb(240, 100, 110)
                            } else {
                                egui::Color32::from_rgb(60, 60, 60)
                            }
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

                        if let Some(weight) = weight_opt {
                            let text = format!("{:.1}", weight);
                            let font_id = egui::FontId::proportional(9.0);
                            let text_color = egui::Color32::WHITE;
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

        // Bottom row: X-axis labels
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
    });

    let new_editing_state = EditingState {
        editing_cell: new_editing_cell,
        edit_buffer: new_edit_buffer,
    };

    (*new_hovered_cell.borrow(), new_editing_state, weight_change)
}
