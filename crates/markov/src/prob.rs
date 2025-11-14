use ndarray::linalg::Dot;
use num_traits::Float;

use crate::vector::Vector;

/// Probability vector with a bidirectional map for labels X.
#[derive(Debug, Clone)]
pub struct Prob<X, N> {
    pub probs: Vector<X, N>,
}

impl<X, N> Prob<X, N>
where
    X: Ord + Clone,
    N: Float,
{
    pub fn from_vector(
        vector: Vector<X, N>,
    ) -> Result<Self, BuildError>
    where
        N: std::iter::Sum + ndarray::ScalarOperand,
    {
        if vector.is_empty() {
            return Err(BuildError::Empty);
        }

        let has_negative = vector.values().any(|x| *x < N::zero());
        if has_negative {
            return Err(BuildError::NegativeValue);
        }

        let sum = vector.values().copied().sum();
        if sum == N::zero() {
            return Err(BuildError::ZeroSum);
        }

        let mut probs = vector.clone();
        probs.mapv_inplace(|x| x / sum);

        Ok(Self { probs })
    }

    /// Get P[X = x] if `x` is known; otherwise None.
    pub fn prob(&self, x: &X) -> Option<N> {
        self.probs.get(x)
    }

    /// Convert to a Vector.
    pub fn to_vec(&self) -> crate::vector::Vector<X, N> {
        self.probs.clone()
    }

    /// Enumerate all (label, probability) pairs.
    pub fn enumerate(&self) -> impl Iterator<Item = (X, N)> + '_
    where
        N: Copy,
    {
        self.probs.enumerate()
    }

    /// Compute Shannon entropy using natural logarithm.
    pub fn entropy(&self) -> N {
        self.probs
            .values()
            .map(|&p| {
                if p > N::zero() {
                    -(p * p.ln())
                } else {
                    N::zero()
                }
            })
            .fold(N::zero(), |acc, x| acc + x)
    }

    /// Compute the effective number of states.
    pub fn effective_states(&self) -> N {
        self.entropy().exp()
    }
}

// Implement Dot<Prob> for Prob: vector · vector -> scalar
impl<X, N> Dot<Prob<X, N>> for Prob<X, N>
where
    X: Ord + Clone + std::fmt::Debug,
    N: Float + Default + ndarray::ScalarOperand + 'static,
{
    type Output = N;
    fn dot(&self, rhs: &Prob<X, N>) -> N {
        self.probs.dot(&rhs.probs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prob_dot_prob_vector_dot_product() {
        let prob1 = Prob::from_vector(Vector::from_assoc(vec![
            ("alice", 0.6),
            ("bob", 0.3),
            ("chico", 0.1),
        ]))
        .unwrap();

        // Setup second probability vector with DIFFERENT input order: chico, bob, alice
        // After sorting internally, both should have same index order
        let prob2 = Prob::from_vector(Vector::from_assoc(vec![
            ("chico", 0.1),
            ("bob", 0.4),
            ("alice", 0.5),
        ]))
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
}

#[derive(thiserror::Error, Debug)]
pub enum BuildError {
    #[error("vector is empty")]
    Empty,
    #[error("vector has sum zero")]
    ZeroSum,
    #[error("negative value encountered")]
    NegativeValue,
}
