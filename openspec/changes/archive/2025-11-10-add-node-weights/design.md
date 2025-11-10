# Design: add-node-weights

## Architecture Overview
This change extends the state graph model with node weights and adds a computation pipeline for propagating these weights through the observable to produce observed node weights.

```
State Graph           Observable Graph        Observed Graph
+---------+          +---------------+        +---------+
| Node A  |          | Source A      |        | Value 1 |
| w=1.0   |          |   ↓  0.7      |        | w=0.52  |
+---------+    →     | Dest Value 1  |   →    +---------+
| Node B  |          | Source B      |        | Value 2 |
| w=2.0   |          |   ↓  0.8      |        | w=0.48  |
+---------+          | Dest Value 2  |        +---------+
                     +---------------+
```

## Data Flow

### 1. User Input → State Weights
- User edits weights in left panel text fields
- Changes update `StateNode.weight` field immediately
- No validation required (Prob will normalize)

### 2. State Weights → Prob Vector
```rust
// Collect (node_index, weight) pairs from state graph
let weights: Vec<(NodeIndex, f64)> = ...;
let prob = Prob::from_assoc(node_count, weights)?;
// prob.probs is now a normalized probability vector
```

### 3. Observable Edges → Markov Matrix
```rust
// Collect (source_idx, dest_idx, edge_weight) triples
let edges: Vec<(NodeIndex, NodeIndex, f64)> = ...;
let markov = Markov::from_assoc(source_count, dest_count, edges)?;
// markov is row-stochastic (each row sums to 1)
```

### 4. Probability Propagation
```rust
// Vector-matrix multiplication via Dot trait
let observed_prob: Prob<NodeIndex, f64> = prob.dot(&markov);
// Result: observed_prob.probs[j] = Σᵢ prob.probs[i] * markov[i,j]
```

### 5. Observed Weights Assignment
```rust
// Extract weights from observed_prob and assign to observed graph nodes
for (dest_idx, obs_node_idx) in mapping {
    if let Some(weight) = observed_prob.prob(&dest_idx) {
        observed_graph[obs_node_idx].weight = weight;
    }
}
```

## Key Design Decisions

### Decision 1: Weights stored as f32, computation uses f64
**Rationale**: UI works with f32 for consistency with existing edge weights, but computation needs f64 precision for probability math. Convert at computation boundary.

### Decision 2: Computation in self-contained function
**Rationale**: Keep probability math isolated, testable, and well-documented. Function signature:
```rust
fn compute_observed_weights(
    state_graph: &StateGraph,
    observable_graph: &ObservableGraph,
) -> Result<Vec<(NodeIndex, f64)>, ComputationError>
```

### Decision 3: Recompute on every change
**Rationale**: Graphs are small (<20 nodes), computation is fast, consistency is critical. Trigger recomputation whenever:
- State node added/removed/renamed (already triggers recompute_observed_graph)
- State node weight changed (new trigger)
- Observable edge added/removed/modified (already triggers recompute_observed_graph)

### Decision 4: Default weight = 1.0
**Rationale**:
- Simple and predictable
- Uniform distribution emerges naturally when all weights = 1.0
- User can easily adjust after creation
- Avoids special cases for new nodes

### Decision 5: Error handling for invalid configurations
**Scenarios**:
- Empty state graph → Error (no weights to propagate)
- Empty observed graph → Return empty weights (valid but trivial)
- Observable has no edges → All observed weights = 0 (valid: nothing maps)
- Source node in observable not in state graph → Error (data inconsistency)

## Module Organization

### graph_state.rs additions
```rust
pub struct StateNode {
    pub name: String,
    pub weight: f32,  // NEW
}

pub struct ObservedNode {
    pub name: String,
    pub observable_node_idx: NodeIndex,
    pub weight: f32,  // NEW
}

#[derive(thiserror::Error, Debug)]
pub enum WeightComputationError {
    #[error("state graph is empty")]
    EmptyStateGraph,
    #[error("probability construction failed: {0}")]
    ProbError(#[from] markov::prob::BuildError),
    #[error("markov matrix construction failed: {0}")]
    MarkovError(#[from] markov::prob::BuildError),
}

pub fn compute_observed_weights(
    state_graph: &StateGraph,
    observable_graph: &ObservableGraph,
) -> Result<HashMap<NodeIndex, f64>, WeightComputationError>
```

### main.rs additions
```rust
// In left panel node list:
ui.horizontal(|ui| {
    ui.label("Weight:");
    let mut weight_str = node.weight.to_string();
    if ui.text_edit_singleline(&mut weight_str).changed() {
        if let Ok(new_weight) = weight_str.parse::<f32>() {
            node.weight = new_weight.max(0.0);
            self.recompute_observed_graph();
        }
    }
});

// New checkbox:
ui.checkbox(&mut self.show_weights, "Show Weights");
```

## Testing Strategy

### Manual Test Cases
1. **Uniform distribution**: All weights = 1.0 → observed weights reflect edge structure
2. **Single hot node**: One weight = 10.0, others = 0.1 → observed weights dominated by hot node
3. **Complete mapping**: All sources map to one destination → destination weight = 1.0
4. **Sparse mapping**: Only some sources map → observed weights partial sum
5. **Save/Load cycle**: Weights persist correctly

### Build Validation
```bash
cargo build  # Must succeed
cargo clippy # No new warnings
```

## Performance Considerations
- **Target scale**: <20 nodes → computation is O(n²) but negligible
- **Recomputation frequency**: Every weight change, but latency imperceptible
- **Memory**: Additional f32 per node, ~80 bytes total for 20 nodes
- **No optimization needed** at this scale

## Future Extensions (Out of Scope)
- Visualization: Node size proportional to weight
- Analysis: Compute stationary distributions, entropy
- Constraints: Enforce specific weight patterns
- Time evolution: Animate weight propagation
