use crate::graph_state::{
    ObservableNodeType, calculate_observed_graph,
    compute_input_statistics, compute_output_statistics,
};
use crate::graph_view::{GraphDisplay, ObservedGraphDisplay};
use crate::heatmap::HeatmapData;
use crate::store::Store;
use crate::versioned::Memoized;
use markov::{Prob, Vector};
use ndarray::linalg::Dot;
use petgraph::{
    Direction,
    stable_graph::NodeIndex,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Validation issues for state graph
#[derive(Debug, Clone)]
pub enum StateValidationIssue {
    NoOutgoingEdges { node: NodeIndex, name: String },
    NoIncomingEdges { node: NodeIndex, name: String },
}

impl std::fmt::Display for StateValidationIssue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StateValidationIssue::NoOutgoingEdges { name, .. } => {
                write!(f, "{} has no outgoing edges", name)
            }
            StateValidationIssue::NoIncomingEdges { name, .. } => {
                write!(f, "{} has no incoming edges", name)
            }
        }
    }
}

/// Validation issues for observable graph
#[derive(Debug, Clone)]
pub enum ObservableValidationIssue {
    SourceNoOutgoingEdges { node: NodeIndex, name: String },
    DestinationNoIncomingEdges { node: NodeIndex, name: String },
}

impl std::fmt::Display for ObservableValidationIssue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ObservableValidationIssue::SourceNoOutgoingEdges { name, .. } => {
                write!(f, "{} has no outgoing edges", name)
            }
            ObservableValidationIssue::DestinationNoIncomingEdges { name, .. } => {
                write!(f, "{} has no incoming edges", name)
            }
        }
    }
}

/// Node ordering for circular layout
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Order(pub Vec<NodeIndex>);

impl Order {
    /// Create an alphabetical order from a graph
    pub fn alphabetical<N, D>(graph: &GraphDisplay<N, D>) -> Self
    where
        N: Clone,
        D: egui_graphs::DisplayNode<N, f32, petgraph::Directed, petgraph::graph::DefaultIx>,
    {
        let mut nodes: Vec<_> = graph
            .nodes_iter()
            .map(|(idx, node)| (idx, node.label().to_string()))
            .collect();
        nodes.sort_by(|a, b| a.1.cmp(&b.1));
        Order(nodes.into_iter().map(|(idx, _)| idx).collect())
    }
}

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
        for (node_idx, _) in distribution.vector.enumerate() {
            labels
                .entry(node_idx)
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
    pub order: Order,
    pub heatmap: HeatmapData,
    pub sorted_weights: Vec<f32>,
    pub weight_distribution: ProbabilityChart,
    pub equilibrium_distribution: Option<ProbabilityChart>,
    pub entropy_rate: Option<f64>,
    pub detailed_balance_deviation: Option<f64>,
    pub validation_errors: Vec<StateValidationIssue>,
}

/// Combined observable data that is calculated together to ensure consistency
pub struct ObservableData {
    pub heatmap: HeatmapData,
    pub sorted_weights: Vec<f32>,
    pub validation_errors: Vec<ObservableValidationIssue>,
}

/// Combined observed data that is calculated together to ensure consistency
pub struct ObservedData {
    pub order: Order,
    pub graph: ObservedGraphDisplay,
    pub heatmap: HeatmapData,
    pub sorted_weights: Vec<f32>,
    pub weight_distribution: ProbabilityChart,
    pub equilibrium_from_state: Option<ProbabilityChart>,
    pub equilibrium_calculated: Option<ProbabilityChart>,
    pub entropy_rate: Option<f64>,
    pub detailed_balance_deviation: Option<f64>,
}

/// Validate state graph for connectivity issues
pub fn validate_state_graph(
    graph: &crate::graph_view::StateGraphDisplay,
) -> Vec<StateValidationIssue> {
    let mut errors = Vec::new();
    let stable = graph.g();

    for node_idx in stable.node_indices() {
        let node_name = stable
            .node_weight(node_idx)
            .map(|n| n.payload().name.clone())
            .unwrap_or_else(|| format!("Node {}", node_idx.index()));

        let mut outgoing = stable.edges(node_idx);
        if outgoing.next().is_none() {
            errors.push(StateValidationIssue::NoOutgoingEdges {
                node: node_idx,
                name: node_name.clone(),
            });
        }

        let mut incoming =
            stable.neighbors_directed(node_idx, Direction::Incoming);
        if incoming.next().is_none() {
            errors.push(StateValidationIssue::NoIncomingEdges {
                node: node_idx,
                name: node_name,
            });
        }
    }

    errors
}

/// Validate observable graph for connectivity issues
pub fn validate_observable_graph(
    graph: &crate::graph_view::ObservableGraphDisplay,
) -> Vec<ObservableValidationIssue> {
    let mut errors = Vec::new();
    let stable = graph.g();

    for (node_idx, node) in graph.nodes_iter() {
        let node_name = node.payload().name.clone();
        match node.payload().node_type {
            ObservableNodeType::Source => {
                let mut outgoing = stable.edges(node_idx);
                if outgoing.next().is_none() {
                    errors.push(
                        ObservableValidationIssue::SourceNoOutgoingEdges {
                            node: node_idx,
                            name: node_name,
                        },
                    );
                }
            }
            ObservableNodeType::Destination => {
                let mut incoming =
                    stable.neighbors_directed(node_idx, Direction::Incoming);
                if incoming.next().is_none() {
                    errors.push(
                        ObservableValidationIssue::DestinationNoIncomingEdges {
                            node: node_idx,
                            name: node_name,
                        },
                    );
                }
            }
        }
    }

    errors
}

