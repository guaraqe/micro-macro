use egui_graphs::{DisplayEdge, DisplayNode, Graph};
use petgraph::visit::{EdgeRef, IntoEdgeReferences};
use petgraph::{EdgeType, stable_graph::IndexType};
use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::graph_state::{
    ObservableGraph, ObservableNode, ObservableNodeType, StateGraph,
    StateNode,
};
use crate::layout_settings::LayoutSettings;

// ------------------------------------------------------------------
// Serialization structures
// ------------------------------------------------------------------

fn default_weight() -> f32 {
    1.0
}

#[derive(Clone, Serialize, Deserialize)]
pub struct SerializableNode {
    name: String,
    #[serde(default = "default_weight")]
    weight: f32,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct SerializableEdge {
    source: usize,
    target: usize,
    weight: f32,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct SerializableGraphState {
    nodes: Vec<SerializableNode>,
    edges: Vec<SerializableEdge>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct SerializableDestinationNode {
    name: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct SerializableObservableState {
    /// Only Destination nodes are serialized; Source nodes are derived from StateGraph
    destination_nodes: Vec<SerializableDestinationNode>,
    /// Edges reference: source is index in StateGraph, target is index in destination_nodes
    edges: Vec<SerializableEdge>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct SerializableState {
    pub dynamical_system: SerializableGraphState,
    pub observable: SerializableObservableState,
    #[serde(default)]
    pub layout_settings: LayoutSettings,
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
            weight: node.payload().weight,
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
            weight: node.weight,
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
    let mut destination_nodes = Vec::new();
    let mut source_to_state_idx = std::collections::HashMap::new();
    let mut dest_to_serial_idx = std::collections::HashMap::new();

    // Collect nodes: only serialize Destination nodes, map Source nodes to their StateGraph indices
    for (node_idx, node) in graph.nodes_iter() {
        let obs_node = node.payload();
        match obs_node.node_type {
            ObservableNodeType::Source => {
                // Source nodes reference StateGraph - store that reference
                if let Some(state_idx) = obs_node.state_node_idx {
                    source_to_state_idx
                        .insert(node_idx, state_idx.index());
                }
            }
            ObservableNodeType::Destination => {
                // Only Destination nodes are serialized
                let serial_idx = destination_nodes.len();
                destination_nodes.push(SerializableDestinationNode {
                    name: obs_node.name.clone(),
                });
                dest_to_serial_idx.insert(node_idx, serial_idx);
            }
        }
    }

    // Collect edges: source is StateGraph index, target is destination_nodes index
    let mut edges = Vec::new();
    for edge_ref in stable_graph.edge_references() {
        let source_idx = edge_ref.source();
        let target_idx = edge_ref.target();

        // Source should be a Source node (maps to StateGraph index)
        if let Some(&state_idx) = source_to_state_idx.get(&source_idx)
        {
            // Target should be a Destination node (maps to destination_nodes index)
            if let Some(&dest_idx) =
                dest_to_serial_idx.get(&target_idx)
            {
                edges.push(SerializableEdge {
                    source: state_idx,
                    target: dest_idx,
                    weight: *edge_ref.weight().payload(),
                });
            }
        }
    }

    SerializableObservableState {
        destination_nodes,
        edges,
    }
}

pub fn serializable_to_observable_graph(
    state: &SerializableObservableState,
    state_graph: &StateGraph,
) -> ObservableGraph {
    let mut g = ObservableGraph::new();

    // First, add Source nodes (derived from StateGraph)
    let mut source_node_indices = Vec::new();
    for (state_idx, state_node) in
        state_graph.node_indices().zip(state_graph.node_weights())
    {
        let obs_idx = g.add_node(ObservableNode {
            name: state_node.name.clone(),
            node_type: ObservableNodeType::Source,
            state_node_idx: Some(state_idx),
        });
        source_node_indices.push(obs_idx);
    }

    // Second, add Destination nodes (from serialized data)
    let mut dest_node_indices = Vec::new();
    for dest_node in &state.destination_nodes {
        let obs_idx = g.add_node(ObservableNode {
            name: dest_node.name.clone(),
            node_type: ObservableNodeType::Destination,
            state_node_idx: None,
        });
        dest_node_indices.push(obs_idx);
    }

    // Add edges: source is StateGraph index, target is destination_nodes index
    for edge in &state.edges {
        g.add_edge(
            source_node_indices[edge.source],
            dest_node_indices[edge.target],
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

// ------------------------------------------------------------------
// Tests
// ------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph_view::{
        setup_observable_graph_display, setup_state_graph_display,
    };

    /// Helper to create a test observable graph with edges from each Source node
    fn create_test_graphs() -> (StateGraph, ObservableGraph) {
        // Create state graph with nodes in alphabetical order
        let mut state_graph = StateGraph::new();
        let a = state_graph.add_node(crate::graph_state::StateNode {
            name: "A".to_string(),
            weight: 1.0,
        });
        let b = state_graph.add_node(crate::graph_state::StateNode {
            name: "B".to_string(),
            weight: 1.0,
        });
        let c = state_graph.add_node(crate::graph_state::StateNode {
            name: "C".to_string(),
            weight: 1.0,
        });

        // Add edges in state graph
        state_graph.add_edge(a, b, 1.0);
        state_graph.add_edge(b, c, 2.0);
        state_graph.add_edge(c, a, 3.0);

        // Create observable graph with Source nodes matching state graph
        let mut obs_graph = ObservableGraph::new();

        // Add Source nodes (matching state graph, in same order)
        let mut source_indices = Vec::new();
        for (state_idx, state_node) in
            state_graph.node_indices().zip(state_graph.node_weights())
        {
            let obs_idx = obs_graph.add_node(ObservableNode {
                name: state_node.name.clone(),
                node_type: ObservableNodeType::Source,
                state_node_idx: Some(state_idx),
            });
            source_indices.push(obs_idx);
        }

        // Add Destination nodes in alphabetical order
        let dest1 = obs_graph.add_node(ObservableNode {
            name: "Value X".to_string(),
            node_type: ObservableNodeType::Destination,
            state_node_idx: None,
        });
        let dest2 = obs_graph.add_node(ObservableNode {
            name: "Value Y".to_string(),
            node_type: ObservableNodeType::Destination,
            state_node_idx: None,
        });

        // Add edges: each Source node has at least one edge to a Destination
        obs_graph.add_edge(source_indices[0], dest1, 1.5); // A -> Value X
        obs_graph.add_edge(source_indices[1], dest2, 2.5); // B -> Value Y
        obs_graph.add_edge(source_indices[2], dest1, 3.5); // C -> Value X

        (state_graph, obs_graph)
    }

    /// Helper to compare two SerializableState objects with tolerance for float comparison
    fn assert_serializable_states_equal(
        state1: &SerializableState,
        state2: &SerializableState,
    ) {
        // Compare dynamical system nodes
        assert_eq!(
            state1.dynamical_system.nodes.len(),
            state2.dynamical_system.nodes.len(),
            "StateGraph node count mismatch"
        );
        for (n1, n2) in state1
            .dynamical_system
            .nodes
            .iter()
            .zip(&state2.dynamical_system.nodes)
        {
            assert_eq!(
                n1.name, n2.name,
                "StateGraph node name mismatch"
            );
        }

        // Compare dynamical system edges
        assert_eq!(
            state1.dynamical_system.edges.len(),
            state2.dynamical_system.edges.len(),
            "StateGraph edge count mismatch"
        );
        for (e1, e2) in state1
            .dynamical_system
            .edges
            .iter()
            .zip(&state2.dynamical_system.edges)
        {
            assert_eq!(
                e1.source, e2.source,
                "StateGraph edge source mismatch"
            );
            assert_eq!(
                e1.target, e2.target,
                "StateGraph edge target mismatch"
            );
            assert!(
                (e1.weight - e2.weight).abs() < 0.001,
                "StateGraph edge weight mismatch"
            );
        }

        // Compare observable destination nodes
        assert_eq!(
            state1.observable.destination_nodes.len(),
            state2.observable.destination_nodes.len(),
            "ObservableGraph destination node count mismatch"
        );
        for (n1, n2) in state1
            .observable
            .destination_nodes
            .iter()
            .zip(&state2.observable.destination_nodes)
        {
            assert_eq!(
                n1.name, n2.name,
                "ObservableGraph destination node name mismatch"
            );
        }

        // Compare observable edges
        assert_eq!(
            state1.observable.edges.len(),
            state2.observable.edges.len(),
            "ObservableGraph edge count mismatch"
        );
        for (e1, e2) in state1
            .observable
            .edges
            .iter()
            .zip(&state2.observable.edges)
        {
            assert_eq!(
                e1.source, e2.source,
                "ObservableGraph edge source mismatch"
            );
            assert_eq!(
                e1.target, e2.target,
                "ObservableGraph edge target mismatch"
            );
            assert!(
                (e1.weight - e2.weight).abs() < 0.001,
                "ObservableGraph edge weight mismatch"
            );
        }
    }

    #[test]
    fn test_serialization_round_trip() {
        // Create test graphs with edges
        let (state_graph, obs_graph) = create_test_graphs();

        // Verify the observable graph has edges from each Source node
        let source_nodes: Vec<_> = obs_graph
            .node_indices()
            .filter(|&idx| {
                obs_graph.node_weight(idx).unwrap().node_type
                    == ObservableNodeType::Source
            })
            .collect();

        for &source_idx in &source_nodes {
            let outgoing_edges: Vec<_> =
                obs_graph.edges(source_idx).collect();
            assert!(
                !outgoing_edges.is_empty(),
                "Source node {:?} has no outgoing edges",
                obs_graph.node_weight(source_idx).unwrap().name
            );
        }

        // Convert to serializable format using setup_graph_display
        let state_display = setup_state_graph_display(&state_graph);
        let obs_display = setup_observable_graph_display(&obs_graph);

        let state_serializable =
            graph_to_serializable(&state_display);
        let obs_serializable =
            observable_graph_to_serializable(&obs_display);
        let original_state = SerializableState {
            dynamical_system: state_serializable,
            observable: obs_serializable,
            layout_settings: LayoutSettings::default(),
        };

        // Save to file
        let temp_file =
            std::env::temp_dir().join("rust_dyn_syst_test.json");
        save_to_file(&original_state, &temp_file)
            .expect("Failed to save file");

        // Load from file
        let loaded_state =
            load_from_file(&temp_file).expect("Failed to load file");

        // Compare: original serializable vs loaded serializable
        assert_serializable_states_equal(
            &original_state,
            &loaded_state,
        );

        // Convert back to graphs
        let loaded_state_graph =
            serializable_to_graph(&loaded_state.dynamical_system);
        let loaded_obs_graph = serializable_to_observable_graph(
            &loaded_state.observable,
            &loaded_state_graph,
        );

        // Convert back to serializable again using setup_graph_display
        let loaded_state_display =
            setup_state_graph_display(&loaded_state_graph);
        let loaded_obs_display =
            setup_observable_graph_display(&loaded_obs_graph);

        let state_serializable2 =
            graph_to_serializable(&loaded_state_display);
        let obs_serializable2 =
            observable_graph_to_serializable(&loaded_obs_display);
        let reloaded_state = SerializableState {
            dynamical_system: state_serializable2,
            observable: obs_serializable2,
            layout_settings: LayoutSettings::default(),
        };

        // Compare: loaded serializable vs re-serialized
        assert_serializable_states_equal(
            &loaded_state,
            &reloaded_state,
        );

        // Cleanup
        std::fs::remove_file(&temp_file).ok();
    }
}
