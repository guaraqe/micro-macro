use eframe::egui;
use egui_graphs::{DisplayEdge, DisplayNode, Graph, Layout, LayoutState};
use petgraph::graph::IndexType;
use petgraph::EdgeType;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LayoutStateBipartite {
    applied: bool,
}

impl LayoutState for LayoutStateBipartite {}

/// Configuration for spacing in the bipartite layout
#[derive(Debug, Clone)]
pub struct BipartiteSpacingConfig {
    /// Horizontal spacing between the two columns
    pub column_spacing: f32,
    /// Vertical spacing between nodes
    pub vertical_spacing: f32,
    /// Left margin from the edge
    pub left_margin: f32,
    /// Top margin from the edge
    pub top_margin: f32,
}

impl Default for BipartiteSpacingConfig {
    fn default() -> Self {
        Self {
            column_spacing: 200.0,  // Base spacing, will be adjusted dynamically
            vertical_spacing: 60.0,
            left_margin: 100.0,
            top_margin: 100.0,
        }
    }
}

/// Bipartite layout with Source nodes on left, Destination nodes on right
#[derive(Debug, Clone)]
pub struct LayoutBipartite {
    state: LayoutStateBipartite,
    spacing: BipartiteSpacingConfig,
}

impl Default for LayoutBipartite {
    fn default() -> Self {
        Self {
            state: LayoutStateBipartite::default(),
            spacing: BipartiteSpacingConfig::default(),
        }
    }
}

impl LayoutBipartite {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self::default()
    }

    #[allow(dead_code)]
    pub fn with_spacing(mut self, spacing: BipartiteSpacingConfig) -> Self {
        self.spacing = spacing;
        self
    }
}

impl Layout<LayoutStateBipartite> for LayoutBipartite {
    fn from_state(state: LayoutStateBipartite) -> impl Layout<LayoutStateBipartite> {
        Self {
            state,
            spacing: BipartiteSpacingConfig::default(),
        }
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

        // Collect all nodes with their labels
        let mut all_nodes: Vec<_> = Vec::new();

        for (idx, node) in g.nodes_iter() {
            let label = node.label().to_string();
            all_nodes.push((idx, label));
        }

        // Separate into Source and Destination based on label patterns
        // Source nodes are those that start with "Node", Destination are "Value"
        let mut source_nodes: Vec<_> = Vec::new();
        let mut dest_nodes: Vec<_> = Vec::new();

        for (idx, label) in all_nodes {
            if label.starts_with("Node") {
                source_nodes.push((idx, label));
            } else {
                dest_nodes.push((idx, label));
            }
        }

        // Sort both lists alphabetically
        source_nodes.sort_by(|a, b| a.1.cmp(&b.1));
        dest_nodes.sort_by(|a, b| a.1.cmp(&b.1));

        let rect = ui.available_rect_before_wrap();
        let center_x = rect.center().x;

        // Calculate dynamic column spacing based on number of Source nodes
        let source_count = source_nodes.len();
        let dynamic_spacing = (80.0 + (source_count as f32) * 10.0).min(300.0);

        // Calculate positions for left column (Source)
        let left_x = center_x - dynamic_spacing / 2.0;

        // Calculate positions for right column (Destination)
        let right_x = center_x + dynamic_spacing / 2.0;

        // Place Source nodes in left column
        for (i, (node_idx, _label)) in source_nodes.iter().enumerate() {
            let y = self.spacing.top_margin + (i as f32) * self.spacing.vertical_spacing;
            if let Some(node) = g.node_mut(*node_idx) {
                node.set_location(egui::Pos2::new(left_x, y));
            }
        }

        // Place Destination nodes in right column
        for (i, (node_idx, _label)) in dest_nodes.iter().enumerate() {
            let y = self.spacing.top_margin + (i as f32) * self.spacing.vertical_spacing;
            if let Some(node) = g.node_mut(*node_idx) {
                node.set_location(egui::Pos2::new(right_x, y));
            }
        }

        self.state.applied = true;
    }

    fn state(&self) -> LayoutStateBipartite {
        self.state.clone()
    }
}
