use ndarray::linalg::Dot;
use num_traits::Float;
use sprs::{prod, CsMat};

use crate::matrix::Matrix;
use crate::prob::Prob;
use crate::vector::{max_difference, Vector};

/// Row-stochastic Markov kernel
#[derive(Debug, Clone)]
pub struct Markov<X, Y, N> {
    pub matrix: Matrix<X, Y, N>,
}

impl<X, Y, N> Markov<X, Y, N>
where
    X: Ord + Clone,
    Y: Ord + Clone,
    N: Float + Default,
{
    pub fn from_matrix(
        matrix: Matrix<X, Y, N>,
    ) -> Result<Self, BuildError>
    where
        N: std::ops::AddAssign,
    {
        let nrows = matrix.x_ix_map.len();
        let ncols = matrix.y_ix_map.len();

        if nrows == 0 || ncols == 0 {
            return Err(BuildError::EmptyMatrix);
        }

        let mut mat: CsMat<N> = matrix.values.clone();

        if mat.data().iter().any(|s| *s < N::zero()) {
            return Err(BuildError::NegativeValue);
        }

        let mut row_sums = vec![N::zero(); nrows];

        for col in mat.outer_iterator() {
            for (&row, &val) in
                col.indices().iter().zip(col.data().iter())
            {
                row_sums[row] += val;
            }
        }

        if row_sums.iter().any(|s| *s <= N::zero()) {
            return Err(BuildError::EmptyRow);
        }

        for mut col in mat.outer_iterator_mut() {
            for (row, val) in col.iter_mut() {
                *val = *val / row_sums[row];
            }
        }

        Ok(Self {
            matrix: Matrix {
                values: mat,
                x_ix_map: matrix.x_ix_map,
                y_ix_map: matrix.y_ix_map,
            },
        })
    }

    /// Enumerate all (row_label, col_label, value) triplets.
    pub fn enumerate(&self) -> impl Iterator<Item = (X, Y, N)> + '_
    where
        N: Copy,
    {
        self.matrix.values.iter().filter_map(
            move |(val, (row_idx, col_idx))| {
                let row_label =
                    self.matrix.x_ix_map.value_of(row_idx)?;
                let col_label =
                    self.matrix.y_ix_map.value_of(col_idx)?;
                Some((row_label.clone(), col_label.clone(), *val))
            },
        )
    }
}

// Implement equilibrium computation for square matrices (A = B)
impl<X, N> Markov<X, X, N>
where
    X: Ord + Clone,
    N: Float
        + Default
        + ndarray::ScalarOperand
        + 'static
        + std::ops::AddAssign,
    for<'r> &'r N: std::ops::Mul<&'r N, Output = N>,
{
    /// Compute equilibrium distribution using power iteration.
    pub fn compute_equilibrium(
        &self,
        initial: &Prob<X, N>,
        tolerance: N,
        max_iterations: usize,
    ) -> Prob<X, N>
    where
        N: Ord + std::iter::Sum,
    {
        let mut current = initial.clone();

        for _ in 0..max_iterations {
            let next = current.dot(self);

            let diff = max_difference(&current.probs, &next.probs);
            if diff < tolerance {
                return next;
            }

            current = next;
        }

        current
    }

    /// Compute the entropy rate of the Markov chain.
    pub fn entropy_rate(&self, stationary: &Prob<X, N>) -> N {
        let csr = self.matrix.values.to_csr();
        let mut total = N::zero();

        for i in 0..self.matrix.x_ix_map.len() {
            let pi = stationary.probs.values[i];
            if pi <= N::zero() {
                continue;
            }

            if let Some(row) = csr.outer_view(i) {
                for &val in row.data().iter() {
                    if val > N::zero() {
                        total += pi * val * val.ln();
                    }
                }
            }
        }

        -total
    }

    /// Compute the detailed balance deviation.
    /// Φ = (1/2) Σ_ij |π_i P_ij - π_j P_ji|
    ///
    /// Measures the degree of irreversibility in the Markov chain.
    /// Returns 0 for reversible chains, larger values for chains with
    /// stronger cyclic probability currents.
    ///
    /// # Arguments
    /// * `stationary` - The stationary distribution π
    ///
    /// # Returns
    /// The total probability circulation measure
    pub fn detailed_balance_deviation(
        &self,
        stationary: &Prob<X, N>,
    ) -> N {
        let csr = self.matrix.values.to_csr();
        let mut total = N::zero();
        let two = N::one() + N::one();

        // Iterate over all pairs (i, j)
        for i in 0..self.matrix.x_ix_map.len() {
            let pi = stationary.probs[i];
            if let Some(row_i) = csr.outer_view(i) {
                for (&j, &p_ij) in
                    row_i.indices().iter().zip(row_i.data().iter())
                {
                    let pj = stationary.probs[j];

                    // Get P_ji (transition from j to i)
                    let p_ji = if let Some(row_j) = csr.outer_view(j)
                    {
                        row_j.get(i).copied().unwrap_or(N::zero())
                    } else {
                        N::zero()
                    };

                    // Add |π_i P_ij - π_j P_ji|
                    let diff = (pi * p_ij - pj * p_ji).abs();
                    total += diff;
                }
            }
        }

        // Divide by 2 since we count each pair twice
        total / two
    }
}

