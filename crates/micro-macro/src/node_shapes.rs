use eframe::egui::{
    self, Color32, FontFamily, FontId, Pos2, Shape, Stroke, Vec2,
    epaint::{CircleShape, TextShape},
};
use egui_graphs::{DisplayNode, DrawContext, NodeProps};
use once_cell::sync::Lazy;
use petgraph::{EdgeType, stable_graph::IndexType};
use serde::{Deserialize, Serialize};
use std::sync::RwLock;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::graph_state::{ObservableNode, ObservableNodeType};

const CIRCULAR_RADIUS: f32 = 5.0;
const CIRCULAR_LABEL_GAP: f32 = 10.0;
const CIRCULAR_LABEL_FONT: f32 = 16.0;
const BIPARTITE_RADIUS: f32 = 5.0;
const BIPARTITE_LABEL_GAP: f32 = 8.0;
const BIPARTITE_LABEL_FONT: f32 = 13.0;

static LABEL_VISIBILITY: AtomicBool = AtomicBool::new(true);
#[derive(Clone, Copy)]
struct VisualParams {
    radius: f32,
    label_gap: f32,
    label_font: f32,
}

static CIRCULAR_VISUALS: Lazy<RwLock<VisualParams>> =
    Lazy::new(|| {
        RwLock::new(VisualParams {
            radius: CIRCULAR_RADIUS,
            label_gap: CIRCULAR_LABEL_GAP,
            label_font: CIRCULAR_LABEL_FONT,
        })
    });

static BIPARTITE_VISUALS: Lazy<RwLock<VisualParams>> =
    Lazy::new(|| {
        RwLock::new(VisualParams {
            radius: BIPARTITE_RADIUS,
            label_gap: BIPARTITE_LABEL_GAP,
            label_font: BIPARTITE_LABEL_FONT,
        })
    });

pub fn set_label_visibility(always: bool) {
    LABEL_VISIBILITY.store(always, Ordering::Relaxed);
}

pub fn set_circular_visual_params(
    radius: f32,
    label_gap: f32,
    label_font: f32,
) {
    let mut guard = CIRCULAR_VISUALS.write().unwrap();
    *guard = VisualParams {
        radius,
        label_gap,
        label_font,
    };
}

pub fn set_bipartite_visual_params(
    radius: f32,
    label_gap: f32,
    label_font: f32,
) {
    let mut guard = BIPARTITE_VISUALS.write().unwrap();
    *guard = VisualParams {
        radius,
        label_gap,
        label_font,
    };
}

fn circular_visuals() -> VisualParams {
    *CIRCULAR_VISUALS.read().unwrap()
}

fn bipartite_visuals() -> VisualParams {
    *BIPARTITE_VISUALS.read().unwrap()
}

fn labels_always() -> bool {
    LABEL_VISIBILITY.load(Ordering::Relaxed)
}

fn label_top_left_for_direction(
    ctx: &DrawContext,
    node_pos: Pos2,
    dir: Vec2,
    galley: &std::sync::Arc<egui::Galley>,
    radius: f32,
    gap: f32,
) -> Pos2 {
    let mut direction = dir;
    if direction.length_sq() < f32::EPSILON {
        direction = Vec2::new(0.0, -1.0);
    } else {
        direction = direction.normalized();
    }

    let radius_screen = ctx.meta.canvas_to_screen_size(radius);
    let gap_screen = ctx.meta.canvas_to_screen_size(gap);
    let support = 0.5
        * (direction.x.abs() * galley.size().x
            + direction.y.abs() * galley.size().y);

    let node_screen = ctx.meta.canvas_to_screen_pos(node_pos);
    let center_screen = node_screen
        + direction * (radius_screen + gap_screen + support);

    Pos2::new(
        center_screen.x - galley.size().x / 2.0,
        center_screen.y - galley.size().y / 2.0,
    )
}

/// Node shape tailored for circular layouts: labels are pushed outward along
/// the radial direction so they do not overlap outgoing edges.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CircularNodeShape {
    pos: Pos2,
    selected: bool,
    dragged: bool,
    hovered: bool,
    color: Option<Color32>,
    label_text: String,
    radius: f32,
    label_font: f32,
    label_gap: f32,
}

