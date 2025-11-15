use crate::cache::Order;
use crate::node_shapes::VisualParams;
use eframe::egui;
use egui_graphs::{
    DisplayEdge, DisplayNode, Graph, Layout, LayoutState,
};
use once_cell::sync::Lazy;
use petgraph::EdgeType;
use petgraph::graph::IndexType;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::sync::RwLock;

// Global storage for layout configuration (set before reset_layout)
static PENDING_ORDER: Lazy<RwLock<Option<Order>>> = Lazy::new(|| RwLock::new(None));
static PENDING_SPACING: Lazy<RwLock<Option<SpacingConfig>>> = Lazy::new(|| RwLock::new(None));
static PENDING_VISUALS: Lazy<RwLock<Option<(VisualParams, bool)>>> = Lazy::new(|| RwLock::new(None));

pub fn set_pending_layout(
    order: Order,
    spacing: SpacingConfig,
    visuals: VisualParams,
    label_visibility: bool,
) {
    *PENDING_ORDER.write().unwrap() = Some(order);
    *PENDING_SPACING.write().unwrap() = Some(spacing);
    *PENDING_VISUALS.write().unwrap() = Some((visuals, label_visibility));
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutStateCircular {
    pub order: Order,
    pub spacing: SpacingConfig,
    pub visuals: VisualParams,
    pub label_visibility: bool,
}

impl Default for LayoutStateCircular {
    fn default() -> Self {
        let order = PENDING_ORDER.write().unwrap().take().unwrap_or_default();
        let spacing = PENDING_SPACING.write().unwrap().take().unwrap_or_default();
        let (visuals, label_visibility) = PENDING_VISUALS
            .write()
            .unwrap()
            .take()
            .unwrap_or((VisualParams::default(), true));
        Self {
            order,
            spacing,
            visuals,
            label_visibility,
        }
    }
}

impl LayoutState for LayoutStateCircular {}

/// Configuration for spacing/radius of the circular layout
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct SpacingConfig {
    /// Base radius when there are few nodes
    pub base_radius: f32,
    /// Additional radius per node (for auto-scaling)
    pub radius_per_node: f32,
    /// If set, overrides the auto-calculated radius
    pub fixed_radius: Option<f32>,
}

impl SpacingConfig {
    pub fn with_fixed_radius(mut self, radius: f32) -> Self {
        self.fixed_radius = Some(radius);
        self
    }
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

/// Circular layout with configurable spacing
#[derive(Debug, Clone, Default)]
pub struct LayoutCircular {
    state: LayoutStateCircular,
    applied: bool,
}

impl Layout<LayoutStateCircular> for LayoutCircular {
    fn from_state(
        state: LayoutStateCircular,
    ) -> impl Layout<LayoutStateCircular> {
        Self {
            state,
            applied: false,
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
        if self.applied {
            return;
        }

        // Use the order from state
        let node_order = &self.state.order.0;
        let node_count = node_order.len();
        if node_count == 0 {
            return;
        }

        // Calculate center of the canvas
        let rect = ui.available_rect_before_wrap();
        let center_x = rect.center().x;
        let center_y = rect.center().y;

        // Calculate radius using configuration from state
        let spacing = &self.state.spacing;
        let radius = if let Some(fixed) = spacing.fixed_radius {
            fixed
        } else {
            spacing.base_radius
                + (node_count as f32) * spacing.radius_per_node
        };

        // Place nodes in a circle according to the order
        for (i, node_idx) in node_order.iter().enumerate() {
            // Start at top (-π/2) and go clockwise
            let angle = -std::f32::consts::PI / 2.0
                + (i as f32) * 2.0 * std::f32::consts::PI
                    / (node_count as f32);

            let x = center_x + radius * angle.cos();
            let y = center_y + radius * angle.sin();

            // Convert NodeIndex to the generic Ix type
            let idx = petgraph::stable_graph::NodeIndex::<Ix>::new(node_idx.index());
            if let Some(node) = g.node_mut(idx) {
                node.set_location(egui::Pos2::new(x, y));
            }
        }

        self.applied = true;
    }

    fn state(&self) -> LayoutStateCircular {
        self.state.clone()
    }
}
