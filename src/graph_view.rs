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
            graph_node.display_mut().radius *= 0.75;
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

/// Custom edge shape that calculates width from edge weight
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct WeightedEdgeShape {
    default_impl: DefaultEdgeShape,
    weight: f32,
}

impl From<EdgeProps<f32>> for WeightedEdgeShape {
    fn from(props: EdgeProps<f32>) -> Self {
        let weight = props.payload;
        let mut default_impl = DefaultEdgeShape::from(props);
        // Calculate width from weight: 1.0 + min(weight, 4.0)
        default_impl.width = 1.0 + weight.min(4.0);
        Self {
            default_impl,
            weight,
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
        // Recalculate width when edge is updated
        self.default_impl.width = 1.0 + self.weight.min(4.0);
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
