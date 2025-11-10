# Tasks: add-node-weights

This change adds node weights to the state graph and computes observed node weights via probability propagation.

## Task List

- [x] 1. Extend StateNode and ObservedNode models with weight field
- [x] 2. Add weight editor UI in Dynamical System left panel
- [x] 3. Add "Show Weights" toggle and implement weight display in graph
- [x] 4. Implement weight computation function
- [x] 5. Integrate weight computation into observed graph updates
- [x] 6. Display observed weights in left panel
- [x] 7. Update serialization to include weights
- [x] 8. Build and test complete implementation

---

### 1. Extend StateNode and ObservedNode models with weight field
**Description**: Add `weight: f32` field to data structures and update constructors

**Implementation**:
- Modify `StateNode` struct in `crates/micro-macro/src/graph_state.rs:13`
  - Add `pub weight: f32` field
  - Update `default_state_graph()` to initialize weights to 1.0
- Modify `ObservedNode` struct in `crates/micro-macro/src/graph_state.rs:106`
  - Add `pub weight: f32` field
  - Update `calculate_observed_graph_from_observable_display()` to initialize weights to 0.0

**Validation**: `cargo build` succeeds

**Dependencies**: None

---

### 2. Add weight editor UI in Dynamical System left panel
**Description**: Add weight input field to node list entries

**Implementation**:
- Modify `render_dynamical_system_tab()` in `crates/micro-macro/src/main.rs:925`
- In the node list loop (around line 965), after the name editor, add:
  ```rust
  ui.horizontal(|ui| {
      ui.label("Weight:");
      let mut weight_str = format!("{:.2}", node.payload().weight);
      let response = ui.text_edit_singleline(&mut weight_str);
      if response.changed() {
          if let Ok(new_weight) = weight_str.parse::<f32>() {
              if let Some(node_mut) = self.state_graph.node_mut(node_idx) {
                  node_mut.payload_mut().weight = new_weight.max(0.0);
                  self.recompute_observed_graph();
              }
          }
      }
  });
  ```

**Validation**: Weight field appears, accepts input, updates trigger recomputation

**Dependencies**: Task 1

---

### 3. Add "Show Weights" toggle and implement weight display in graph
**Description**: Add checkbox control and conditionally display weights in graph labels

**Implementation**:
- Add field to `GraphEditor` struct in `main.rs:204`:
  ```rust
  show_weights: bool,
  ```
- Initialize in `main()` around line 160: `show_weights: false,`
- Add checkbox in `render_dynamical_system_tab()` bottom controls (around line 1252):
  ```rust
  ui.checkbox(&mut self.show_weights, "Show Weights");
  ```
- Modify node label display logic:
  - When setting node labels, conditionally append weight if `self.show_weights`
  - Update labels dynamically when toggle changes
- Repeat for `render_observed_dynamics_tab()` (around line 1848)

**Validation**: Toggle controls weight visibility in both tabs

**Dependencies**: Task 1, 2

---

### 4. Implement weight computation function
**Description**: Create self-contained function in graph_state.rs implementing probability propagation

**Implementation**:
- Add error type in `crates/micro-macro/src/graph_state.rs`:
  ```rust
  #[derive(thiserror::Error, Debug)]
  pub enum WeightComputationError {
      #[error("state graph is empty")]
      EmptyStateGraph,
      #[error("probability construction failed: {0}")]
      ProbError(#[from] markov::prob::BuildError),
  }
  ```
- Add imports: `use markov::{Prob, Markov}; use ndarray::linalg::Dot;`
- Implement function:
  ```rust
  pub fn compute_observed_weights(
      state_graph: &StateGraph,
      observable_graph: &ObservableGraph,
  ) -> Result<HashMap<NodeIndex, f64>, WeightComputationError> {
      // 1. Validate state graph not empty
      if state_graph.node_count() == 0 {
          return Err(WeightComputationError::EmptyStateGraph);
      }

      // 2. Build Prob from state node weights
      let state_weights: Vec<(NodeIndex, f64)> = state_graph
          .node_indices()
          .map(|idx| (idx, state_graph[idx].weight as f64))
          .collect();
      let prob = Prob::from_assoc(state_graph.node_count(), state_weights)?;

      // 3. Build Markov from observable edges (source -> destination)
      let dest_nodes: Vec<NodeIndex> = observable_graph
          .node_indices()
          .filter(|&idx| observable_graph[idx].node_type == ObservableNodeType::Destination)
          .collect();

      if dest_nodes.is_empty() {
          return Ok(HashMap::new());
      }

      let edges: Vec<(NodeIndex, NodeIndex, f64)> = observable_graph
          .edge_references()
          .map(|e| (e.source(), e.target(), *e.weight() as f64))
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
  ```

