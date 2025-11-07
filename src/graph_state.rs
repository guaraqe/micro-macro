// Graph state module - centralized graph type definitions and operations

use petgraph::stable_graph::StableGraph;
use serde::{Deserialize, Serialize};

// Trait for types that have a name
pub trait HasName {
    fn name(&self) -> String;
}

// StateGraph types
#[derive(Clone)]
pub struct StateNode {
    pub name: String,
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
    });
    let b = g.add_node(StateNode {
        name: format!("Node {}", 1),
    });
    let c = g.add_node(StateNode {
        name: format!("Node {}", 2),
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
    #[allow(dead_code)] // Will be used for edge computation logic
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
    for (state_idx, node) in source_graph.node_indices().zip(source_graph.node_weights()) {
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
use petgraph::stable_graph::NodeIndex;

#[derive(Clone)]
pub struct ObservedNode {
    pub name: String,
    #[allow(dead_code)] // Will be used for edge computation logic
    pub observable_node_idx: NodeIndex,
}

impl HasName for ObservedNode {
    fn name(&self) -> String {
        self.name.clone()
    }
}

pub type ObservedGraph = StableGraph<ObservedNode, f32>;

#[allow(dead_code)] // Alternative interface, kept for potential future use
pub fn calculate_observed_graph(
    _state_graph: &StateGraph,
    observable_graph: &ObservableGraph,
) -> ObservedGraph {
    let mut g = ObservedGraph::new();

    // Create nodes from Destination nodes in the observable graph
    for (idx, node) in observable_graph.node_indices().zip(observable_graph.node_weights()) {
        if node.node_type == ObservableNodeType::Destination {
            g.add_node(ObservedNode {
                name: node.name.clone(),
                observable_node_idx: idx,
            });
        }
    }

    // TODO: Implement edge computation logic
    // Edges will be computed based on state transitions and observable mappings
    // This is left as placeholder for future user implementation

    g
}

// Helper function to calculate observed graph from ObservableGraphDisplay
// Works with the concrete display graph type
pub fn calculate_observed_graph_from_observable_display<Dn, De>(
    observable_display: &egui_graphs::Graph<ObservableNode, f32, petgraph::Directed, petgraph::graph::DefaultIx, Dn, De>,
) -> ObservedGraph
where
    Dn: egui_graphs::DisplayNode<ObservableNode, f32, petgraph::Directed, petgraph::graph::DefaultIx>,
    De: egui_graphs::DisplayEdge<ObservableNode, f32, petgraph::Directed, petgraph::graph::DefaultIx, Dn>,
{
    let mut g = ObservedGraph::new();

    // Create nodes from Destination nodes in the observable graph
    for (idx, node) in observable_display.nodes_iter() {
        let obs_node = node.payload();
        if obs_node.node_type == ObservableNodeType::Destination {
            g.add_node(ObservedNode {
                name: obs_node.name.clone(),
                observable_node_idx: idx,
            });
        }
    }

    // TODO: Implement edge computation logic
    // Edges will be computed based on state transitions and observable mappings
    // This is left as placeholder for future user implementation

    g
}
