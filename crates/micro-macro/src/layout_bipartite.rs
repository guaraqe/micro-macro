use crate::graph_state::{ObservableNode, ObservableNodeType};
use crate::node_shapes::VisualParams;
use eframe::egui;
use egui_graphs::{
    DisplayEdge, DisplayNode, Graph, Layout, LayoutState,
};
use once_cell::sync::Lazy;
use petgraph::EdgeType;
use petgraph::graph::IndexType;
use petgraph::stable_graph::NodeIndex;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::sync::RwLock;

// Global storage for layout configuration (set before reset_layout)
static PENDING_SPACING: Lazy<RwLock<Option<BipartiteSpacingConfig>>> =
    Lazy::new(|| RwLock::new(None));
static PENDING_VISUALS: Lazy<RwLock<Option<(VisualParams, bool)>>> =
    Lazy::new(|| RwLock::new(None));

pub fn set_pending_layout(
    spacing: BipartiteSpacingConfig,
    visuals: VisualParams,
    label_visibility: bool,
) {
    *PENDING_SPACING.write().unwrap() = Some(spacing);
    *PENDING_VISUALS.write().unwrap() =
        Some((visuals, label_visibility));
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutStateBipartite {
    pub spacing: BipartiteSpacingConfig,
    pub visuals: VisualParams,
    pub label_visibility: bool,
}

impl Default for LayoutStateBipartite {
    fn default() -> Self {
        let spacing = PENDING_SPACING
            .write()
            .unwrap()
            .take()
            .unwrap_or_default();
        let (visuals, label_visibility) =
            PENDING_VISUALS.write().unwrap().take().unwrap_or((
                VisualParams {
                    radius: 5.0,
                    label_gap: 8.0,
                    label_font: 13.0,
                },
                true,
            ));
        Self {
            spacing,
            visuals,
            label_visibility,
        }
    }
}

impl LayoutState for LayoutStateBipartite {}

/// Configuration for spacing in the bipartite layout
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct BipartiteSpacingConfig {
    /// Vertical spacing between nodes within a column
    pub node_gap: f32,
    /// Distance between Source and Destination columns
    pub layer_gap: f32,
}

impl Default for BipartiteSpacingConfig {
    fn default() -> Self {
        Self {
            node_gap: 60.0,
            layer_gap: 220.0,
        }
    }
}

/// Bipartite layout with Source nodes on left, Destination nodes on right
#[derive(Debug, Clone, Default)]
pub struct LayoutBipartite {
    state: LayoutStateBipartite,
    applied: bool,
}

impl Layout<LayoutStateBipartite> for LayoutBipartite {
    fn from_state(
        state: LayoutStateBipartite,
    ) -> impl Layout<LayoutStateBipartite> {
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

        // Separate Source and Destination nodes based on node_type field
        //
        // NOTE: This layout is specifically designed for ObservableNode.
        // The unsafe cast is required because:
        // 1. The Layout trait is generic over N but doesn't allow adding trait bounds
        // 2. This layout is only ever used with ObservableNode (enforced by ObservableGraphView type alias)
        // 3. The memory layout is compatible (we're just reinterpreting the reference)
        // 4. This is safer than the alternative of string parsing node labels
        let mut source_nodes: Vec<_> = Vec::new();
        let mut dest_nodes: Vec<_> = Vec::new();

        for (idx, node) in g.nodes_iter() {
            let label = node.label().to_string();
            let payload = node.payload();

            // SAFETY: This layout is only instantiated with N = ObservableNode via ObservableGraphView
            let node_data = unsafe {
                &*(payload as *const N as *const ObservableNode)
            };

            match node_data.node_type {
                ObservableNodeType::Source => {
                    source_nodes.push((idx, label))
                }
                ObservableNodeType::Destination => {
                    dest_nodes.push((idx, label))
                }
            }
        }

        // Sort both lists alphabetically
        source_nodes.sort_by(|a, b| a.1.cmp(&b.1));
        dest_nodes.sort_by(|a, b| a.1.cmp(&b.1));

        let rect = ui.available_rect_before_wrap();
        let center_x = rect.center().x;
        let center_y = rect.center().y;

        let spacing = &self.state.spacing;
        let half_layer = (spacing.layer_gap.max(40.0)) / 2.0;
        let source_x = center_x - half_layer;
        let dest_x = center_x + half_layer;

        place_column(
            g,
            &source_nodes,
            source_x,
            center_y,
            spacing.node_gap.max(5.0),
        );

        place_column(
            g,
            &dest_nodes,
            dest_x,
            center_y,
            spacing.node_gap.max(5.0),
        );

        self.applied = true;
    }

    fn state(&self) -> LayoutStateBipartite {
        self.state.clone()
    }
}

fn place_column<N, E, Ty, Ix, Dn, De>(
    g: &mut Graph<N, E, Ty, Ix, Dn, De>,
    nodes: &[(NodeIndex<Ix>, String)],
    x: f32,
    center_y: f32,
    spacing: f32,
) where
    N: Clone,
    E: Clone,
    Ty: EdgeType,
    Ix: IndexType,
    Dn: DisplayNode<N, E, Ty, Ix>,
    De: DisplayEdge<N, E, Ty, Ix, Dn>,
{
    if nodes.is_empty() {
        return;
    }
    let count = nodes.len() as f32;
    let start_y = center_y - ((count - 1.0) * spacing) / 2.0;
    for (i, (node_idx, _)) in nodes.iter().enumerate() {
        if let Some(node) = g.node_mut(*node_idx) {
            let y = start_y + (i as f32) * spacing;
            node.set_location(egui::Pos2::new(x, y));
        }
    }
}
