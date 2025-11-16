// Graph state module - centralized graph type definitions and operations

use crate::graph_view::{
    ObservableGraphDisplay, ObservedGraphDisplay, StateGraphDisplay, setup_observed_graph_display,
};
use markov::{Markov, Matrix, Prob, Vector};
use ndarray::linalg::Dot;
use petgraph::stable_graph::NodeIndex;
use petgraph::stable_graph::StableGraph;
use petgraph::visit::{EdgeRef, IntoEdgeReferences};
use serde::{Deserialize, Serialize};

// Trait for types that have a name
pub trait HasName {
    fn name(&self) -> String;
}

// StateGraph types
#[derive(Clone)]
pub struct StateNode {
    pub name: String,
    pub weight: f64,
}

impl HasName for StateNode {
    fn name(&self) -> String {
        self.name.clone()
    }
}

pub type StateGraph = StableGraph<StateNode, f64>;

pub fn default_state_graph() -> StateGraph {
    let mut g = StateGraph::new();
    let make_name = |n: u32| format!("State {}", n);

    let s_1 = g.add_node(StateNode {
        name: make_name(1),
        weight: 1.0,
    });
    let s_2 = g.add_node(StateNode {
        name: make_name(2),
        weight: 3.0,
    });
    let s_3 = g.add_node(StateNode {
        name: make_name(3),
        weight: 2.0,
    });

    g.add_edge(s_1, s_2, 1.0);
    g.add_edge(s_2, s_1, 2.0);
    g.add_edge(s_2, s_3, 1.0);
    g.add_edge(s_3, s_1, 1.0);

    g
}

// ObservableGraph types
#[derive(Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ObservableNodeType {
    Source,
    Destination,
}

#[derive(Clone)]
pub struct ObservableNode {
    pub name: String,
    pub node_type: ObservableNodeType,
    /// Reference to the corresponding StateGraph node for Source nodes
    /// None for Destination nodes
    // Will be used for edge computation logic
    pub state_node_idx: Option<NodeIndex>,
}

impl HasName for ObservableNode {
    fn name(&self) -> String {
        self.name.clone()
    }
}

pub type ObservableGraph = StableGraph<ObservableNode, f64>;

pub fn default_observable_graph(source_graph: &StateGraph) -> ObservableGraph {
    let mut g = ObservableGraph::new();
    let mut state_nodes = Vec::new();

    // Add Source nodes mirroring the dynamical system
    for (state_idx, node) in source_graph.node_indices().zip(source_graph.node_weights()) {
        let s = g.add_node(ObservableNode {
            name: node.name.clone(),
            node_type: ObservableNodeType::Source,
            state_node_idx: Some(state_idx),
        });

        state_nodes.push(s);
    }

    // Add two default Destination nodes
    let t_1 = g.add_node(ObservableNode {
        name: String::from("Value 0"),
        node_type: ObservableNodeType::Destination,
        state_node_idx: None,
    });
    let t_2 = g.add_node(ObservableNode {
        name: String::from("Value 1"),
        node_type: ObservableNodeType::Destination,
        state_node_idx: None,
    });

    g.add_edge(state_nodes[0], t_1, 1.0);
    g.add_edge(state_nodes[1], t_1, 2.0);
    g.add_edge(state_nodes[1], t_2, 1.0);
    g.add_edge(state_nodes[2], t_2, 1.0);

    g
}

// ObservedGraph types

#[derive(Clone)]
pub struct ObservedNode {
    pub name: String,
    pub observable_node_idx: NodeIndex,
    pub weight: f64,
}

impl HasName for ObservedNode {
    fn name(&self) -> String {
        self.name.clone()
    }
}

pub type ObservedGraph = StableGraph<ObservedNode, f64>;

