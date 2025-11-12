use crate::graph_state::{
    ObservableNodeType, calculate_observed_graph,
    compute_input_statistics, compute_output_statistics,
};
use crate::graph_view::ObservedGraphDisplay;
use crate::heatmap::HeatmapData;
use crate::store::Store;
use crate::versioned::Memoized;
use markov::Prob;
use ndarray::linalg::Dot;
use petgraph::{
    Direction,
    stable_graph::NodeIndex,
    visit::{EdgeRef, IntoEdgeReferences},
};
use std::collections::{HashMap, HashSet};

pub struct ProbabilityChart {
    pub labels: HashMap<NodeIndex, String>,
    pub distribution: Prob<NodeIndex, f64>,
    pub entropy: f64,
    pub effective_states: f64,
}

impl ProbabilityChart {
    pub fn new(
        distribution: Prob<NodeIndex, f64>,
        mut labels: HashMap<NodeIndex, String>,
    ) -> Self {
        if labels.is_empty() {
            labels.insert(NodeIndex::new(0), "Node 0".to_string());
        }

        // Ensure every index present in the probability map has a label.
        for (_, node_idx) in distribution.map.iter() {
            labels
                .entry(*node_idx)
                .or_insert_with(|| format!("Node {:?}", node_idx));
        }

        let entropy = distribution.entropy();
        let effective_states = distribution.effective_states();

        Self {
            labels,
            distribution,
            entropy,
            effective_states,
        }
    }
}

/// Combined state data that is calculated together to ensure consistency
pub struct StateData {
    pub heatmap: HeatmapData,
    pub sorted_weights: Vec<f32>,
    pub weight_distribution: ProbabilityChart,
    pub equilibrium_distribution: ProbabilityChart,
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
    pub weight_distribution: ProbabilityChart,
    pub equilibrium_from_state: ProbabilityChart,
    pub equilibrium_calculated: ProbabilityChart,
    pub entropy_rate: f64,
    pub detailed_balance_deviation: f64,
}

pub struct ValidationErrors {
    pub state: Vec<String>,
    pub observable: Vec<String>,
}

pub struct Cache {
    pub state_data: Memoized<Store, u64, StateData>,
    pub observable_data: Memoized<Store, u64, ObservableData>,
    pub observed_data: Memoized<Store, (u64, u64), ObservedData>,
    pub validation_errors:
        Memoized<Store, (u64, u64, u64), ValidationErrors>,
}

