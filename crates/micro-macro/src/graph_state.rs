// Graph state module - centralized graph type definitions and operations

use crate::graph_view::{
    GraphDisplay, ObservableGraphDisplay, ObservedGraphDisplay,
    StateGraphDisplay, setup_graph_display,
};
use petgraph::stable_graph::NodeIndex;
use petgraph::stable_graph::StableGraph;
use petgraph::visit::{EdgeRef, IntoEdgeReferences};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use markov::{Markov, Prob, Vector};
use ndarray::linalg::Dot;


// Trait for types that have a name
pub trait HasName {
    fn name(&self) -> String;
}

// StateGraph types
#[derive(Clone)]
pub struct StateNode {
    pub name: String,
    pub weight: f32,
}

impl HasName for StateNode {
    fn name(&self) -> String {
        self.name.clone()
    }
}

pub type StateGraph = StableGraph<StateNode, f32>;

pub fn default_state_graph() -> StateGraph {
    let mut g = StateGraph::new();

    let a = g.add_node(StateNode {
        name: format!("Node {}", 0),
        weight: 1.0,
    });
    let b = g.add_node(StateNode {
        name: format!("Node {}", 1),
        weight: 1.0,
    });
    let c = g.add_node(StateNode {
        name: format!("Node {}", 2),
        weight: 1.0,
    });

    g.add_edge(a, b, 1.0);
    g.add_edge(b, c, 1.0);
    g.add_edge(c, a, 1.0);

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
    #[allow(dead_code)]
    // Will be used for edge computation logic
    pub state_node_idx: Option<NodeIndex>,
}

impl HasName for ObservableNode {
    fn name(&self) -> String {
        self.name.clone()
    }
}

pub type ObservableGraph = StableGraph<ObservableNode, f32>;

pub fn default_observable_graph(
    source_graph: &StateGraph,
) -> ObservableGraph {
    let mut g = ObservableGraph::new();

    // Add Source nodes mirroring the dynamical system
    for (state_idx, node) in
        source_graph.node_indices().zip(source_graph.node_weights())
    {
        g.add_node(ObservableNode {
            name: node.name.clone(),
            node_type: ObservableNodeType::Source,
            state_node_idx: Some(state_idx),
        });
    }

    // Add two default Destination nodes
    g.add_node(ObservableNode {
        name: String::from("Value 0"),
        node_type: ObservableNodeType::Destination,
        state_node_idx: None,
    });
    g.add_node(ObservableNode {
        name: String::from("Value 1"),
        node_type: ObservableNodeType::Destination,
        state_node_idx: None,
    });

    g
}

// ObservedGraph types

#[derive(Clone)]
pub struct ObservedNode {
    pub name: String,
    #[allow(dead_code)] // Will be used for edge computation logic
    pub observable_node_idx: NodeIndex,
    pub weight: f32,
}

impl HasName for ObservedNode {
    fn name(&self) -> String {
        self.name.clone()
    }
}

pub type ObservedGraph = StableGraph<ObservedNode, f32>;

pub fn calculate_observed_graph_new(
    state_graph: &StateGraphDisplay,
    observable_graph: &ObservableGraphDisplay,
) {
    let mut nodes = Vec::new();

    // Create nodes from Destination nodes in the observable graph
    for (idx, node) in observable_graph.nodes_iter() {
        let obs_node = node.payload();
        if obs_node.node_type == ObservableNodeType::Destination {
            nodes.push(ObservedNode {
                name: obs_node.name.clone(),
                observable_node_idx: idx,
                weight: 0.0,
            });
        }
    }
}