impl<N: Clone> From<NodeProps<N>> for CircularNodeShape {
    fn from(props: NodeProps<N>) -> Self {
        let mut shape = Self {
            pos: props.location(),
            selected: props.selected,
            dragged: props.dragged,
            hovered: props.hovered,
            color: props.color(),
            label_text: props.label,
            radius: CIRCULAR_RADIUS,
            label_font: CIRCULAR_LABEL_FONT,
            label_gap: CIRCULAR_LABEL_GAP,
        };
        shape.refresh_visuals();
        shape
    }
}

impl<N: Clone, E: Clone, Ty: EdgeType, Ix: IndexType>
    DisplayNode<N, E, Ty, Ix> for CircularNodeShape
{
    fn closest_boundary_point(&self, dir: Vec2) -> Pos2 {
        self.pos + dir.normalized() * self.radius
    }

    fn shapes(&mut self, ctx: &DrawContext) -> Vec<Shape> {
        self.refresh_visuals();
        let mut res = Vec::with_capacity(2);
        let center_screen = ctx.meta.canvas_to_screen_pos(self.pos);
        let radius_screen =
            ctx.meta.canvas_to_screen_size(self.radius);
        let color = self.effective_color(ctx);
        let stroke = self.effective_stroke();

        res.push(
            CircleShape {
                center: center_screen,
                radius: radius_screen,
                fill: color,
                stroke,
            }
            .into(),
        );

        if !self.should_show_label() {
            return res;
        }

        let galley = self.label_galley(ctx, color);
        let label_pos = self.circular_label_pos(ctx, &galley);
        res.push(TextShape::new(label_pos, galley, color).into());
        res
    }

    fn update(&mut self, state: &NodeProps<N>) {
        self.refresh_visuals();
        self.pos = state.location();
        self.selected = state.selected;
        self.dragged = state.dragged;
        self.hovered = state.hovered;
        self.color = state.color();
        self.label_text = state.label.clone();
    }

    fn is_inside(&self, pos: Pos2) -> bool {
        (pos - self.pos).length() <= self.radius
    }
}

impl CircularNodeShape {
    fn refresh_visuals(&mut self) {
        let visuals = circular_visuals();
        self.radius = visuals.radius;
        self.label_gap = visuals.label_gap;
        self.label_font = visuals.label_font;
    }

    fn should_show_label(&self) -> bool {
        labels_always()
            || self.selected
            || self.dragged
            || self.hovered
    }

    fn effective_color(&self, ctx: &DrawContext) -> Color32 {
        if let Some(c) = self.color {
            return c;
        }
        let visuals = if self.selected || self.dragged || self.hovered
        {
            ctx.ctx.style().visuals.widgets.active
        } else {
            ctx.ctx.style().visuals.widgets.inactive
        };
        visuals.fg_stroke.color
    }

    fn effective_stroke(&self) -> Stroke {
        if self.selected {
            Stroke::new(4.0, egui::Color32::from_rgb(200, 60, 70))
        } else {
            Stroke::new(2.0, egui::Color32::from_rgb(80, 80, 80))
        }
    }

    fn label_galley(
        &self,
        ctx: &DrawContext,
        color: Color32,
    ) -> std::sync::Arc<egui::Galley> {
        ctx.ctx.fonts_mut(|f| {
            f.layout_no_wrap(
                self.label_text.clone(),
                FontId::new(self.label_font, FontFamily::Monospace),
                color,
            )
        })
    }

