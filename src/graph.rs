use serde::{Deserialize, Serialize};

// ------------------------------------------------------------------
// Node types - Core graph data structures
// ------------------------------------------------------------------

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
