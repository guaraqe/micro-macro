use crate::graph_state::{HasName, ObservableNode, ObservedNode, StateNode};
use crate::layout_bipartite::{LayoutBipartite, LayoutStateBipartite};
use crate::layout_circular::{LayoutCircular, LayoutStateCircular};
use crate::node_shapes::{BipartiteNodeShape, CircularNodeShape};
use eframe::egui::{self, Pos2, Shape, Vec2};
use egui_graphs::{
    DefaultEdgeShape, DisplayEdge, DisplayNode, DrawContext, EdgeProps, Graph, GraphView, Node,
    node_size,
};
use once_cell::sync::Lazy;
use petgraph::graph::DefaultIx;
use petgraph::stable_graph::{IndexType, StableGraph};
use petgraph::{Directed, EdgeType};
use std::sync::RwLock;

static EDGE_THICKNESS_BOUNDS: Lazy<RwLock<(f64, f64)>> = Lazy::new(|| RwLock::new((1.0, 3.0)));
static LOOP_RADIUS: Lazy<RwLock<f64>> = Lazy::new(|| RwLock::new(3.0));

fn edge_thickness_bounds() -> (f64, f64) {
    *EDGE_THICKNESS_BOUNDS.read().unwrap()
}

fn edge_thickness_default() -> f64 {
    let (min, max) = edge_thickness_bounds();
    (min + max) * 0.5
}

pub fn set_edge_thickness_bounds(min: f64, max: f64) {
    let mut guard = EDGE_THICKNESS_BOUNDS.write().unwrap();
    let clamped_min = min.min(max);
    let clamped_max = max.max(clamped_min);
    *guard = (clamped_min, clamped_max);
}

fn loop_radius_value() -> f64 {
    *LOOP_RADIUS.read().unwrap()
}

pub fn set_loop_radius(radius: f64) {
    let mut guard = LOOP_RADIUS.write().unwrap();
    *guard = radius.max(0.1);
}

// ------------------------------------------------------------------
// Type aliases for graph types
// ------------------------------------------------------------------

pub type GraphDisplay<N, D> = Graph<N, f64, Directed, DefaultIx, D, WeightedEdgeShape>;

pub fn setup_graph_display<N, D>(g: &StableGraph<N, f64>) -> GraphDisplay<N, D>
where
    N: Clone,
    N: HasName,
    D: DisplayNode<N, f64, Directed, DefaultIx>,
{
    let mut graph: GraphDisplay<N, D> = GraphDisplay::from(g);
    // Set labels and size for all nodes
    for (idx, node) in g.node_indices().zip(g.node_weights()) {
        if let Some(graph_node) = graph.node_mut(idx) {
            graph_node.set_label(node.name());
        }
    }
    // Clear labels for all edges, inneficient
    let edge_indices: Vec<_> = graph.edges_iter().map(|(idx, _)| idx).collect();
    for edge_idx in edge_indices {
        if let Some(edge) = graph.edge_mut(edge_idx) {
            edge.set_label(String::new());
        }
    }
    graph
}

pub fn setup_state_graph_display(g: &StableGraph<StateNode, f64>) -> StateGraphDisplay {
    setup_graph_display::<StateNode, CircularNodeShape>(g)
}

pub fn setup_observable_graph_display(
    g: &StableGraph<ObservableNode, f64>,
) -> ObservableGraphDisplay {
    setup_graph_display::<ObservableNode, BipartiteNodeShape>(g)
}

pub fn setup_observed_graph_display(g: &StableGraph<ObservedNode, f64>) -> ObservedGraphDisplay {
    setup_graph_display::<ObservedNode, CircularNodeShape>(g)
}

// Type aliases for the display graph types (with visualization properties)
pub type StateGraphDisplay = GraphDisplay<StateNode, CircularNodeShape>;

pub type ObservableGraphDisplay = GraphDisplay<ObservableNode, BipartiteNodeShape>;

pub type ObservedGraphDisplay = GraphDisplay<ObservedNode, CircularNodeShape>;

// ------------------------------------------------------------------
// Type aliases for graph views (with layout configurations)
// ------------------------------------------------------------------

