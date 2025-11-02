use eframe::egui;
use egui_graphs::{DisplayEdge, DisplayNode, Graph, Layout, LayoutState};
use petgraph::graph::IndexType;
use petgraph::EdgeType;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LayoutStateCircular {
    applied: bool,
}

impl LayoutState for LayoutStateCircular {}

#[derive(Debug, Default)]
pub struct LayoutCircular {
    state: LayoutStateCircular,
}

impl Layout<LayoutStateCircular> for LayoutCircular {
    fn from_state(state: LayoutStateCircular) -> impl Layout<LayoutStateCircular> {
        Self { state }
    }

    fn next<N, E, Ty, Ix, Dn, De>(
        &mut self,
        g: &mut Graph<N, E, Ty, Ix, Dn, De>,
        ui: &egui::Ui,
    )
    where
        N: Clone,
        E: Clone,
        Ty: EdgeType,
        Ix: IndexType,
        Dn: DisplayNode<N, E, Ty, Ix>,
        De: DisplayEdge<N, E, Ty, Ix, Dn>,
    {
        // Only apply layout once
        if self.state.applied {
            return;
        }

        // Collect all nodes with their indices and labels
        let mut nodes: Vec<_> = g
            .nodes_iter()
            .map(|(idx, node)| (idx, node.label().to_string()))
            .collect();

        // Sort alphabetically by label
        nodes.sort_by(|a, b| a.1.cmp(&b.1));

        let node_count = nodes.len();
        if node_count == 0 {
            return;
        }

        // Calculate center of the canvas
        let rect = ui.available_rect_before_wrap();
        let center_x = rect.center().x;
        let center_y = rect.center().y;

        // Calculate radius proportional to number of nodes
        // Base radius + scaling factor per node
        let base_radius = 50.0;
        let radius_per_node = 5.0;
        let radius = base_radius + (node_count as f32) * radius_per_node;

        // Place nodes in a circle
        for (i, (node_idx, _label)) in nodes.iter().enumerate() {
            // Start at top (-π/2) and go clockwise
            let angle = -std::f32::consts::PI / 2.0 + (i as f32) * 2.0 * std::f32::consts::PI / (node_count as f32);

            let x = center_x + radius * angle.cos();
            let y = center_y + radius * angle.sin();

            if let Some(node) = g.node_mut(*node_idx) {
                node.set_location(egui::Pos2::new(x, y));
            }
        }

        self.state.applied = true;
    }

    fn state(&self) -> LayoutStateCircular {
        self.state.clone()
    }
}
