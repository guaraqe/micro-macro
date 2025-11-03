use eframe::egui;

/// Render a heatmap visualization of a directed graph's adjacency matrix
/// Returns the new hovered cell state to be stored by the caller
pub fn show_heatmap(
    ui: &mut egui::Ui,
    x_labels: &[String],
    y_labels: &[String],
    matrix: &[Vec<bool>],
    prev_hovered_cell: Option<(usize, usize)>,
) -> Option<(usize, usize)> {
    if x_labels.is_empty() || y_labels.is_empty() {
        ui.label("No nodes to display");
        return None;
    }

    let available_rect = ui.available_rect_before_wrap();
    let spacing = 2.0; // Small spacing for labels
    let label_height = 20.0;
    let label_width = 60.0;

    // Calculate cell size based on available space
    let available_width = available_rect.width() - label_width - spacing;
    let available_height = available_rect.height() - label_height - spacing;

    let cell_width = available_width / x_labels.len() as f32;
    let cell_height = available_height / y_labels.len() as f32;
    let cell_size = cell_width.min(cell_height).max(10.0); // Minimum 10px cells

    // Track which cell is hovered in this frame
    let new_hovered_cell = std::cell::RefCell::new(None::<(usize, usize)>);

    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing = egui::Vec2::ZERO;

        // Grid rows (from top to bottom, but y_idx goes from high to low for bottom-to-top Y-axis)
        for y_idx in (0..y_labels.len()).rev() {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing = egui::Vec2::ZERO;

                // Y-axis label (target) - use previous frame's hover state
                let is_y_highlighted = prev_hovered_cell.map(|(_, hy)| hy == y_idx).unwrap_or(false);

                    let text_color = if is_y_highlighted {
                        egui::Color32::from_rgb(255, 255, 255) // Bright white when highlighted
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

                // Grid cells for this row
                for x_idx in 0..x_labels.len() {
                    let has_edge = matrix[y_idx][x_idx];

                        let cell_color = if has_edge {
                            egui::Color32::from_rgb(200, 60, 70)
                        } else {
                            ui.style().visuals.extreme_bg_color
                        };

                        let (rect, response) = ui.allocate_exact_size(
                            egui::Vec2::new(cell_size, cell_size),
                            egui::Sense::hover(),
                        );

                        if response.hovered() {
                            *new_hovered_cell.borrow_mut() = Some((x_idx, y_idx));
                        }

                        let is_hovered = response.hovered();
                        let final_color = if is_hovered {
                            // Brighten the color on hover
                            if has_edge {
                                egui::Color32::from_rgb(240, 100, 110)
                            } else {
                                egui::Color32::from_rgb(60, 60, 60)
                            }
                        } else {
                            cell_color
                        };

                        ui.painter().rect_filled(rect, 0.0, final_color);

                        // Draw border
                    ui.painter().rect_stroke(
                        rect,
                        0.0,
                        egui::Stroke::new(0.5, egui::Color32::from_gray(40)),
                        egui::epaint::StrokeKind::Outside,
                    );
                }
            });
        }

        // Bottom row: X-axis labels
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = egui::Vec2::ZERO;

            // Empty corner cell
            ui.add_space(label_width);

            // X-axis labels (sources)
            for (x_idx, label) in x_labels.iter().enumerate() {
                let is_highlighted = prev_hovered_cell.map(|(hx, _)| hx == x_idx).unwrap_or(false);

                    let text_color = if is_highlighted {
                        egui::Color32::from_rgb(255, 255, 255) // Bright white when highlighted
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

    // Return the new hovered cell for next frame
    *new_hovered_cell.borrow()
}
