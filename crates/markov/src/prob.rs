use ndarray::{linalg::Dot, Array1};
use num_traits::Float;

use crate::ix_map::IxMap;

#[derive(thiserror::Error, Debug)]
pub enum BuildError {
    #[error(
        "requested size {0} does not match number of distinct labels {1}"
    )]
    SizeMismatch(usize, usize),
    #[error("non-positive value encountered")]
    NonPositive,
    #[error(
        "row '{0:?}' has zero total weight and cannot be normalized"
    )]
    EmptyRow(String),
    #[error("matrix has zero size")]
    EmptyMatrix,
}

/// Probability vector with a bidirectional map for labels X.
#[derive(Debug, Clone)]
pub struct Prob<X, N> {
    /// Dense probability vector of length n, sums to 1.
    pub probs: Array1<N>,
    /// Labels <-> indices
    pub map: IxMap<X>,
}

impl<X, N> Prob<X, N>
where
    X: Ord + Clone + std::fmt::Debug,
    N: Float + ndarray::ScalarOperand,
{
    pub fn from_assoc(
        size: usize,
        assoc: impl IntoIterator<Item = (X, N)>,
    ) -> Result<Self, BuildError> {
        if size == 0 {
            return Err(BuildError::SizeMismatch(0, 0));
        }

        // Collect and sort pairs by key
        let mut pairs: Vec<(X, N)> = assoc.into_iter().collect();
        pairs.sort_by(|a, b| a.0.cmp(&b.0));

        // Check positivity and aggregate duplicates
        let mut aggregated: Vec<(X, N)> = Vec::new();
        for (x, w) in pairs {
            if !(w > N::zero()) {
                return Err(BuildError::NonPositive);
            }
            if let Some(last) = aggregated.last_mut() {
                if last.0 == x {
                    last.1 = last.1 + w;
                    continue;
                }
            }
            aggregated.push((x, w));
        }

        if aggregated.len() > size {
            return Err(BuildError::SizeMismatch(
                size,
                aggregated.len(),
            ));
        }

        // Extract sorted distinct keys for IxMap
        let keys: Vec<X> =
            aggregated.iter().map(|(x, _)| x.clone()).collect();
        let map = IxMap::from_distinct_sorted(keys);

        // Place values into fixed-size array
        let mut probs = Array1::zeros(size);
        let mut total = N::zero();
        for (x, w) in aggregated {
            let i = map.index_of(&x).expect("built from same keys");
            probs[i] = w;
            total = total + w;
        }
        if !(total > N::zero()) {
            return Err(BuildError::NonPositive);
        }

        // Normalize
        probs.mapv_inplace(|x| x / total);

        Ok(Self { probs, map })
    }

    /// Get P[X = x] if `x` is known; otherwise None.
    pub fn prob(&self, x: &X) -> Option<N> {
        self.map
            .index_of(x)
            .and_then(|i| self.probs.get(i))
            .copied()
    }

    /// Get P[X = x], returning 0 if `x` is unknown.
    pub fn prob0(&self, x: &X) -> N {
        self.prob(x).unwrap_or(N::zero())
    }

    /// Direct by index (useful if you already looked up `x`).
    pub fn prob_at(&self, i: usize) -> Option<N> {
        self.probs.get(i).copied()
    }

    /// Index of label (exposed for callers who want to cache it).
    pub fn index_of(&self, x: &X) -> Option<usize> {
        self.map.index_of(x)
    }
}

// Import Markov for the cross-type dot method
use crate::markov::Markov;

// Implement Dot<Prob> for Prob: vector · vector -> scalar
impl<X, N> Dot<Prob<X, N>> for Prob<X, N>
where
    X: Ord + Clone + std::fmt::Debug,
    N: Float + Default + ndarray::ScalarOperand + 'static,
{
    type Output = N;

    /// Vector-vector dot product: self · other
    /// Uses ndarray's optimized dot product implementation
    /// Note: Assumes label ordering matches (ignoring label matching for now)
    fn dot(&self, rhs: &Prob<X, N>) -> N {
        self.probs.dot(&rhs.probs)
    }
}

