use egui_graphs::{DisplayEdge, DisplayNode, Graph};
use petgraph::visit::{EdgeRef, IntoEdgeReferences};
use petgraph::{EdgeType, stable_graph::IndexType};
use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::graph::{
    ObservableGraph, ObservableNode, ObservableNodeType, StateGraph,
    StateNode,
};

// ------------------------------------------------------------------
// Serialization structures
// ------------------------------------------------------------------

#[derive(Serialize, Deserialize)]
pub struct SerializableNode {
    name: String,
}

#[derive(Serialize, Deserialize)]
pub struct SerializableEdge {
    source: usize,
    target: usize,
    weight: f32,
}

#[derive(Serialize, Deserialize)]
pub struct SerializableGraphState {
    nodes: Vec<SerializableNode>,
    edges: Vec<SerializableEdge>,
}

#[derive(Serialize, Deserialize)]
pub struct SerializableObservableNode {
    name: String,
    node_type: ObservableNodeType,
}

#[derive(Serialize, Deserialize)]
pub struct SerializableObservableState {
    nodes: Vec<SerializableObservableNode>,
    edges: Vec<SerializableEdge>,
}

#[derive(Serialize, Deserialize)]
pub struct SerializableState {
    pub dynamical_system: SerializableGraphState,
    pub observable: SerializableObservableState,
}

// ------------------------------------------------------------------
// Serialization conversion functions
// ------------------------------------------------------------------

pub fn graph_to_serializable<
    Ty: EdgeType,
    Ix: IndexType,
    Dn: DisplayNode<StateNode, f32, Ty, Ix>,
    De: DisplayEdge<StateNode, f32, Ty, Ix, Dn>,
>(
    graph: &Graph<StateNode, f32, Ty, Ix, Dn, De>,
) -> SerializableGraphState {
    let stable_graph = graph.g();
    let mut nodes = Vec::new();
    let mut node_index_map = std::collections::HashMap::new();

    // Collect nodes and build index observable using the nodes_iter from Graph
    for (new_idx, (node_idx, node)) in graph.nodes_iter().enumerate()
    {
        nodes.push(SerializableNode {
            name: node.payload().name.clone(),
        });
        node_index_map.insert(node_idx, new_idx);
    }

    // Collect edges using petgraph's edge_references
    let mut edges = Vec::new();
    for edge_ref in stable_graph.edge_references() {
        edges.push(SerializableEdge {
            source: node_index_map[&edge_ref.source()],
            target: node_index_map[&edge_ref.target()],
            weight: *edge_ref.weight().payload(),
        });
    }

    SerializableGraphState { nodes, edges }
}

pub fn serializable_to_graph(
    state: &SerializableGraphState,
) -> StateGraph {
    let mut g = StateGraph::new();
    let mut node_indices = Vec::new();

    // Add nodes
    for node in &state.nodes {
        let idx = g.add_node(StateNode {
            name: node.name.clone(),
        });
        node_indices.push(idx);
    }

    // Add edges
    for edge in &state.edges {
        g.add_edge(
            node_indices[edge.source],
            node_indices[edge.target],
            edge.weight,
        );
    }

    g
}

pub fn observable_graph_to_serializable<
    Ty: EdgeType,
    Ix: IndexType,
    Dn: DisplayNode<ObservableNode, f32, Ty, Ix>,
    De: DisplayEdge<ObservableNode, f32, Ty, Ix, Dn>,
>(
    graph: &Graph<ObservableNode, f32, Ty, Ix, Dn, De>,
) -> SerializableObservableState {
    let stable_graph = graph.g();
    let mut nodes = Vec::new();
    let mut node_index_map = std::collections::HashMap::new();

    // Collect nodes and build index observable using the nodes_iter from Graph
    for (new_idx, (node_idx, node)) in graph.nodes_iter().enumerate()
    {
        nodes.push(SerializableObservableNode {
            name: node.payload().name.clone(),
            node_type: node.payload().node_type,
        });
        node_index_map.insert(node_idx, new_idx);
    }

    // Collect edges using petgraph's edge_references
    let mut edges = Vec::new();
    for edge_ref in stable_graph.edge_references() {
        edges.push(SerializableEdge {
            source: node_index_map[&edge_ref.source()],
            target: node_index_map[&edge_ref.target()],
            weight: *edge_ref.weight().payload(),
        });
    }

    SerializableObservableState { nodes, edges }
}

pub fn serializable_to_observable_graph(
    state: &SerializableObservableState,
) -> ObservableGraph {
    let mut g = ObservableGraph::new();
    let mut node_indices = Vec::new();

    // Add nodes
    for node in &state.nodes {
        let idx = g.add_node(ObservableNode {
            name: node.name.clone(),
            node_type: node.node_type,
        });
        node_indices.push(idx);
    }

    // Add edges
    for edge in &state.edges {
        g.add_edge(
            node_indices[edge.source],
            node_indices[edge.target],
            edge.weight,
        );
    }

    g
}

// ------------------------------------------------------------------
// File I/O operations
// ------------------------------------------------------------------

pub fn save_to_file(
    state: &SerializableState,
    path: &Path,
) -> Result<(), String> {
    let json = serde_json::to_string_pretty(&state)
        .map_err(|e| format!("Failed to serialize state: {}", e))?;

    std::fs::write(path, json)
        .map_err(|e| format!("Failed to write file: {}", e))?;

    Ok(())
}

pub fn load_from_file(
    path: &Path,
) -> Result<SerializableState, String> {
    let json_str = std::fs::read_to_string(path)
        .map_err(|e| format!("Failed to read file: {}", e))?;

    let state: SerializableState = serde_json::from_str(&json_str)
        .map_err(|e| format!("Failed to parse JSON: {}", e))?;

    Ok(state)
}
