use crate::graph_state::{
    HasName, ObservableNode, ObservedNode, StateNode,
};
use crate::layout_bipartite::{
    LayoutBipartite, LayoutStateBipartite,
};
use crate::layout_circular::{LayoutCircular, LayoutStateCircular};
use eframe::egui;
use egui_graphs::{
    DefaultEdgeShape, DefaultNodeShape, DisplayEdge, DisplayNode,
    DrawContext, EdgeProps, Graph, GraphView, Node,
};
use petgraph::graph::DefaultIx;
use petgraph::stable_graph::{IndexType, StableGraph};
use petgraph::{Directed, EdgeType};

// ------------------------------------------------------------------
// Type aliases for graph types
// ------------------------------------------------------------------

pub type GraphDisplay<N> = Graph<
    N,
    f32,
    Directed,
    DefaultIx,
    DefaultNodeShape,
    WeightedEdgeShape,
>;

pub fn setup_graph_display<N>(
    g: &StableGraph<N, f32>,
) -> GraphDisplay<N>
where
    N: Clone,
    N: HasName,
{
    let mut graph: GraphDisplay<N> = GraphDisplay::from(g);
    // Set labels and size for all nodes
    for (idx, node) in g.node_indices().zip(g.node_weights()) {
        if let Some(graph_node) = graph.node_mut(idx) {
            graph_node.set_label(node.name());
        }
    }
    // Clear labels for all edges, inneficient
    let edge_indices: Vec<_> =
        graph.edges_iter().map(|(idx, _)| idx).collect();
    for edge_idx in edge_indices {
        if let Some(edge) = graph.edge_mut(edge_idx) {
            edge.set_label(String::new());
        }
    }
    graph
}

// Type aliases for the display graph types (with visualization properties)
pub type StateGraphDisplay = Graph<
    StateNode,
    f32,
    Directed,
    DefaultIx,
    DefaultNodeShape,
    WeightedEdgeShape,
>;

pub type ObservableGraphDisplay = Graph<
    ObservableNode,
    f32,
    Directed,
    DefaultIx,
    DefaultNodeShape,
    WeightedEdgeShape,
>;

pub type ObservedGraphDisplay = Graph<
    ObservedNode,
    f32,
    Directed,
    DefaultIx,
    DefaultNodeShape,
    WeightedEdgeShape,
>;

// ------------------------------------------------------------------
// Type aliases for graph views (with layout configurations)
// ------------------------------------------------------------------

pub type StateGraphView<'a> = GraphView<
    'a,
    StateNode,
    f32,
    Directed,
    DefaultIx,
    DefaultNodeShape,
    WeightedEdgeShape,
    LayoutStateCircular,
    LayoutCircular,
>;

pub type ObservableGraphView<'a> = GraphView<
    'a,
    ObservableNode,
    f32,
    Directed,
    DefaultIx,
    DefaultNodeShape,
    WeightedEdgeShape,
    LayoutStateBipartite,
    LayoutBipartite,
>;

pub type ObservedGraphView<'a> = GraphView<
    'a,
    ObservedNode,
    f32,
    Directed,
    DefaultIx,
    DefaultNodeShape,
    WeightedEdgeShape,
    LayoutStateCircular,
    LayoutCircular,
>;

// ------------------------------------------------------------------
// Custom edge shape for visualization
// ------------------------------------------------------------------

