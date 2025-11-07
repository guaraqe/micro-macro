use petgraph::stable_graph::StableGraph;
use serde::{Deserialize, Serialize};

// Type aliases for the basic graph types
pub type StateGraph = StableGraph<StateNode, f32>;
pub type ObservableGraph = StableGraph<ObservableNode, f32>;

pub trait HasName {
    fn name(&self) -> String;
}

#[derive(Clone)]
pub struct StateNode {
    pub name: String,
}

impl HasName for StateNode {
    fn name(&self) -> String {
        self.name.clone()
    }
}

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ObservableNodeType {
    Source,
    Destination,
}

#[derive(Clone)]
pub struct ObservableNode {
    pub name: String,
    pub node_type: ObservableNodeType,
}

impl HasName for ObservableNode {
    fn name(&self) -> String {
        self.name.clone()
    }
}

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

pub fn default_observable_graph(
    source_graph: &StateGraph,
) -> ObservableGraph {
    let mut g = ObservableGraph::new();

    // Add Source nodes mirroring the dynamical system
    for node in source_graph.node_weights() {
        g.add_node(ObservableNode {
            name: node.name.clone(),
            node_type: ObservableNodeType::Source,
        });
    }

    // Add two default Destination nodes
    g.add_node(ObservableNode {
        name: String::from("Value 0"),
        node_type: ObservableNodeType::Destination,
    });
    g.add_node(ObservableNode {
        name: String::from("Value 1"),
        node_type: ObservableNodeType::Destination,
    });

    g
}
