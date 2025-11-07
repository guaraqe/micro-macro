use crate::graph::{ObservableNode, StateNode};
use crate::layout_bipartite::{
    LayoutBipartite, LayoutStateBipartite,
};
use crate::layout_circular::{LayoutCircular, LayoutStateCircular};
use eframe::egui;
use egui_graphs::{
    DefaultEdgeShape, DefaultNodeShape, DisplayEdge, DisplayNode,
    DrawContext, EdgeProps, GraphView, Node,
};
use petgraph::graph::DefaultIx;
use petgraph::stable_graph::IndexType;
use petgraph::{Directed, EdgeType};

// ------------------------------------------------------------------
// Type aliases for graph views
// ------------------------------------------------------------------

pub type MyGraphView<'a> = GraphView<
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

pub type MappingGraphView<'a> = GraphView<
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