/// Calculate edge thickness based on weight position in sorted weight list
///
/// Algorithm:
/// 1. Find all occurrences of weight in sorted_weights
/// 2. Calculate middle index between first and last occurrence
/// 3. Interpolate thickness between 1px and 5px based on position
///
/// Special cases:
/// - If sorted_weights is empty, return 3.0 (middle thickness)
/// - If only one weight in list, return 3.0 (middle thickness)
/// - For duplicate weights, use averaged position
fn calculate_edge_thickness(
    weight: f32,
    sorted_weights: &[f32],
) -> f32 {
    if sorted_weights.is_empty() {
        return 3.0; // Middle thickness when no weights available
    }

    if sorted_weights.len() == 1 {
        return 3.0; // Middle thickness when only one edge
    }

    // Find first and last index of the weight in sorted list
    let mut first_idx = None;
    let mut last_idx = None;

    for (i, &w) in sorted_weights.iter().enumerate() {
        if (w - weight).abs() < 1e-6 {
            // Use epsilon comparison for floats
            if first_idx.is_none() {
                first_idx = Some(i);
            }
            last_idx = Some(i);
        }
    }

    // If weight not found (shouldn't happen), use middle thickness
    let (first, last) = match (first_idx, last_idx) {
        (Some(f), Some(l)) => (f, l),
        _ => return 3.0,
    };

    // Calculate middle index
    let middle_idx = (first + last) / 2;

    // Interpolate between 1.0 and 5.0
    let n = sorted_weights.len();
    let ratio = middle_idx as f32 / (n - 1) as f32;

    1.0 + 4.0 * ratio
}

/// Custom edge shape that calculates width from edge weight
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct WeightedEdgeShape {
    default_impl: DefaultEdgeShape,
    weight: f32,
    #[serde(skip)]
    sorted_weights: Vec<f32>,
}

impl From<EdgeProps<f32>> for WeightedEdgeShape {
    fn from(props: EdgeProps<f32>) -> Self {
        let weight = props.payload;
        let mut default_impl = DefaultEdgeShape::from(props);
        // Initialize with middle thickness - will be updated with global weights later
        default_impl.width = 3.0;
        Self {
            default_impl,
            weight,
            sorted_weights: Vec::new(),
        }
    }
}

impl<
    N: Clone,
    Ty: EdgeType,
    Ix: IndexType,
    D: DisplayNode<N, f32, Ty, Ix>,
> DisplayEdge<N, f32, Ty, Ix, D> for WeightedEdgeShape
{
    fn is_inside(
        &self,
        start: &Node<N, f32, Ty, Ix, D>,
        end: &Node<N, f32, Ty, Ix, D>,
        pos: egui::Pos2,
    ) -> bool {
        self.default_impl.is_inside(start, end, pos)
    }

    fn shapes(
        &mut self,
        start: &Node<N, f32, Ty, Ix, D>,
        end: &Node<N, f32, Ty, Ix, D>,
        ctx: &DrawContext,
    ) -> Vec<egui::Shape> {
        self.default_impl.shapes(start, end, ctx)
    }

    fn update(&mut self, state: &EdgeProps<f32>) {
        self.weight = state.payload;
        // Recalculate width using global weight distribution
        self.default_impl.width = calculate_edge_thickness(
            self.weight,
            &self.sorted_weights,
        );
        DisplayEdge::<N, f32, Ty, Ix, D>::update(
            &mut self.default_impl,
            state,
        );
    }

    fn extra_bounds(
        &self,
        start: &Node<N, f32, Ty, Ix, D>,
        end: &Node<N, f32, Ty, Ix, D>,
    ) -> Option<(egui::Pos2, egui::Pos2)> {
        self.default_impl.extra_bounds(start, end)
    }
}

/// Update all edge thicknesses in a graph based on sorted weight distribution
///
/// This function:
/// 1. Updates the sorted_weights field in each edge shape
/// 2. Recalculates edge widths based on global weight distribution
pub fn update_edge_thicknesses<N>(
    graph: &mut GraphDisplay<N>,
    sorted_weights: Vec<f32>,
) where
    N: Clone,
{
    // Get all edge indices first (to avoid borrowing issues)
    let edge_indices: Vec<_> =
        graph.edges_iter().map(|(idx, _)| idx).collect();

    // Update each edge with the sorted weights
    for edge_idx in edge_indices {
        if let Some(edge) = graph.edge_mut(edge_idx) {
            // Update the sorted_weights field
            edge.display_mut().sorted_weights =
                sorted_weights.clone();

            // Recalculate width
            let weight = edge.display().weight;
            edge.display_mut().default_impl.width =
                calculate_edge_thickness(weight, &sorted_weights);
        }
    }
}
