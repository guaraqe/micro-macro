use eframe::egui;
use egui_graphs::{
    DisplayEdge, DisplayNode, Graph, Layout, LayoutState,
};
use petgraph::EdgeType;
use petgraph::graph::IndexType;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LayoutStateCircular {
    applied: bool,
}

impl LayoutState for LayoutStateCircular {}

/// Configuration for spacing/radius of the circular layout
#[derive(Debug, Clone)]
pub struct SpacingConfig {
    /// Base radius when there are few nodes
    pub base_radius: f32,
    /// Additional radius per node (for auto-scaling)
    pub radius_per_node: f32,
    /// If set, overrides the auto-calculated radius
    pub fixed_radius: Option<f32>,
}

impl Default for SpacingConfig {
    fn default() -> Self {
        Self {
            base_radius: 50.0,
            radius_per_node: 5.0,
            fixed_radius: None,
        }
    }
}

#[allow(dead_code)]
impl SpacingConfig {
    pub fn with_base_radius(mut self, base: f32) -> Self {
        self.base_radius = base;
        self
    }

    pub fn with_radius_per_node(mut self, per_node: f32) -> Self {
        self.radius_per_node = per_node;
        self
    }

    pub fn with_fixed_radius(mut self, radius: f32) -> Self {
        self.fixed_radius = Some(radius);
        self
    }
}

/// Sort order for circular layout nodes
#[allow(dead_code)]
#[derive(Debug, Clone, Default)]
pub enum SortOrder {
    /// Alphabetical by label (ascending)
    #[default]
    Alphabetical,
    /// Reverse alphabetical by label (descending)
    ReverseAlphabetical,
    /// No sorting - preserve insertion order
    None,
}

/// Circular layout with configurable sorting and spacing
#[derive(Debug, Clone, Default)]
pub struct LayoutCircular {
    state: LayoutStateCircular,
    sort_order: SortOrder,
    spacing: SpacingConfig,
}

#[allow(dead_code)]
impl LayoutCircular {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_sort_order(mut self, sort_order: SortOrder) -> Self {
        self.sort_order = sort_order;
        self
    }

    pub fn without_sorting(mut self) -> Self {
        self.sort_order = SortOrder::None;
        self
    }

    pub fn with_spacing(mut self, spacing: SpacingConfig) -> Self {
        self.spacing = spacing;
        self
    }
}

impl Layout<LayoutStateCircular> for LayoutCircular {
    fn from_state(
        state: LayoutStateCircular,
    ) -> impl Layout<LayoutStateCircular> {
        Self {
            state,
            sort_order: SortOrder::default(),
            spacing: SpacingConfig::default(),
        }
    }

    fn next<N, E, Ty, Ix, Dn, De>(
        &mut self,
        g: &mut Graph<N, E, Ty, Ix, Dn, De>,
        ui: &egui::Ui,
    ) where
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

        // Sort according to the configured sort order
        match self.sort_order {
            SortOrder::Alphabetical => {
                nodes.sort_by(|a, b| a.1.cmp(&b.1));
            }
            SortOrder::ReverseAlphabetical => {
                nodes.sort_by(|a, b| b.1.cmp(&a.1));
            }
            SortOrder::None => {
                // Keep insertion order - no sorting
            }
        }

        let node_count = nodes.len();
        if node_count == 0 {
            return;
        }

        // Calculate center of the canvas
        let rect = ui.available_rect_before_wrap();
        let center_x = rect.center().x;
        let center_y = rect.center().y;

        // Calculate radius using configuration
        let radius = if let Some(fixed) = self.spacing.fixed_radius {
            fixed
        } else {
            self.spacing.base_radius
                + (node_count as f32) * self.spacing.radius_per_node
        };

        // Place nodes in a circle
        for (i, (node_idx, _label)) in nodes.iter().enumerate() {
            // Start at top (-π/2) and go clockwise
            let angle = -std::f32::consts::PI / 2.0
                + (i as f32) * 2.0 * std::f32::consts::PI
                    / (node_count as f32);

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