// Implement Dot<Markov> for Prob: vector · matrix -> vector
impl<X, Y, N> Dot<Markov<X, Y, N>> for Prob<X, N>
where
    X: Ord,
    Y: Ord + Clone,
    N: Float
        + std::ops::AddAssign
        + std::iter::Sum
        + ndarray::ScalarOperand,
    for<'r> &'r N: std::ops::Mul<&'r N, Output = N>,
{
    type Output = Prob<Y, N>;
    fn dot(&self, markov: &Markov<X, Y, N>) -> Prob<Y, N> {
        Prob::from_vector(self.probs.dot(&markov.matrix)).unwrap()
    }
}

#[derive(thiserror::Error, Debug)]
pub enum BuildError {
    #[error("negative value encountered")]
    NegativeValue,
    #[error("a row has zero total weight")]
    EmptyRow,
    #[error("matrix has zero size")]
    EmptyMatrix,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prob_dot_markov_alice_bob_chico() {
        // Setup probability vector with one order: chico, alice, bob
        // Will be sorted internally to: alice, bob, chico
        let prob = Prob::from_vector(Vector::from_assoc(vec![
            ("chico", 0.2),
            ("alice", 0.5),
            ("bob", 0.3),
        ]))
        .unwrap();

        // Setup Markov matrix (3×2) with DIFFERENT input order:
        //          1     2
        // alice:  0.7   0.3
        // bob:    0.4   0.6
        // chico:  0.2   0.8
        // Input order: bob, chico, alice (scrambled)
        // Will be sorted internally to match prob vector
        let markov = Markov::from_matrix(Matrix::from_assoc(vec![
            ("bob", 2, 0.6),
            ("chico", 1, 0.2),
            ("alice", 1, 0.7),
            ("bob", 1, 0.4),
            ("chico", 2, 0.8),
            ("alice", 2, 0.3),
        ]))
        .unwrap();

        // Test: prob · markov (left multiplication, row vector × matrix)
        let result = prob.dot(&markov);

        // Expected calculation:
        // result[1] = 0.5×0.7 + 0.3×0.4 + 0.2×0.2
        //           = 0.35 + 0.12 + 0.04
        //           = 0.51
        // result[2] = 0.5×0.3 + 0.3×0.6 + 0.2×0.8
        //           = 0.15 + 0.18 + 0.16
        //           = 0.49

        let p1 =
            result.prob(&1).expect("Result should have outcome 1");
        let p2 =
            result.prob(&2).expect("Result should have outcome 2");

        assert!(
            (p1 - 0.51).abs() < 1e-10,
            "P(1) should be 0.51, got {}",
            p1
        );
        assert!(
            (p2 - 0.49).abs() < 1e-10,
            "P(2) should be 0.49, got {}",
            p2
        );

        println!(
            "✓ Left multiplication (prob · markov) test passed!"
        );
        println!(
            "  Input: alice={}, bob={}, chico={}",
            prob.prob(&"alice").unwrap(),
            prob.prob(&"bob").unwrap(),
            prob.prob(&"chico").unwrap()
        );
        println!("  Result: 1={}, 2={}", p1, p2);
    }
}
