// Graph state module - centralized graph type definitions and operations

use crate::graph_view::{StateGraphDisplay, ObservableGraphDisplay, ObservedGraphDisplay, setup_graph_display};
use petgraph::stable_graph::NodeIndex;
use petgraph::stable_graph::StableGraph;
use petgraph::visit::{EdgeRef, IntoEdgeReferences};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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

pub fn calculate_observed_graph(
    state_graph: &StateGraphDisplay,
    observable_graph: &ObservableGraphDisplay,
) -> ObservedGraphDisplay {
    let observed_stable_graph =
        calculate_observed_graph_from_observable_display(
            observable_graph,
        );

    let mut observed_graph = setup_graph_display(&observed_stable_graph);

    match compute_observed_weights(state_graph, observable_graph) {
        Ok(weights) => {
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
            eprintln!("Weight computation error: {}", e);
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
pub enum WeightComputationError {
    #[error("state graph is empty")]
    EmptyStateGraph,
    #[error("probability construction failed: {0}")]
    ProbError(#[from] markov::prob::BuildError),
}

pub fn compute_observed_weights<Dn1, De1, Dn2, De2>(
    state_graph: &egui_graphs::Graph<
        StateNode,
        f32,
        petgraph::Directed,
        petgraph::graph::DefaultIx,
        Dn1,
        De1,
    >,
    observable_graph: &egui_graphs::Graph<
        ObservableNode,
        f32,
        petgraph::Directed,
        petgraph::graph::DefaultIx,
        Dn2,
        De2,
    >,
) -> Result<HashMap<NodeIndex, f64>, WeightComputationError>
where
    Dn1: egui_graphs::DisplayNode<
            StateNode,
            f32,
            petgraph::Directed,
            petgraph::graph::DefaultIx,
        >,
    De1: egui_graphs::DisplayEdge<
            StateNode,
            f32,
            petgraph::Directed,
            petgraph::graph::DefaultIx,
            Dn1,
        >,
    Dn2: egui_graphs::DisplayNode<
            ObservableNode,
            f32,
            petgraph::Directed,
            petgraph::graph::DefaultIx,
        >,
    De2: egui_graphs::DisplayEdge<
            ObservableNode,
            f32,
            petgraph::Directed,
            petgraph::graph::DefaultIx,
            Dn2,
        >,
{
    use markov::{Markov, Prob};
    use ndarray::linalg::Dot;

    // 1. Validate state graph not empty
    if state_graph.node_count() == 0 {
        return Err(WeightComputationError::EmptyStateGraph);
    }

    // 2. Build Prob from state node weights
    let state_weights: Vec<(NodeIndex, f64)> = state_graph
        .nodes_iter()
        .map(|(idx, node)| (idx, node.payload().weight as f64))
        .collect();
    let prob =
        Prob::from_assoc(state_graph.node_count(), state_weights)?;

    // 3. Build Markov from observable edges (source -> destination)
    let dest_nodes: Vec<NodeIndex> = observable_graph
        .nodes_iter()
        .filter(|(_, node)| {
            node.payload().node_type
                == ObservableNodeType::Destination
        })
        .map(|(idx, _)| idx)
        .collect();

    if dest_nodes.is_empty() {
        return Ok(HashMap::new());
    }

    // Get edges from the underlying petgraph
    let stable_g = observable_graph.g();
    let edges: Vec<(NodeIndex, NodeIndex, f64)> = stable_g
        .edge_references()
        .map(|e| {
            (e.source(), e.target(), *e.weight().payload() as f64)
        })
        .collect();

    let markov = Markov::from_assoc(
        state_graph.node_count(),
        dest_nodes.len(),
        edges,
    )?;

    // 4. Compute prob.dot(markov)
    let observed_prob: Prob<NodeIndex, f64> = prob.dot(&markov);

    // 5. Extract weights for destination nodes
    let mut result = HashMap::new();
    for &dest_idx in &dest_nodes {
        if let Some(weight) = observed_prob.prob(&dest_idx) {
            result.insert(dest_idx, weight);
        }
    }

    Ok(result)
}
