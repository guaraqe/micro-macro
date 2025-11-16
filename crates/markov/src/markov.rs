use ndarray::linalg::Dot;

use crate::matrix::Matrix;
use crate::prob::Prob;
use crate::vector::max_difference;

/// Row-stochastic Markov kernel
#[derive(Debug, Clone)]
pub struct Markov<X, Y> {
    pub matrix: Matrix<X, Y>,
}

impl<X, Y> Markov<X, Y>
where
    X: Ord + Clone,
    Y: Ord + Clone,
{
    pub fn from_matrix(
        matrix: Matrix<X, Y>,
    ) -> Result<Self, BuildError> {
        let nrows = matrix.x_ix_map.len();
        let ncols = matrix.y_ix_map.len();

        if nrows == 0 || ncols == 0 {
            return Err(BuildError::EmptyMatrix);
        }

        if matrix.values.data().iter().any(|s| *s < 0.0) {
            return Err(BuildError::NegativeValue);
        }

        let row_sums = matrix.get_rows_sums();

        if row_sums.values.iter().any(|s| *s <= 0.0) {
            return Err(BuildError::EmptyRow);
        }

        let result = matrix.map_rows(&row_sums, |v, s| v / s);

        Ok(Self { matrix: result })
    }

    /// To matrix
    pub fn to_matrix(&self) -> &Matrix<X, Y> {
        &self.matrix
    }

    /// Enumerate all (row_label, col_label, value) triplets.
    pub fn enumerate(&self) -> impl Iterator<Item = (X, Y, f64)> + '_ {
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
impl<X> Markov<X, X>
where
    X: Ord + Clone,
{
    /// Compute equilibrium distribution using power iteration.
    pub fn compute_equilibrium(
        &self,
        initial: &Prob<X>,
        tolerance: f64,
        max_iterations: usize,
    ) -> Prob<X> {
        let mut current = initial.clone();

        for _ in 0..max_iterations {
            let next = current.dot(self);

            let diff = max_difference(&current.vector, &next.vector);
            if diff < tolerance {
                return next;
            }

            current = next;
        }

        current
    }

    /// Compute the entropy rate of the Markov chain.
    pub fn entropy_rate(&self, stationary: &Prob<X>) -> f64 {
        let csr = self.matrix.values.to_csr();
        let mut total = 0.0;

        for i in 0..self.matrix.x_ix_map.len() {
            let pi = stationary.vector.values[i];
            if pi <= 0.0 {
                continue;
            }

            if let Some(row) = csr.outer_view(i) {
                for &val in row.data().iter() {
                    if val > 0.0 {
                        total += pi * val * val.ln();
                    }
                }
            }
        }

        -total
    }

    /// Compute the detailed balance deviation.
    /// Φ = (1/2) Σ_ij |π_i P_ij - π_j P_ji|
    pub fn detailed_balance_deviation(
        &self,
        stationary: &Prob<X>,
    ) -> Matrix<X, X> {
        let transition =
            self.matrix.map_rows(&stationary.vector, |v, p| v * p);

        let transpose = transition.transpose();

        transition.binop(&transpose, |x, y| x - y)
    }

    pub fn detailed_balance_deviation_sum(
        &self,
        stationary: &Prob<X>,
    ) -> f64 {
        let matrix = self.detailed_balance_deviation(stationary);
        matrix
            .values
            .iter()
            .map(|(v, _)| v.abs() / 2.0)
            .sum()
    }
}

// Implement Dot<Markov> for Prob: vector · matrix -> vector
impl<X, Y> Dot<Markov<X, Y>> for Prob<X>
where
    X: Ord,
    Y: Ord + Clone,
{
    type Output = Prob<Y>;
    fn dot(&self, markov: &Markov<X, Y>) -> Prob<Y> {
        Prob::from_vector(self.vector.dot(&markov.matrix)).unwrap()
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
    use crate::Vector;

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
        let markov = Markov::from_matrix(Matrix::from_assoc(vec![
            ("bob", 2, 0.6),
            ("chico", 1, 0.2),
            ("alice", 1, 0.7),
            ("bob", 1, 0.4),
            ("chico", 2, 0.8),
            ("alice", 2, 0.3),
        ]))
        .unwrap();

        let result = prob.dot(&markov);

        // Expected calculation:
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
