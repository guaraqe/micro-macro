# Proposal: add-node-weights

## Why
To allow the study of probability distributions over state spaces and their propagation through observables.

- Add `weight: f32` field to `StateNode` (default 1.0)
- Add weight editor UI in left panel of Dynamical System tab
- Add "Show Weights" toggle for optional graph visualization display
- Add `weight: f32` field to `ObservedNode` (computed, read-only)
- Implement `compute_observed_weights()` function: state weights → Prob → dot(Markov) → observed weights
- Automatic recomputation when state/observable graphs change
- Serialize/deserialize node weights

## Impact
- **Affected specs**: state-node-weights (new), observed-weight-computation (new)
- **Affected code**:
  - `crates/micro-macro/src/graph_state.rs` - Add weight fields and computation function
  - `crates/micro-macro/src/main.rs` - Add weight UI (lines 930+, 1600+)
  - `crates/micro-macro/src/serialization.rs` - Add weight serialization

## Scope Details
### In Scope
- Add `weight` field to `StateNode` with default value 1.0
- UI for editing node weights in the left panel of the Dynamical System tab
- Optional display of weights in graph visualization (toggle control)
- Self-contained computation function implementing: state weights → Prob → dot(Markov) → observed weights
- Automatic recomputation when state graph or observable graph changes
- Serialization support for node weights

### Out of Scope
- Weight normalization UI (Prob handles this internally)
- Weight constraints or validation (any positive value allowed)
- Historical tracking of weight changes
- Advanced probability analysis features

## Alternatives Considered
1. **Manual weight entry for observed nodes**: Rejected because weights should be derived mathematically from state weights
2. **Display weights as edge properties**: Rejected because weights are inherent to nodes in probability distributions
3. **Separate "distribution" object**: Rejected for simplicity; weights live directly on nodes

## Dependencies
- Depends on existing `markov` crate with `Prob` and `Markov` types
- Depends on existing observable-definition spec for bipartite mapping graph

## Rollout Plan
1. Implement state node weights with UI
2. Implement computation function (can be tested independently)
3. Integrate computation into observed graph updates
4. Update serialization
5. Manual testing with cargo build

## Success Metrics
- Build passes without errors
- Weights persist across save/load cycles
- Computed observed weights correctly reflect probability propagation
- UI allows easy weight editing
