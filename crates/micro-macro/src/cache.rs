use crate::graph_state::calculate_observed_graph;
use crate::graph_view::ObservedGraphDisplay;
use crate::heatmap::HeatmapData;
use crate::store::Store;
use crate::versioned::Memoized;

pub struct Cache {
    pub observed_graph:
        Memoized<Store, (u64, u64), ObservedGraphDisplay>,
    pub state_heatmap: Memoized<Store, u64, HeatmapData>,
    pub observable_heatmap: Memoized<Store, u64, HeatmapData>,
    pub observed_heatmap: Memoized<Store, (u64, u64), HeatmapData>,
    pub state_sorted_weights: Memoized<Store, u64, Vec<f32>>,
    pub observable_sorted_weights: Memoized<Store, u64, Vec<f32>>,
    pub observed_sorted_weights: Memoized<Store, (u64, u64), Vec<f32>>,
}

impl Cache {
    pub fn new() -> Self {
        let observed_graph = Memoized::new(
            |s: &Store| {
                (
                    s.state_graph.version(),
                    s.observable_graph.version(),
                )
            },
            |s: &Store| {
                calculate_observed_graph(
                    s.state_graph.get(),
                    s.observable_graph.get(),
                )
            },
        );

        let state_heatmap = Memoized::new(
            |s: &Store| s.state_graph.version(),
            |s: &Store| s.state_heatmap_uncached(),
        );

        let observable_heatmap = Memoized::new(
            |s: &Store| s.observable_graph.version(),
            |s: &Store| s.observable_heatmap_uncached(),
        );

        let observed_heatmap = Memoized::new(
            |s: &Store| {
                (
                    s.state_graph.version(),
                    s.observable_graph.version(),
                )
            },
            |s: &Store| s.observed_heatmap_uncached(),
        );

        let state_sorted_weights = Memoized::new(
            |s: &Store| s.state_graph.version(),
            |s: &Store| s.state_sorted_weights_uncached(),
        );

        let observable_sorted_weights = Memoized::new(
            |s: &Store| s.observable_graph.version(),
            |s: &Store| s.observable_sorted_weights_uncached(),
        );

        let observed_sorted_weights = Memoized::new(
            |s: &Store| {
                (
                    s.state_graph.version(),
                    s.observable_graph.version(),
                )
            },
            |s: &Store| s.observed_sorted_weights_uncached(),
        );

        Self {
            observed_graph,
            state_heatmap,
            observable_heatmap,
            observed_heatmap,
            state_sorted_weights,
            observable_sorted_weights,
            observed_sorted_weights,
        }
    }
}