pub fn calculate_observed_graph(
    state_graph: &StateGraphDisplay,
    observable_graph: &ObservableGraphDisplay,
    validation_passed: bool,
) -> ObservedGraphDisplay {
    let observed_stable_graph = calculate_observed_graph_from_observable_display(observable_graph);

    let mut observed_graph = setup_observed_graph_display(&observed_stable_graph);

    // Only compute edges if validation passed
    if !validation_passed {
        return observed_graph;
    }

    match compute_input_statistics(state_graph, observable_graph) {
        Ok(input_stats) => {
            match compute_output_statistics(&input_stats) {
                Ok(output_stats) => {
                    // Update node weights from observed_prob
                    let node_updates: Vec<(NodeIndex, NodeIndex, f64)> = observed_graph
                        .nodes_iter()
                        .filter_map(|(obs_idx, node)| {
                            let obs_dest_idx = node.payload().observable_node_idx;
                            output_stats
                                .observed_prob
                                .prob(&obs_dest_idx)
                                .map(|weight| (obs_idx, obs_dest_idx, weight))
                        })
                        .collect();

                    for (obs_idx, _, weight) in node_updates {
                        if let Some(node_mut) = observed_graph.node_mut(obs_idx) {
                            node_mut.payload_mut().weight = weight;
                        }
                    }

                    // Update edge weights from observed_markov
                    // Create a mapping from observable node indices to observed graph node indices
                    let obs_to_observed_idx: std::collections::HashMap<NodeIndex, NodeIndex> =
                        observed_graph
                            .nodes_iter()
                            .map(|(obs_idx, node)| (node.payload().observable_node_idx, obs_idx))
                            .collect();

                    // Add edges based on observed_markov transitions
                    for (source_obs_idx, target_obs_idx, weight) in
                        output_stats.observed_markov.enumerate()
                    {
                        // Skip edges with negligible weight
                        if weight.abs() < 1e-10 {
                            continue;
                        }

                        // Map observable indices to observed graph indices
                        if let (Some(&source_idx), Some(&target_idx)) = (
                            obs_to_observed_idx.get(&source_obs_idx),
                            obs_to_observed_idx.get(&target_obs_idx),
                        ) {
                            observed_graph.add_edge(source_idx, target_idx, weight);
                        }
                    }

                    // Clear edge labels after all edges have been added
                    let edge_indices: Vec<_> =
                        observed_graph.edges_iter().map(|(idx, _)| idx).collect();
                    for edge_idx in edge_indices {
                        if let Some(edge) = observed_graph.edge_mut(edge_idx) {
                            edge.set_label(String::new());
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Output statistics computation error: {}", e);
                }
            }
        }
        Err(e) => {
            eprintln!("Input statistics computation error: {}", e);
        }
    }

    observed_graph
}

// Helper function to calculate observed graph from ObservableGraphDisplay
// Works with the concrete display graph type
pub fn calculate_observed_graph_from_observable_display<Dn, De>(
    observable_display: &egui_graphs::Graph<
        ObservableNode,
        f64,
        petgraph::Directed,
        petgraph::graph::DefaultIx,
        Dn,
        De,
    >,
) -> ObservedGraph
where
    Dn: egui_graphs::DisplayNode<
            ObservableNode,
            f64,
            petgraph::Directed,
            petgraph::graph::DefaultIx,
        >,
    De: egui_graphs::DisplayEdge<
            ObservableNode,
            f64,
            petgraph::Directed,
            petgraph::graph::DefaultIx,
            Dn,
        >,
{
    let mut g = ObservedGraph::new();

    // Create nodes from Destination nodes in the observable graph
    for (idx, node) in observable_display.nodes_iter() {
        let obs_node = node.payload();
        if obs_node.node_type == ObservableNodeType::Destination {
            g.add_node(ObservedNode {
                name: obs_node.name.clone(),
                observable_node_idx: idx,
                weight: 0.0,
            });
        }
    }

    // TODO: Implement edge computation logic
    // Edges will be computed based on state transitions and observable relationships
    // This is left as placeholder for future user implementation

    g
}

// ------------------------------------------------------------------
// Weight Computation
// ------------------------------------------------------------------

#[derive(thiserror::Error, Debug)]
pub enum StatisticsError {
    #[error("state graph is empty")]
    EmptyStateGraph,
    #[error("probability construction failed: {0}")]
    ProbError(#[from] markov::prob::BuildError),
    #[error("markov construction failed: {0}")]
    MarkovError(#[from] markov::markov::BuildError),
}

#[derive(Clone)]
pub struct InputStatistics {
    pub state_prob: Prob<NodeIndex>,
    pub state_markov: Markov<NodeIndex, NodeIndex>,
    pub observable_markov: Markov<NodeIndex, NodeIndex>,
}

pub fn compute_input_statistics(
    state_graph: &StateGraphDisplay,
    observable_graph: &ObservableGraphDisplay,
) -> Result<InputStatistics, StatisticsError> {
    // 1. Validate state graph not empty
    if state_graph.node_count() == 0 {
        return Err(StatisticsError::EmptyStateGraph);
    }

    // 2. Build Prob from state node weights
    let state_weights: Vec<(NodeIndex, f64)> = state_graph
        .nodes_iter()
        .map(|(idx, node)| (idx, node.payload().weight))
        .collect();

    let state_prob = Prob::from_vector(Vector::from_assoc(state_weights))?;

    // 3. Build state_markov from state graph edges (all nodes)
    let state_g = state_graph.g();
    let state_edges: Vec<(NodeIndex, NodeIndex, f64)> = state_g
        .edge_references()
        .map(|e| (e.source(), e.target(), (*e.weight().payload())))
        .collect();

    let state_markov = Markov::from_matrix(Matrix::from_assoc(state_edges))?;

    // 4. Build observable_markov from observable edges (source -> destination)
    // Get edges from the underlying petgraph
    let observable_g = observable_graph.g();
    let observable_edges: Vec<(NodeIndex, NodeIndex, f64)> = observable_g
        .edge_references()
        .map(|e| (e.source(), e.target(), (*e.weight().payload())))
        .collect();

    let observable_markov = Markov::from_matrix(Matrix::from_assoc(observable_edges))?;

    Ok(InputStatistics {
        state_prob,
        state_markov,
        observable_markov,
    })
}

#[derive(Clone)]
pub struct OutputStatistics {
    pub observed_prob: Prob<NodeIndex>,
    pub observed_markov: Markov<NodeIndex, NodeIndex>,
}

pub fn compute_output_statistics(
    input_statistics: &InputStatistics,
) -> Result<OutputStatistics, StatisticsError> {
    // Compute observed probability: p · F
    let observed_prob: Prob<NodeIndex> = input_statistics
        .state_prob
        .dot(&input_statistics.observable_markov);

    // Compute observed Markov transitions: Φ^f
    let observed_markov = compute_observable_markov(input_statistics)?;

    Ok(OutputStatistics {
        observed_prob,
        observed_markov,
    })
}

pub fn compute_observable_markov(
    statistics: &InputStatistics,
) -> Result<Markov<NodeIndex, NodeIndex>, StatisticsError> {
    // Extract destination observable node indices (columns of observable_markov)
    let dest_nodes: Vec<NodeIndex> = (0..statistics.observable_markov.matrix.y_ix_map.len())
        .filter_map(|i| {
            statistics
                .observable_markov
                .matrix
                .y_ix_map
                .value_of(i)
                .cloned()
        })
        .collect();

    if dest_nodes.is_empty() {
        return Err(StatisticsError::EmptyStateGraph); // reuse error for now
    }

    // Convert state_prob to Vector for element-wise operations
    let p_vec = statistics.state_prob.to_vec();

    // Collect all (y', y, value) triplets for the observable transition matrix
    let mut triplets: Vec<(NodeIndex, NodeIndex, f64)> = Vec::new();

    for &y in &dest_nodes {
        // Get column F_y from observable_markov
        let f_y = statistics
            .observable_markov
            .matrix
            .get_column(&y)
            .ok_or(StatisticsError::EmptyStateGraph)?; // Column should exist

        // Compute pF_y = p ⊙ F_y (element-wise multiplication)
        let pf_y = &p_vec * &f_y;

        // Compute denominator: p · F_y
        let denominator = p_vec.dot(&f_y);

        // Skip if denominator is zero (undefined transition)
        if denominator.abs() < 1e-10 {
            continue;
        }

        for &y_prime in &dest_nodes {
            // Get column F_{y'}
            let f_y_prime = statistics
                .observable_markov
                .matrix
                .get_column(&y_prime)
                .ok_or(StatisticsError::EmptyStateGraph)?;

            // Compute numerator: (pF_y) · Φ · F_{y'}
            // First: pF_y · Φ (left multiply matrix by vector)
            let temp_vec = pf_y.dot(&statistics.state_markov.matrix);

            // Then: temp_vec · F_{y'} (dot product)
            let numerator = temp_vec.dot(&f_y_prime);

            // Compute the entry: Φ^f_{y' y}
            let value = numerator / denominator;

            triplets.push((y, y_prime, value));
        }
    }

    // Build the observable Markov matrix from triplets
    let observable_transition = Markov::from_matrix(Matrix::from_assoc(triplets))?;

    Ok(observable_transition)
}