pub fn calculate_observed_graph(
    state_graph: &StateGraphDisplay,
    observable_graph: &ObservableGraphDisplay,
) -> ObservedGraphDisplay {
    let observed_stable_graph =
        calculate_observed_graph_from_observable_display(
            observable_graph,
        );

    let mut observed_graph =
        setup_graph_display(&observed_stable_graph);

    match compute_statistics(state_graph, observable_graph) {
        Ok(statistics) => {
            let weights = compute_observed_weights(&statistics, observable_graph);

            let node_updates: Vec<(NodeIndex, NodeIndex, f64)> =
                observed_graph
                    .nodes_iter()
                    .filter_map(|(obs_idx, node)| {
                        let obs_dest_idx =
                            node.payload().observable_node_idx;
                        weights.get(&obs_dest_idx).map(|&weight| {
                            (obs_idx, obs_dest_idx, weight)
                        })
                    })
                    .collect();

            for (obs_idx, _, weight) in node_updates {
                if let Some(node_mut) =
                    observed_graph.node_mut(obs_idx)
                {
                    node_mut.payload_mut().weight = weight as f32;
                }
            }
        }
        Err(e) => {
            eprintln!("Statistics computation error: {}", e);
        }
    }

    observed_graph
}

// Helper function to calculate observed graph from ObservableGraphDisplay
// Works with the concrete display graph type
pub fn calculate_observed_graph_from_observable_display<Dn, De>(
    observable_display: &egui_graphs::Graph<
        ObservableNode,
        f32,
        petgraph::Directed,
        petgraph::graph::DefaultIx,
        Dn,
        De,
    >,
) -> ObservedGraph
where
    Dn: egui_graphs::DisplayNode<
            ObservableNode,
            f32,
            petgraph::Directed,
            petgraph::graph::DefaultIx,
        >,
    De: egui_graphs::DisplayEdge<
            ObservableNode,
            f32,
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
}

#[derive(Clone)]
pub struct Statistics {
    pub state_prob: Prob<NodeIndex,f64>,
    pub state_markov: Markov<NodeIndex,NodeIndex,f64>,
    pub observable_markov: Markov<NodeIndex,NodeIndex,f64>,
}

pub fn compute_statistics(
    state_graph: &StateGraphDisplay,
    observable_graph: &ObservableGraphDisplay,
) -> Result<Statistics, StatisticsError> {
    // 1. Validate state graph not empty
    if state_graph.node_count() == 0 {
        return Err(StatisticsError::EmptyStateGraph);
    }

    // 2. Build Prob from state node weights
    let state_weights: Vec<(NodeIndex, f64)> = state_graph
        .nodes_iter()
        .map(|(idx, node)| (idx, node.payload().weight as f64))
        .collect();

    let state_prob =
        Prob::from_assoc(state_graph.node_count(), state_weights)?;

    // 3. Build state_markov from state graph edges (all nodes)
    let state_g = state_graph.g();
    let state_edges: Vec<(NodeIndex, NodeIndex, f64)> = state_g
        .edge_references()
        .map(|e| {
            (e.source(), e.target(), *e.weight().payload() as f64)
        })
        .collect();

    let state_markov = Markov::from_assoc(
        state_graph.node_count(),
        state_graph.node_count(),
        state_edges,
    )?;

    // 4. Build observable_markov from observable edges (source -> destination)
    let dest_nodes: Vec<NodeIndex> = observable_graph
        .nodes_iter()
        .filter(|(_, node)| {
            node.payload().node_type
                == ObservableNodeType::Destination
        })
        .map(|(idx, _)| idx)
        .collect();

    // Get edges from the underlying petgraph
    let observable_g = observable_graph.g();
    let observable_edges: Vec<(NodeIndex, NodeIndex, f64)> = observable_g
        .edge_references()
        .map(|e| {
            (e.source(), e.target(), *e.weight().payload() as f64)
        })
        .collect();

    let observable_markov = Markov::from_assoc(
        state_graph.node_count(),
        dest_nodes.len(),
        observable_edges,
    )?;

    Ok(Statistics {
        state_prob,
        state_markov,
        observable_markov,
    })
}

pub fn compute_observed_weights(
    statistics: &Statistics,
    observable_graph: &ObservableGraphDisplay,
) -> HashMap<NodeIndex, f64> {
    let observed_prob: Prob<NodeIndex, f64> = statistics.state_prob.dot(&statistics.observable_markov);

    observed_prob
        .enumerate()
        .collect()
}
