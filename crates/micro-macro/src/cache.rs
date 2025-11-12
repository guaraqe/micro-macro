use crate::graph_state::{calculate_observed_graph, compute_input_statistics, compute_output_statistics};
use crate::graph_view::ObservedGraphDisplay;
use crate::heatmap::HeatmapData;
use crate::store::Store;
use crate::versioned::Memoized;
use markov::Prob;
use ndarray::linalg::Dot;
use petgraph::stable_graph::NodeIndex;

/// Combined state data that is calculated together to ensure consistency
pub struct StateData {
    pub heatmap: HeatmapData,
    pub sorted_weights: Vec<f32>,
    pub equilibrium: Prob<NodeIndex, f64>,
}

/// Combined observable data that is calculated together to ensure consistency
pub struct ObservableData {
    pub heatmap: HeatmapData,
    pub sorted_weights: Vec<f32>,
}

/// Combined observed data that is calculated together to ensure consistency
pub struct ObservedData {
    pub graph: ObservedGraphDisplay,
    pub heatmap: HeatmapData,
    pub sorted_weights: Vec<f32>,
    pub equilibrium_from_state: Prob<NodeIndex, f64>,
    pub equilibrium_calculated: Prob<NodeIndex, f64>,
}

pub struct Cache {
    pub state_data: Memoized<Store, u64, StateData>,
    pub observable_data: Memoized<Store, u64, ObservableData>,
    pub observed_data: Memoized<Store, (u64, u64), ObservedData>,
}

impl Cache {
    pub fn new() -> Self {
        let state_data = Memoized::new(
            |s: &Store| s.state_graph.version(),
            |s: &Store| {
                let heatmap = s.state_heatmap_uncached();
                let sorted_weights = s.state_sorted_weights_uncached();

                // Compute equilibrium distribution for state graph
                let equilibrium = if s.state_graph.get().node_count() > 0 {
                    if let Ok(input_stats) = compute_input_statistics(
                        s.state_graph.get(),
                        s.observable_graph.get(),
                    ) {
                        input_stats.state_markov.compute_equilibrium(
                            &input_stats.state_prob,
                            1e-4,
                            100,
                        )
                    } else {
                        // If we can't compute stats, return uniform distribution
                        let node_count = s.state_graph.get().node_count();
                        let indices: Vec<_> = s.state_graph.get().nodes_iter().map(|(idx, _)| idx).collect();
                        Prob::from_assoc(
                            node_count,
                            indices.into_iter().map(|idx| (idx, 1.0)),
                        ).unwrap_or_else(|_| {
                            // Fallback: create a minimal valid Prob
                            Prob::from_assoc(1, vec![(NodeIndex::new(0), 1.0)]).unwrap()
                        })
                    }
                } else {
                    // Empty graph: create a minimal valid Prob
                    Prob::from_assoc(1, vec![(NodeIndex::new(0), 1.0)]).unwrap()
                };

                StateData {
                    heatmap,
                    sorted_weights,
                    equilibrium,
                }
            },
        );

        let observable_data = Memoized::new(
            |s: &Store| s.observable_graph.version(),
            |s: &Store| {
                let heatmap = s.observable_heatmap_uncached();
                let sorted_weights = s.observable_sorted_weights_uncached();

                ObservableData {
                    heatmap,
                    sorted_weights,
                }
            },
        );

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

                // Compute equilibrium distributions
                let (equilibrium_from_state, equilibrium_calculated) =
                    if s.state_graph.get().node_count() > 0 {
                        match compute_input_statistics(
                            s.state_graph.get(),
                            s.observable_graph.get(),
                        ) {
                            Ok(input_stats) => {
                                // 1. State equilibrium
                                let state_eq = input_stats.state_markov.compute_equilibrium(
                                    &input_stats.state_prob,
                                    1e-4,
                                    100,
                                );

                                // 2. Observed equilibrium = state_eq · observable_markov
                                let obs_eq_from_state = state_eq.dot(&input_stats.observable_markov);

                                // 3. Calculated observed equilibrium
                                let obs_eq_calculated = match compute_output_statistics(&input_stats) {
                                    Ok(output_stats) => {
                                        output_stats.observed_markov.compute_equilibrium(
                                            &output_stats.observed_prob,
                                            1e-4,
                                            100,
                                        )
                                    }
                                    Err(_) => {
                                        // Fallback to observed_prob if calculation fails
                                        obs_eq_from_state.clone()
                                    }
                                };

                                (obs_eq_from_state, obs_eq_calculated)
                            }
                            Err(_) => {
                                // Create fallback distributions
                                let node_count = graph.node_count().max(1);
                                let indices: Vec<_> = graph.nodes_iter().map(|(idx, _)| idx).collect();
                                let fallback = Prob::from_assoc(
                                    node_count,
                                    indices.into_iter().map(|idx| (idx, 1.0)),
                                ).unwrap_or_else(|_| {
                                    Prob::from_assoc(1, vec![(NodeIndex::new(0), 1.0)]).unwrap()
                                });
                                (fallback.clone(), fallback)
                            }
                        }
                    } else {
                        // Empty graph fallback
                        let fallback = Prob::from_assoc(1, vec![(NodeIndex::new(0), 1.0)]).unwrap();
                        (fallback.clone(), fallback)
                    };

                ObservedData {
                    graph,
                    heatmap,
                    sorted_weights: weights,
                    equilibrium_from_state,
                    equilibrium_calculated,
                }
            },
        );

        Self {
            state_data,
            observable_data,
            observed_data,
        }
    }
}