    fn circular_label_pos(
        &self,
        ctx: &DrawContext,
        galley: &std::sync::Arc<egui::Galley>,
    ) -> Pos2 {
        let graph_center = ctx.meta.graph_bounds().center();
        label_top_left_for_direction(
            ctx,
            self.pos,
            self.pos - graph_center,
            galley,
            self.radius,
            self.label_gap,
        )
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
enum LabelSide {
    Left,
    Right,
}

impl LabelSide {
    fn from_node(node_type: ObservableNodeType) -> Self {
        match node_type {
            ObservableNodeType::Source => Self::Left,
            ObservableNodeType::Destination => Self::Right,
        }
    }
}

/// Node shape for bipartite layouts: source nodes show labels on the left,
/// destination nodes on the right, keeping edge lanes unobstructed.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BipartiteNodeShape {
    pos: Pos2,
    selected: bool,
    dragged: bool,
    hovered: bool,
    color: Option<Color32>,
    label_text: String,
    side: LabelSide,
    radius: f32,
    label_font: f32,
    label_gap: f32,
}

impl From<NodeProps<ObservableNode>> for BipartiteNodeShape {
    fn from(props: NodeProps<ObservableNode>) -> Self {
        let mut shape = Self {
            pos: props.location(),
            selected: props.selected,
            dragged: props.dragged,
            hovered: props.hovered,
            color: props.color(),
            label_text: props.label,
            side: LabelSide::from_node(props.payload.node_type),
            radius: BIPARTITE_RADIUS,
            label_font: BIPARTITE_LABEL_FONT,
            label_gap: BIPARTITE_LABEL_GAP,
        };
        shape.refresh_visuals();
        shape
    }
}

impl<E: Clone, Ty: EdgeType, Ix: IndexType>
    DisplayNode<ObservableNode, E, Ty, Ix> for BipartiteNodeShape
{
    fn closest_boundary_point(&self, dir: Vec2) -> Pos2 {
        self.pos + dir.normalized() * self.radius
    }

    fn shapes(&mut self, ctx: &DrawContext) -> Vec<Shape> {
        self.refresh_visuals();
        let mut res = Vec::with_capacity(2);
        let center_screen = ctx.meta.canvas_to_screen_pos(self.pos);
        let radius_screen =
            ctx.meta.canvas_to_screen_size(self.radius);
        let color = self.effective_color(ctx);
        let stroke = self.effective_stroke();

        res.push(
            CircleShape {
                center: center_screen,
                radius: radius_screen,
                fill: color,
                stroke,
            }
            .into(),
        );

        if !self.should_show_label() {
            return res;
        }

        let galley = self.label_galley(ctx, color);
        let label_pos = self.bipartite_label_pos(ctx, &galley);
        res.push(TextShape::new(label_pos, galley, color).into());
        res
    }

    fn update(&mut self, state: &NodeProps<ObservableNode>) {
        self.refresh_visuals();
        self.pos = state.location();
        self.selected = state.selected;
        self.dragged = state.dragged;
        self.hovered = state.hovered;
        self.color = state.color();
        self.label_text = state.label.clone();
        self.side = LabelSide::from_node(state.payload.node_type);
    }

    fn is_inside(&self, pos: Pos2) -> bool {
        (pos - self.pos).length() <= self.radius
    }
}

impl BipartiteNodeShape {
    fn refresh_visuals(&mut self) {
        let visuals = bipartite_visuals();
        self.radius = visuals.radius;
        self.label_gap = visuals.label_gap;
        self.label_font = visuals.label_font;
    }

    fn should_show_label(&self) -> bool {
        labels_always()
            || self.selected
            || self.dragged
            || self.hovered
    }

    fn effective_color(&self, ctx: &DrawContext) -> Color32 {
        if let Some(c) = self.color {
            return c;
        }
        let visuals = if self.selected || self.dragged || self.hovered
        {
            ctx.ctx.style().visuals.widgets.active
        } else {
            ctx.ctx.style().visuals.widgets.inactive
        };
        visuals.fg_stroke.color
    }

    fn effective_stroke(&self) -> Stroke {
        if self.selected {
            Stroke::new(4.0, egui::Color32::from_rgb(200, 60, 70))
        } else {
            Stroke::new(2.0, egui::Color32::from_rgb(80, 80, 80))
        }
    }

    fn label_galley(
        &self,
        ctx: &DrawContext,
        color: Color32,
    ) -> std::sync::Arc<egui::Galley> {
        ctx.ctx.fonts_mut(|f| {
            f.layout_no_wrap(
                self.label_text.clone(),
                FontId::new(self.label_font, FontFamily::Monospace),
                color,
            )
        })
    }

    fn bipartite_label_pos(
        &self,
        ctx: &DrawContext,
        galley: &std::sync::Arc<egui::Galley>,
    ) -> Pos2 {
        let dir = match self.side {
            LabelSide::Left => Vec2::new(-1.0, 0.0),
            LabelSide::Right => Vec2::new(1.0, 0.0),
        };
        label_top_left_for_direction(
            ctx,
            self.pos,
            dir,
            galley,
            self.radius,
            self.label_gap,
        )
    }
}
