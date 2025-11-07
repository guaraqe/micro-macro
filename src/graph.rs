use petgraph::stable_graph::StableGraph;
use serde::{Deserialize, Serialize};

// Type aliases for the basic graph types
pub type StateGraph = StableGraph<StateNode, f32>;
pub type ObservableGraph = StableGraph<ObservableNode, f32>;

#[derive(Clone)]
pub struct StateNode {
    pub name: String,
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
