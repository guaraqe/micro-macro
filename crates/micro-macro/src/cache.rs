use crate::graph_state::calculate_observed_graph;
use crate::graph_view::ObservedGraphDisplay;
use crate::heatmap::HeatmapData;
use crate::store::Store;
use crate::versioned::Memoized;

/// Combined observed data that is calculated together to ensure consistency
pub struct ObservedData {
    pub graph: ObservedGraphDisplay,
    pub heatmap: HeatmapData,
    pub sorted_weights: Vec<f32>,
}

pub struct Cache {
    pub observed_data: Memoized<Store, (u64, u64), ObservedData>,
    pub state_heatmap: Memoized<Store, u64, HeatmapData>,
    pub observable_heatmap: Memoized<Store, u64, HeatmapData>,
    pub state_sorted_weights: Memoized<Store, u64, Vec<f32>>,
    pub observable_sorted_weights: Memoized<Store, u64, Vec<f32>>,
}

impl Cache {
    pub fn new() -> Self {
        let observed_data = Memoized::new(
            |s: &Store| {
                (
                    s.state_graph.version(),
                    s.observable_graph.version(),
                )
            },
            |s: &Store| {
                let graph = calculate_observed_graph(
                    s.state_graph.get(),
                    s.observable_graph.get(),
                );

                // Collect heatmap from the graph we just created
                let heatmap = s.observed_heatmap_from_graph(&graph);

                // Collect sorted weights from the graph we just created
                let mut weights: Vec<f32> = graph
                    .edges_iter()
                    .map(|(_, edge)| *edge.payload())
                    .collect();
                weights.sort_by(|a, b| {
                    a.partial_cmp(b)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
                weights.insert(0, 0.0);

                ObservedData {
                    graph,
                    heatmap,
                    sorted_weights: weights,
                }
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

        let state_sorted_weights = Memoized::new(
            |s: &Store| s.state_graph.version(),
            |s: &Store| s.state_sorted_weights_uncached(),
        );

        let observable_sorted_weights = Memoized::new(
            |s: &Store| s.observable_graph.version(),
            |s: &Store| s.observable_sorted_weights_uncached(),
        );

        Self {
            observed_data,
            state_heatmap,
            observable_heatmap,
            state_sorted_weights,
            observable_sorted_weights,
        }
    }
}
