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
    pub entropy: f64,
    pub effective_states: f64,
    pub entropy_rate: f64,
    pub detailed_balance_deviation: f64,
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
    pub entropy_from_state: f64,
    pub effective_states_from_state: f64,
    pub entropy_calculated: f64,
    pub effective_states_calculated: f64,
    pub entropy_rate: f64,
    pub detailed_balance_deviation: f64,
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

                // Compute equilibrium distribution and statistics for state graph
                let (equilibrium, entropy, effective_states, entropy_rate, detailed_balance_deviation) =
                    if s.state_graph.get().node_count() > 0 {
                        if let Ok(input_stats) = compute_input_statistics(
                            s.state_graph.get(),
                            s.observable_graph.get(),
                        ) {
                            let eq = input_stats.state_markov.compute_equilibrium(
                                &input_stats.state_prob,
                                1e-4,
                                100,
                            );
                            let ent = eq.entropy();
                            let eff = eq.effective_states();
                            let ent_rate = input_stats.state_markov.entropy_rate(&eq);
                            let deviation = input_stats.state_markov.detailed_balance_deviation(&eq);
                            (eq, ent, eff, ent_rate, deviation)
                        } else {
                            // If we can't compute stats, return uniform distribution with default stats
                            let node_count = s.state_graph.get().node_count();
                            let indices: Vec<_> = s.state_graph.get().nodes_iter().map(|(idx, _)| idx).collect();
                            let eq = Prob::from_assoc(
                                node_count,
                                indices.into_iter().map(|idx| (idx, 1.0)),
                            ).unwrap_or_else(|_| {
                                Prob::from_assoc(1, vec![(NodeIndex::new(0), 1.0)]).unwrap()
                            });
                            let ent = eq.entropy();
                            let eff = eq.effective_states();
                            (eq, ent, eff, 0.0, 0.0)
                        }
                    } else {
                        // Empty graph: create a minimal valid Prob with default stats
                        let eq = Prob::from_assoc(1, vec![(NodeIndex::new(0), 1.0)]).unwrap();
                        let ent = eq.entropy();
                        let eff = eq.effective_states();
                        (eq, ent, eff, 0.0, 0.0)
                    };

                StateData {
                    heatmap,
                    sorted_weights,
                    equilibrium,
                    entropy,
                    effective_states,
                    entropy_rate,
                    detailed_balance_deviation,
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

                // Compute equilibrium distributions and statistics
                let (
                    equilibrium_from_state,
                    equilibrium_calculated,
                    entropy_from_state,
                    effective_states_from_state,
                    entropy_calculated,
                    effective_states_calculated,
                    entropy_rate,
                    detailed_balance_deviation,
                ) = if s.state_graph.get().node_count() > 0 {
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
                            let ent_from_state = obs_eq_from_state.entropy();
                            let eff_from_state = obs_eq_from_state.effective_states();

                            // 3. Calculated observed equilibrium and statistics
                            let (obs_eq_calculated, ent_calc, eff_calc, ent_rate, deviation) =
                                match compute_output_statistics(&input_stats) {
                                    Ok(output_stats) => {
                                        let eq_calc = output_stats.observed_markov.compute_equilibrium(
                                            &output_stats.observed_prob,
                                            1e-4,
                                            100,
                                        );
                                        let ent_c = eq_calc.entropy();
                                        let eff_c = eq_calc.effective_states();
                                        let ent_r = output_stats.observed_markov.entropy_rate(&eq_calc);
                                        let dev = output_stats.observed_markov.detailed_balance_deviation(&eq_calc);
                                        (eq_calc, ent_c, eff_c, ent_r, dev)
                                    }
                                    Err(_) => {
                                        // Fallback to observed_prob if calculation fails
                                        let ent_c = obs_eq_from_state.entropy();
                                        let eff_c = obs_eq_from_state.effective_states();
                                        (obs_eq_from_state.clone(), ent_c, eff_c, 0.0, 0.0)
                                    }
                                };

                            (
                                obs_eq_from_state,
                                obs_eq_calculated,
                                ent_from_state,
                                eff_from_state,
                                ent_calc,
                                eff_calc,
                                ent_rate,
                                deviation,
                            )
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
                            let ent = fallback.entropy();
                            let eff = fallback.effective_states();
                            (fallback.clone(), fallback, ent, eff, ent, eff, 0.0, 0.0)
                        }
                    }
                } else {
                    // Empty graph fallback
                    let fallback = Prob::from_assoc(1, vec![(NodeIndex::new(0), 1.0)]).unwrap();
                    let ent = fallback.entropy();
                    let eff = fallback.effective_states();
                    (fallback.clone(), fallback, ent, eff, ent, eff, 0.0, 0.0)
                };

                ObservedData {
                    graph,
                    heatmap,
                    sorted_weights: weights,
                    equilibrium_from_state,
                    equilibrium_calculated,
                    entropy_from_state,
                    effective_states_from_state,
                    entropy_calculated,
                    effective_states_calculated,
                    entropy_rate,
                    detailed_balance_deviation,
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