pub struct Cache {
    pub state_data: Memoized<Store, u64, StateData>,
    pub observable_data: Memoized<Store, u64, ObservableData>,
    pub observed_data: Memoized<Store, (u64, u64), ObservedData>,
}

impl Cache {
    pub fn new() -> Self {
        let state_data = Memoized::new(
            |s: &Store| s.state.graph.version(),
            |s: &Store| {
                let state_graph = s.state.graph.get();

                // Validate state graph
                let validation_errors = validate_state_graph(state_graph);

                let order = Order::alphabetical(state_graph);
                let heatmap = s.state_heatmap_uncached();
                let sorted_weights =
                    s.state_sorted_weights_uncached();
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

                    Prob::from_vector(Vector::from_assoc(weight_assoc))
                        .unwrap_or_else(|_| {
                            Prob::from_vector(Vector::from_assoc(
                                vec![(NodeIndex::new(0), 1.0)],
                            ))
                            .unwrap()
                        })
                } else {
                    Prob::from_vector(Vector::from_assoc(
                        vec![(NodeIndex::new(0), 1.0)],
                    ))
                    .unwrap()
                };

                let weight_distribution = ProbabilityChart::new(
                    node_stats,
                    node_labels.clone(),
                );

                // Compute equilibrium distribution and statistics for state graph only if validation passes
                let (
                    equilibrium,
                    entropy_rate,
                    detailed_balance_deviation,
                ) = if !validation_errors.is_empty() {
                    // Validation failed - don't compute equilibrium
                    (None, None, None)
                } else if s.state.graph.get().node_count() > 0 {
                    if let Ok(input_stats) = compute_input_statistics(
                        s.state.graph.get(),
                        s.observable.graph.get(),
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
                            .detailed_balance_deviation_sum(&eq);
                        (Some(eq), Some(ent_rate), Some(deviation))
                    } else {
                        // If we can't compute stats, return None
                        (None, None, None)
                    }
                } else {
                    // Empty graph - return None
                    (None, None, None)
                };

                let equilibrium_distribution = equilibrium.map(|eq| {
                    ProbabilityChart::new(eq, node_labels.clone())
                });

                StateData {
                    order,
                    heatmap,
                    sorted_weights,
                    weight_distribution,
                    equilibrium_distribution,
                    entropy_rate,
                    detailed_balance_deviation,
                    validation_errors,
                }
            },
        );

        let observable_data = Memoized::new(
            |s: &Store| s.observable.graph.version(),
            |s: &Store| {
                let observable_graph = s.observable.graph.get();

                // Validate observable graph
                let validation_errors = validate_observable_graph(observable_graph);

                let heatmap = s.observable_heatmap_uncached();
                let sorted_weights =
                    s.observable_sorted_weights_uncached();

                ObservableData {
                    heatmap,
                    sorted_weights,
                    validation_errors,
                }
            },
        );

        let observed_data = Memoized::new(
            |s: &Store| {
                (
                    s.state.graph.version(),
                    s.observable.graph.version(),
                )
            },
            |s: &Store| {
                let state_graph = s.state.graph.get();
                let observable_graph = s.observable.graph.get();

                // Check validation status
                let state_valid = validate_state_graph(state_graph).is_empty();
                let observable_valid = validate_observable_graph(observable_graph).is_empty();
                let validation_passed = state_valid && observable_valid;

                let graph = calculate_observed_graph(
                    state_graph,
                    observable_graph,
                    validation_passed,
                );
                let order = Order::alphabetical(&graph);
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
                    Prob::from_vector(Vector::from_assoc(
                        vec![(NodeIndex::new(0), 1.0)],
                    ))
                    .unwrap()
                };

                let observed_weight_assoc: Vec<(NodeIndex, f64)> =
                    graph
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
                    let prob = Prob::from_vector(Vector::from_assoc(
                        observed_weight_assoc,
                    ))
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

                // Compute equilibrium distributions and statistics only if validation passes
                let (
                    equilibrium_from_state,
                    equilibrium_calculated,
                    entropy_rate,
                    detailed_balance_deviation,
                ) = if !validation_passed {
                    // Validation failed - don't compute equilibria
                    (None, None, None, None)
                } else if state_graph.node_count() > 0 {
                    match compute_input_statistics(
                        s.state.graph.get(),
                        s.observable.graph.get(),
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
                                        .detailed_balance_deviation_sum(
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
                                Some(obs_eq_from_state),
                                Some(obs_eq_calculated),
                                Some(ent_rate),
                                Some(deviation),
                            )
                        }
                        Err(_) => {
                            // Computation failed - return None
                            (None, None, None, None)
                        }
                    }
                } else {
                    // Empty graph - return None
                    (None, None, None, None)
                };

                let equilibrium_from_state = equilibrium_from_state.map(|eq| {
                    ProbabilityChart::new(eq, observed_labels.clone())
                });

                let equilibrium_calculated = equilibrium_calculated.map(|eq| {
                    ProbabilityChart::new(eq, observed_labels.clone())
                });

                ObservedData {
                    order,
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

        Self {
            state_data,
            observable_data,
            observed_data,
        }
    }
}
