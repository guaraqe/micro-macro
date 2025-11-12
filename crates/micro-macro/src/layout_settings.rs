use serde::{Deserialize, Serialize};

/// Common slider metadata so bounds live in one place.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SliderRange {
    pub min: f32,
    pub max: f32,
    pub step: f32,
}

impl SliderRange {
    pub const fn new(min: f32, max: f32, step: f32) -> Self {
        Self { min, max, step }
    }
}

// Visual ranges
pub const NODE_RADIUS_RANGE: SliderRange =
    SliderRange::new(2.0, 32.0, 0.5);
pub const LABEL_GAP_RANGE: SliderRange =
    SliderRange::new(2.0, 24.0, 0.5);
pub const LABEL_FONT_RANGE: SliderRange =
    SliderRange::new(8.0, 32.0, 1.0);
pub const EDGE_THICKNESS_MIN_RANGE: SliderRange =
    SliderRange::new(0.5, 6.0, 0.1);
pub const EDGE_THICKNESS_MAX_RANGE: SliderRange =
    SliderRange::new(1.0, 12.0, 0.1);
pub const LOOP_RADIUS_RANGE: SliderRange =
    SliderRange::new(0.5, 8.0, 0.1);

// Layout ranges
pub const CIRCULAR_BASE_RADIUS_RANGE: SliderRange =
    SliderRange::new(60.0, 400.0, 5.0);
pub const BIPARTITE_LAYER_GAP_RANGE: SliderRange =
    SliderRange::new(80.0, 500.0, 5.0);
pub const BIPARTITE_NODE_GAP_RANGE: SliderRange =
    SliderRange::new(20.0, 160.0, 2.0);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutSettings {
    pub dynamical_system: CircularTabLayoutSettings,
    pub observable_editor: BipartiteTabLayoutSettings,
    pub observed_dynamics: CircularTabLayoutSettings,
}

impl Default for LayoutSettings {
    fn default() -> Self {
        Self {
            dynamical_system: CircularTabLayoutSettings::new(
                NodeVisualSettings::circular_defaults(),
                EdgeThicknessSettings::default(),
                CircularLayoutSettings::new(120.0),
            ),
            observable_editor: BipartiteTabLayoutSettings::new(
                NodeVisualSettings::bipartite_defaults(),
                EdgeThicknessSettings::default(),
                BipartiteLayoutSettings::new(220.0, 60.0),
            ),
            observed_dynamics: CircularTabLayoutSettings::new(
                NodeVisualSettings::circular_defaults(),
                EdgeThicknessSettings::default(),
                CircularLayoutSettings::new(120.0),
            ),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircularTabLayoutSettings {
    pub visuals: NodeVisualSettings,
    pub edges: EdgeThicknessSettings,
    pub layout: CircularLayoutSettings,
}

impl CircularTabLayoutSettings {
    pub fn new(
        visuals: NodeVisualSettings,
        edges: EdgeThicknessSettings,
        layout: CircularLayoutSettings,
    ) -> Self {
        Self {
            visuals,
            edges,
            layout,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BipartiteTabLayoutSettings {
    pub visuals: NodeVisualSettings,
    pub edges: EdgeThicknessSettings,
    pub layout: BipartiteLayoutSettings,
}

impl BipartiteTabLayoutSettings {
    pub fn new(
        visuals: NodeVisualSettings,
        edges: EdgeThicknessSettings,
        layout: BipartiteLayoutSettings,
    ) -> Self {
        Self {
            visuals,
            edges,
            layout,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeVisualSettings {
    pub node_radius: f32,
    pub label_gap: f32,
    pub label_font_size: f32,
    pub show_labels: bool,
}

impl NodeVisualSettings {
    pub fn circular_defaults() -> Self {
        Self {
            node_radius: 5.0,
            label_gap: 10.0,
            label_font_size: 16.0,
            show_labels: true,
        }
    }

    pub fn bipartite_defaults() -> Self {
        Self {
            node_radius: 5.0,
            label_gap: 8.0,
            label_font_size: 13.0,
            show_labels: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeThicknessSettings {
    pub min_width: f32,
    pub max_width: f32,
}

impl Default for EdgeThicknessSettings {
    fn default() -> Self {
        Self {
            min_width: 1.0,
            max_width: 3.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircularLayoutSettings {
    pub base_radius: f32,
    #[serde(default = "CircularLayoutSettings::default_loop_radius")]
    pub loop_radius: f32,
}

impl CircularLayoutSettings {
    pub fn new(base_radius: f32) -> Self {
        Self {
            base_radius,
            loop_radius: Self::default_loop_radius(),
        }
    }

    pub const fn default_loop_radius() -> f32 {
        3.0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BipartiteLayoutSettings {
    pub layer_gap: f32,
    pub node_gap: f32,
}

impl BipartiteLayoutSettings {
    pub fn new(layer_gap: f32, node_gap: f32) -> Self {
        Self {
            layer_gap,
            node_gap,
        }
    }
}