impl Cache {
    pub fn new() -> Self {
        let state_data = Memoized::new(
            |s: &Store| s.state_graph.version(),
            |s: &Store| {
                let heatmap = s.state_heatmap_uncached();
                let sorted_weights =
                    s.state_sorted_weights_uncached();

                let state_graph = s.state_graph.get();
                let node_count = state_graph.node_count();
                let node_labels: HashMap<NodeIndex, String> =
                    state_graph
                        .nodes_iter()
                        .map(|(idx, node)| {
                            (idx, node.payload().name.clone())
                        })
                        .collect();

                // Compute weight distribution
                let node_stats = if node_count > 0 {
                    let stats = s.state_node_weight_stats();
                    let weight_assoc: Vec<(NodeIndex, f64)> =
                        state_graph
                            .nodes_iter()
                            .filter_map(|(idx, node)| {
                                stats
                                    .iter()
                                    .find(|(name, _)| {
                                        name == &node.payload().name
                                    })
                                    .map(|(_, weight)| {
                                        (idx, *weight as f64)
                                    })
                            })
                            .collect();

                    Prob::from_assoc(node_count, weight_assoc)
                        .unwrap_or_else(|_| {
                            Prob::from_assoc(
                                1,
                                vec![(NodeIndex::new(0), 1.0)],
                            )
                            .unwrap()
                        })
                } else {
                    Prob::from_assoc(
                        1,
                        vec![(NodeIndex::new(0), 1.0)],
                    )
                    .unwrap()
                };

                let weight_distribution = ProbabilityChart::new(
                    node_stats,
                    node_labels.clone(),
                );

                // Compute equilibrium distribution and statistics for state graph
                let (
                    equilibrium,
                    entropy_rate,
                    detailed_balance_deviation,
                ) = if s.state_graph.get().node_count() > 0 {
                    if let Ok(input_stats) = compute_input_statistics(
                        s.state_graph.get(),
                        s.observable_graph.get(),
                    ) {
                        let eq = input_stats
                            .state_markov
                            .compute_equilibrium(
                                &input_stats.state_prob,
                                1e-4,
                                100,
                            );
                        let ent_rate = input_stats
                            .state_markov
                            .entropy_rate(&eq);
                        let deviation = input_stats
                            .state_markov
                            .detailed_balance_deviation(&eq);
                        (eq, ent_rate, deviation)
                    } else {
                        // If we can't compute stats, return uniform distribution with default stats
                        let node_count =
                            s.state_graph.get().node_count();
                        let indices: Vec<_> = s
                            .state_graph
                            .get()
                            .nodes_iter()
                            .map(|(idx, _)| idx)
                            .collect();
                        let eq = Prob::from_assoc(
                            node_count,
                            indices.into_iter().map(|idx| (idx, 1.0)),
                        )
                        .unwrap_or_else(|_| {
                            Prob::from_assoc(
                                1,
                                vec![(NodeIndex::new(0), 1.0)],
                            )
                            .unwrap()
                        });
                        (eq, 0.0, 0.0)
                    }
                } else {
                    // Empty graph: create a minimal valid Prob with default stats
                    let eq = Prob::from_assoc(
                        1,
                        vec![(NodeIndex::new(0), 1.0)],
                    )
                    .unwrap();
                    (eq, 0.0, 0.0)
                };

                let equilibrium_distribution = ProbabilityChart::new(
                    equilibrium,
                    node_labels.clone(),
                );

                StateData {
                    heatmap,
                    sorted_weights,
                    weight_distribution,
                    equilibrium_distribution,
                    entropy_rate,
                    detailed_balance_deviation,
                }
            },
        );

        let observable_data = Memoized::new(
            |s: &Store| s.observable_graph.version(),
            |s: &Store| {
                let heatmap = s.observable_heatmap_uncached();
                let sorted_weights =
                    s.observable_sorted_weights_uncached();

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
                let observed_labels: HashMap<NodeIndex, String> =
                    graph
                        .nodes_iter()
                        .map(|(_, node)| {
                            (
                                node.payload().observable_node_idx,
                                node.payload().name.clone(),
                            )
                        })
                        .collect();

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

                let prob_fallback = || {
                    Prob::from_assoc(
                        1,
                        vec![(NodeIndex::new(0), 1.0)],
                    )
                    .unwrap()
                };

                let node_count = graph.node_count().max(1);
                let observed_weight_assoc: Vec<(NodeIndex, f64)> = graph
                    .nodes_iter()
                    .map(|(_, node)| {
                        (
                            node.payload().observable_node_idx,
                            node.payload().weight as f64,
                        )
                    })
                    .collect();

                let total_weight: f64 = observed_weight_assoc
                    .iter()
                    .map(|(_, weight)| *weight)
                    .sum();

                let weight_distribution = if total_weight > 0.0 {
                    let prob = Prob::from_assoc(
                        node_count,
                        observed_weight_assoc,
                    )
                    .unwrap_or_else(|_| prob_fallback());
                    ProbabilityChart::new(
                        prob,
                        observed_labels.clone(),
                    )
                } else {
                    ProbabilityChart::new(
                        prob_fallback(),
                        observed_labels.clone(),
                    )
                };

                // Compute equilibrium distributions and statistics
                let (
                    equilibrium_from_state,
                    equilibrium_calculated,
                    entropy_rate,
                    detailed_balance_deviation,
                ) = if s.state_graph.get().node_count() > 0 {
                    match compute_input_statistics(
                        s.state_graph.get(),
                        s.observable_graph.get(),
                    ) {
                        Ok(input_stats) => {
                            // 1. State equilibrium
                            let state_eq = input_stats
                                .state_markov
                                .compute_equilibrium(
                                    &input_stats.state_prob,
                                    1e-4,
                                    100,
                                );

                            // 2. Observed equilibrium = state_eq · observable_markov
                            let obs_eq_from_state = state_eq
                                .dot(&input_stats.observable_markov);

                            // 3. Calculated observed equilibrium and statistics
                            let (
                                obs_eq_calculated,
                                ent_rate,
                                deviation,
                            ) = match compute_output_statistics(
                                &input_stats,
                            ) {
                                Ok(output_stats) => {
                                    let eq_calc = output_stats
                                        .observed_markov
                                        .compute_equilibrium(
                                            &output_stats
                                                .observed_prob,
                                            1e-4,
                                            100,
                                        );
                                    let ent_r = output_stats
                                        .observed_markov
                                        .entropy_rate(&eq_calc);
                                    let dev = output_stats
                                        .observed_markov
                                        .detailed_balance_deviation(
                                            &eq_calc,
                                        );
                                    (eq_calc, ent_r, dev)
                                }
                                Err(_) => {
                                    // Fallback to observed_prob if calculation fails
                                    (
                                        obs_eq_from_state.clone(),
                                        0.0,
                                        0.0,
                                    )
                                }
                            };

                            (
                                obs_eq_from_state,
                                obs_eq_calculated,
                                ent_rate,
                                deviation,
                            )
                        }
                        Err(_) => {
                            // Create fallback distributions
                            let node_count =
                                graph.node_count().max(1);
                            let indices: Vec<_> = graph
                                .nodes_iter()
                                .map(|(idx, _)| idx)
                                .collect();
                            let fallback = Prob::from_assoc(
                                node_count,
                                indices
                                    .into_iter()
                                    .map(|idx| (idx, 1.0)),
                            )
                            .unwrap_or_else(|_| {
                                Prob::from_assoc(
                                    1,
                                    vec![(NodeIndex::new(0), 1.0)],
                                )
                                .unwrap()
                            });
                            (fallback.clone(), fallback, 0.0, 0.0)
                        }
                    }
                } else {
                    // Empty graph fallback
                    let fallback = Prob::from_assoc(
                        1,
                        vec![(NodeIndex::new(0), 1.0)],
                    )
                    .unwrap();
                    (fallback.clone(), fallback, 0.0, 0.0)
                };

                let equilibrium_from_state = ProbabilityChart::new(
                    equilibrium_from_state,
                    observed_labels.clone(),
                );

                let equilibrium_calculated = ProbabilityChart::new(
                    equilibrium_calculated,
                    observed_labels.clone(),
                );

                ObservedData {
                    graph,
                    heatmap,
                    sorted_weights: weights,
                    weight_distribution,
                    equilibrium_from_state,
                    equilibrium_calculated,
                    entropy_rate,
                    detailed_balance_deviation,
                }
            },
        );
        let validation_errors = Memoized::new(
            |s: &Store| s.validation_error_key(),
            |s: &Store| {
                let mut state_messages =
                    s.validation_event_messages_state().to_vec();
                let mut observable_messages =
                    s.validation_event_messages_observable().to_vec();
                let state_graph = s.state_graph.get();
                let stable = state_graph.g();
                let mut seen_pairs = HashSet::new();

                for edge_ref in stable.edge_references() {
                    let key = (edge_ref.source(), edge_ref.target());
                    if !seen_pairs.insert(key) {
                        state_messages.push(format!(
                            "Duplicate edge detected: {} -> {}",
                            s.state_node_name(edge_ref.source()),
                            s.state_node_name(edge_ref.target()),
                        ));
                    }
                }

                for node_idx in stable.node_indices() {
                    let mut outgoing = stable.edges(node_idx);
                    if outgoing.next().is_none() {
                        state_messages.push(format!(
                            "{} has no outgoing edges",
                            s.state_node_name(node_idx),
                        ));
                    }
                    let mut incoming = stable.neighbors_directed(
                        node_idx,
                        Direction::Incoming,
                    );
                    if incoming.next().is_none() {
                        state_messages.push(format!(
                            "{} has no incoming edges",
                            s.state_node_name(node_idx),
                        ));
                    }
                }

                let observable_display = s.observable_graph.get();
                let observable_stable = observable_display.g();
                for (node_idx, node) in
                    observable_display.nodes_iter()
                {
                    match node.payload().node_type {
                        ObservableNodeType::Source => {
                            let mut outgoing =
                                observable_stable.edges(node_idx);
                            if outgoing.next().is_none() {
                                observable_messages.push(format!(
                                    "{} has no outgoing edges",
                                    node.payload().name
                                ));
                            }
                        }
                        ObservableNodeType::Destination => {
                            let mut incoming = observable_stable
                                .neighbors_directed(
                                    node_idx,
                                    Direction::Incoming,
                                );
                            if incoming.next().is_none() {
                                observable_messages.push(format!(
                                    "{} has no incoming edges",
                                    node.payload().name
                                ));
                            }
                        }
                    }
                }

                ValidationErrors {
                    state: state_messages,
                    observable: observable_messages,
                }
            },
        );

        Self {
            state_data,
            observable_data,
            observed_data,
            validation_errors,
        }
    }
}