**Validation**: Function compiles, can be tested with simple cases

**Dependencies**: Task 1

**Notes**: This is the core mathematical logic; test independently before integration

---

### 5. Integrate weight computation into observed graph updates
**Description**: Call computation function in `recompute_observed_graph()` and apply weights

**Implementation**:
- Modify `recompute_observed_graph()` in `main.rs:442`:
  ```rust
  fn recompute_observed_graph(&mut self) {
      let observed_graph_raw = calculate_observed_graph_from_observable_display(
          &self.observable_graph,
      );
      self.observed_graph = setup_graph_display(&observed_graph_raw);

      // Compute and apply weights
      match compute_observed_weights(&self.state_graph.g(), &self.observable_graph.g()) {
          Ok(weights) => {
              // Map observable destination indices to observed node indices
              for (obs_idx, node) in self.observed_graph.nodes_iter() {
                  let obs_dest_idx = node.payload().observable_node_idx;
                  if let Some(&weight) = weights.get(&obs_dest_idx) {
                      if let Some(node_mut) = self.observed_graph.node_mut(obs_idx) {
                          node_mut.payload_mut().weight = weight as f32;
                      }
                  }
              }
          },
          Err(e) => {
              eprintln!("Weight computation error: {}", e);
              // Set all weights to 0.0 on error
              for (obs_idx, _) in self.observed_graph.nodes_iter() {
                  if let Some(node_mut) = self.observed_graph.node_mut(obs_idx) {
                      node_mut.payload_mut().weight = 0.0;
                  }
              }
          }
      }

      self.observed_layout_reset_needed = true;
  }
  ```

**Validation**: Observed weights update correctly when state/observable changes

**Dependencies**: Task 4

---

### 6. Display observed weights in left panel
**Description**: Show computed weights in Observed Dynamics tab left panel

**Implementation**:
- Modify `render_observed_dynamics_tab()` in `main.rs:1601`
- In the node list (around line 1628), add after the name label:
  ```rust
  ui.horizontal(|ui| {
      ui.label("Weight:");
      ui.label(format!("{:.4}", node.payload().weight));
  });
  ```

**Validation**: Weights display as read-only in observed tab

**Dependencies**: Task 1, 5

---

### 7. Update serialization to include weights
**Description**: Modify SerializableNode to include weight field

**Implementation**:
- Modify `SerializableNode` in `crates/micro-macro/src/serialization.rs:17`:
  ```rust
  pub struct SerializableNode {
      pub name: String,
      #[serde(default = "default_weight")]
      pub weight: f32,
  }

  fn default_weight() -> f32 { 1.0 }
  ```
- Update `graph_to_serializable()` to include weight:
  ```rust
  nodes.push(SerializableNode {
      name: node.name.clone(),
      weight: node.weight,
  });
  ```
- Update `serializable_to_graph()` to restore weight:
  ```rust
  graph.add_node(StateNode {
      name: node.name.clone(),
      weight: node.weight,
  });
  ```

**Validation**: Save graph, modify weights, load graph, verify weights restored

**Dependencies**: Task 1

**Notes**: `#[serde(default)]` ensures backward compatibility with old files

---

### 8. Build and test complete implementation
**Description**: Verify everything works end-to-end

**Implementation**:
- Run `cargo build` and ensure no errors
- Run `cargo clippy` and address any warnings
- Manual testing:
  1. Create state graph with 3 nodes, set weights [1.0, 2.0, 1.0]
  2. Create observable with 2 destinations
  3. Add edges with weights from sources to destinations
  4. Verify observed weights computed correctly
  5. Toggle "Show Weights" and verify display
  6. Save and load, verify weights persist

**Validation**: All requirements met, build succeeds, manual tests pass

**Dependencies**: All previous tasks

---

## Parallel Work Opportunities
- Tasks 2 and 4 can be done in parallel (UI vs computation logic)
- Task 7 (serialization) can be done anytime after Task 1

## Testing Notes
- Focus on manual testing per project conventions
- Test edge cases: empty graphs, zero weights, single node
- Verify mathematical correctness with known distributions