pub type StateGraphView<'a> = GraphView<
    'a,
    StateNode,
    f64,
    Directed,
    DefaultIx,
    CircularNodeShape,
    WeightedEdgeShape,
    LayoutStateCircular,
    LayoutCircular,
>;

pub type ObservableGraphView<'a> = GraphView<
    'a,
    ObservableNode,
    f64,
    Directed,
    DefaultIx,
    BipartiteNodeShape,
    WeightedEdgeShape,
    LayoutStateBipartite,
    LayoutBipartite,
>;

pub type ObservedGraphView<'a> = GraphView<
    'a,
    ObservedNode,
    f64,
    Directed,
    DefaultIx,
    CircularNodeShape,
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
fn calculate_edge_thickness(weight: f64, sorted_weights: &[f64]) -> f64 {
    let (min_width, max_width) = edge_thickness_bounds();
    if sorted_weights.is_empty() {
        return edge_thickness_default();
    }

    if sorted_weights.len() == 1 {
        return edge_thickness_default();
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

    // Interpolate between configured min/max
    let n = sorted_weights.len();
    let ratio = middle_idx as f64 / (n - 1) as f64;
    min_width + (max_width - min_width) * ratio
}

/// Custom edge shape that calculates width from edge weight
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct WeightedEdgeShape {
    default_impl: DefaultEdgeShape,
    weight: f64,
    #[serde(skip)]
    sorted_weights: Vec<f64>,
}

impl From<EdgeProps<f64>> for WeightedEdgeShape {
    fn from(props: EdgeProps<f64>) -> Self {
        let weight = props.payload;
        let mut default_impl = DefaultEdgeShape::from(props);
        // Initialize with middle thickness - will be updated with global weights later
        default_impl.width = edge_thickness_default() as f32;
        Self {
            default_impl,
            weight,
            sorted_weights: Vec::new(),
        }
    }
}

impl<N: Clone, Ty: EdgeType, Ix: IndexType, D: DisplayNode<N, f64, Ty, Ix>>
    DisplayEdge<N, f64, Ty, Ix, D> for WeightedEdgeShape
{
    fn is_inside(
        &self,
        start: &Node<N, f64, Ty, Ix, D>,
        end: &Node<N, f64, Ty, Ix, D>,
        pos: egui::Pos2,
    ) -> bool {
        self.default_impl.is_inside(start, end, pos)
    }

    fn shapes(
        &mut self,
        start: &Node<N, f64, Ty, Ix, D>,
        end: &Node<N, f64, Ty, Ix, D>,
        ctx: &DrawContext,
    ) -> Vec<egui::Shape> {
        self.default_impl.loop_size = loop_radius_value() as f32;
        let mut shapes = self.default_impl.shapes(start, end, ctx);
        if start.id() == end.id() {
            shapes = self.rotate_loop_shapes(start, ctx, shapes);
        }
        shapes
    }

    fn update(&mut self, state: &EdgeProps<f64>) {
        self.weight = state.payload;
        // Recalculate width using global weight distribution
        self.default_impl.width =
            calculate_edge_thickness(self.weight, &self.sorted_weights) as f32;
        DisplayEdge::<N, f64, Ty, Ix, D>::update(&mut self.default_impl, state);
    }

    fn extra_bounds(
        &self,
        start: &Node<N, f64, Ty, Ix, D>,
        end: &Node<N, f64, Ty, Ix, D>,
    ) -> Option<(egui::Pos2, egui::Pos2)> {
        if start.id() == end.id() {
            return Some(self.loop_extra_bounds(start));
        }
        self.default_impl.extra_bounds(start, end)
    }
}

impl WeightedEdgeShape {
    fn rotate_loop_shapes<N: Clone, Ty: EdgeType, Ix: IndexType, D: DisplayNode<N, f64, Ty, Ix>>(
        &self,
        node: &Node<N, f64, Ty, Ix, D>,
        ctx: &DrawContext,
        shapes: Vec<egui::Shape>,
    ) -> Vec<egui::Shape> {
        let graph_center = ctx.meta.graph_bounds().center();
        let node_center_canvas = node.location();
        let node_center_screen = ctx.meta.canvas_to_screen_pos(node_center_canvas);
        let graph_center_screen = ctx.meta.canvas_to_screen_pos(graph_center);

        let mut radial = node_center_screen - graph_center_screen;
        if radial.length_sq() < f32::EPSILON {
            return shapes;
        }
        radial = radial.normalized();
        let base = Vec2::new(0.0, -1.0);
        let angle = Self::signed_angle(base, radial);

        if angle.abs() < f64::EPSILON {
            return shapes;
        }

        shapes
            .into_iter()
            .map(|shape| Self::rotate_shape_about(shape, node_center_screen, angle))
            .collect()
    }

    fn rotate_shape_about(shape: Shape, center: Pos2, angle: f64) -> Shape {
        match shape {
            Shape::CubicBezier(mut cubic) => {
                for point in cubic.points.iter_mut() {
                    *point = Self::rotate_point(*point, center, angle);
                }
                Shape::CubicBezier(cubic)
            }
            Shape::Text(mut text) => {
                text.pos = Self::rotate_point(text.pos, center, angle);
                Shape::Text(text)
            }
            Shape::Vec(shapes) => Shape::Vec(
                shapes
                    .into_iter()
                    .map(|s| Self::rotate_shape_about(s, center, angle))
                    .collect(),
            ),
            other => other,
        }
    }

    fn rotate_point(point: Pos2, center: Pos2, angle: f64) -> Pos2 {
        let offset = point - center;
        let rotated = Self::rotate_vec(offset, angle);
        center + rotated
    }

    fn rotate_vec(vec: Vec2, angle: f64) -> Vec2 {
        let (sin, cos) = angle.sin_cos();
        let sin = sin as f32;
        let cos = cos as f32;
        Vec2::new(vec.x * cos - vec.y * sin, vec.x * sin + vec.y * cos)
    }

    fn signed_angle(from: Vec2, to: Vec2) -> f64 {
        let from_n = if from.length_sq() < f32::EPSILON {
            Vec2::ZERO
        } else {
            from.normalized()
        };
        let to_n = if to.length_sq() < f32::EPSILON {
            Vec2::ZERO
        } else {
            to.normalized()
        };
        if from_n == Vec2::ZERO || to_n == Vec2::ZERO {
            0.0
        } else {
            let det = from_n.x * to_n.y - from_n.y * to_n.x;
            let dot = from_n.dot(to_n);
            det.atan2(dot) as f64
        }
    }

    fn loop_extra_bounds<N: Clone, Ty: EdgeType, Ix: IndexType, D: DisplayNode<N, f64, Ty, Ix>>(
        &self,
        node: &Node<N, f64, Ty, Ix, D>,
    ) -> (Pos2, Pos2) {
        let radius = node_size(node, Vec2::new(1.0, 0.0));
        let order = self.default_impl.order as f32;
        let loop_extent = radius * (loop_radius_value() as f32 + order);
        let max_offset = loop_extent + radius;
        let center = node.location();
        let min = Pos2::new(center.x - max_offset, center.y - max_offset);
        let max = Pos2::new(center.x + max_offset, center.y + max_offset);
        (min, max)
    }
}

/// Update all edge thicknesses in a graph based on sorted weight distribution
///
/// This function:
/// 1. Updates the sorted_weights field in each edge shape
/// 2. Recalculates edge widths based on global weight distribution
pub fn update_edge_thicknesses<N, D>(graph: &mut GraphDisplay<N, D>, sorted_weights: Vec<f64>)
where
    N: Clone,
    D: DisplayNode<N, f64, Directed, DefaultIx>,
{
    // Get all edge indices first (to avoid borrowing issues)
    let edge_indices: Vec<_> = graph.edges_iter().map(|(idx, _)| idx).collect();

    // Update each edge with the sorted weights
    for edge_idx in edge_indices {
        if let Some(edge) = graph.edge_mut(edge_idx) {
            // Update the sorted_weights field
            edge.display_mut().sorted_weights = sorted_weights.clone();

            // Recalculate width
            let weight = edge.display().weight;
            edge.display_mut().default_impl.width =
                calculate_edge_thickness(weight, &sorted_weights) as f32;
        }
    }
}