// Implement Dot<Markov> for Prob: vector · matrix -> vector
impl<X, B, N> Dot<Markov<X, B, N>> for Prob<X, N>
where
    X: Ord + Clone + std::fmt::Debug,
    B: Ord + Clone + std::fmt::Debug,
    N: Float
        + Default
        + ndarray::ScalarOperand
        + 'static
        + std::ops::AddAssign,
    for<'r> &'r N: std::ops::Mul<&'r N, Output = N>,
{
    type Output = Prob<B, N>;

    /// Vector-matrix dot product: self · matrix (left multiplication)
    /// Treats self as a row vector, returns Prob<B, N> with column labels.
    ///
    /// Computes: result[b] = sum_x self[x] * matrix[x, b]
    /// Uses sprs CSC format to efficiently access columns
    fn dot(&self, matrix: &Markov<X, B, N>) -> Prob<B, N> {
        // For vector · matrix, compute dot product of vector with each column
        // CSC format is perfect for this since columns are contiguous
        let n = matrix.cols.len();
        let mut result_vec = vec![N::zero(); n];

        // For each column j
        for j in 0..n {
            let col = matrix.csc.outer_view(j).unwrap();
            // Dot product of self with column j
            for (row_idx, &val) in
                col.indices().iter().zip(col.data().iter())
            {
                result_vec[j] += self.probs[*row_idx] * val;
            }
        }

        let result_probs = ndarray::Array1::from(result_vec);

        Prob {
            probs: result_probs,
            map: matrix.cols.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prob_dot_prob_vector_dot_product() {
        // Setup first probability vector with one order: alice=0.6, bob=0.3, chico=0.1
        let prob1 = Prob::from_assoc(
            3,
            vec![("alice", 0.6), ("bob", 0.3), ("chico", 0.1)],
        )
        .unwrap();

        // Setup second probability vector with DIFFERENT input order: chico, bob, alice
        // After sorting internally, both should have same index order
        let prob2 = Prob::from_assoc(
            3,
            vec![("chico", 0.1), ("bob", 0.4), ("alice", 0.5)],
        )
        .unwrap();

        // Test: prob1 · prob2 (vector-vector dot product)
        // Expected: 0.6×0.5 + 0.3×0.4 + 0.1×0.1 = 0.3 + 0.12 + 0.01 = 0.43
        // This should work because both vectors are sorted internally by label
        let result = prob1.dot(&prob2);

        assert!(
            (result - 0.43).abs() < 1e-10,
            "Dot product should be 0.43, got {}",
            result
        );

        println!("✓ Vector-vector dot product test passed!");
        println!("  prob1 · prob2 = {} (order-independent)", result);
    }

    #[test]
    fn test_prob_dot_markov_alice_bob_chico() {
        // Setup probability vector with one order: chico, alice, bob
        // Will be sorted internally to: alice, bob, chico
        let prob = Prob::from_assoc(
            3,
            vec![("chico", 0.2), ("alice", 0.5), ("bob", 0.3)],
        )
        .unwrap();

        // Verify probabilities sum to 1.0
        let sum: f64 = prob.probs.iter().sum();
        assert!(
            (sum - 1.0).abs() < 1e-10,
            "Input probabilities should sum to 1.0"
        );

        // Setup Markov matrix (3×2) with DIFFERENT input order:
        //          1     2
        // alice:  0.7   0.3
        // bob:    0.4   0.6
        // chico:  0.2   0.8
        // Input order: bob, chico, alice (scrambled)
        // Will be sorted internally to match prob vector
        let markov = Markov::from_assoc(
            3,
            2,
            vec![
                ("bob", 2, 0.6),
                ("chico", 1, 0.2),
                ("alice", 1, 0.7),
                ("bob", 1, 0.4),
                ("chico", 2, 0.8),
                ("alice", 2, 0.3),
            ],
        )
        .unwrap();

        // Verify the matrix is row-stochastic (each row sums to 1)
        for name in ["alice", "bob", "chico"] {
            let row_sum = markov.p0(&name, &1) + markov.p0(&name, &2);
            assert!(
                (row_sum - 1.0).abs() < 1e-10,
                "Row {} should sum to 1.0",
                name
            );
        }

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

        // Verify result probabilities sum to 1.0
        let result_sum: f64 = result.probs.iter().sum();
        assert!(
            (result_sum - 1.0).abs() < 1e-10,
            "Result probabilities should sum to 1.0, got {}",
            result_sum
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

    #[test]
    fn test_markov_dot_prob_right_multiplication() {
        // Setup Markov matrix (3×2) with DIFFERENT input order:
        //          1     2
        // alice:  0.7   0.3
        // bob:    0.4   0.6
        // chico:  0.2   0.8
        // Input in reverse order to test sorting
        let markov = Markov::from_assoc(
            3,
            2,
            vec![
                ("chico", 2, 0.8),
                ("chico", 1, 0.2),
                ("bob", 2, 0.6),
                ("bob", 1, 0.4),
                ("alice", 2, 0.3),
                ("alice", 1, 0.7),
            ],
        )
        .unwrap();

        // Setup probability vector over outcomes with DIFFERENT order: 2, 1
        // Will be sorted internally to: 1, 2
        let prob =
            Prob::from_assoc(2, vec![(2, 0.4), (1, 0.6)]).unwrap();

        // Test: markov · prob (right multiplication, matrix × column vector)
        let result = markov.dot(&prob);

        // Expected calculation:
        // result[alice] = 0.7×0.6 + 0.3×0.4 = 0.42 + 0.12 = 0.54
        // result[bob]   = 0.4×0.6 + 0.6×0.4 = 0.24 + 0.24 = 0.48
        // result[chico] = 0.2×0.6 + 0.8×0.4 = 0.12 + 0.32 = 0.44

        let p_alice =
            result.prob(&"alice").expect("Result should have alice");
        let p_bob =
            result.prob(&"bob").expect("Result should have bob");
        let p_chico =
            result.prob(&"chico").expect("Result should have chico");

        assert!(
            (p_alice - 0.54).abs() < 1e-10,
            "P(alice) should be 0.54, got {}",
            p_alice
        );
        assert!(
            (p_bob - 0.48).abs() < 1e-10,
            "P(bob) should be 0.48, got {}",
            p_bob
        );
        assert!(
            (p_chico - 0.44).abs() < 1e-10,
            "P(chico) should be 0.44, got {}",
            p_chico
        );

        // Note: Result may not sum to 1.0 because this is matrix × vector,
        // not a probability propagation
        let result_sum: f64 = result.probs.iter().sum();
        println!(
            "✓ Right multiplication (markov · prob) test passed!"
        );
        println!(
            "  Result: alice={}, bob={}, chico={} (sum={})",
            p_alice, p_bob, p_chico, result_sum
        );
    }
}
