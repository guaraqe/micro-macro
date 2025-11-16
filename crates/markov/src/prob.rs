use ndarray::linalg::Dot;

use crate::vector::Vector;

/// Probability vector with a bidirectional map for labels X.
#[derive(Debug, Clone)]
pub struct Prob<X> {
    pub vector: Vector<X>,
}

impl<X> Prob<X>
where
    X: Ord + Clone,
{
    pub fn from_vector(vector: Vector<X>) -> Result<Self, BuildError> {
        if vector.is_empty() {
            return Err(BuildError::Empty);
        }

        let has_negative = vector.values().any(|x| *x < 0.0);
        if has_negative {
            return Err(BuildError::NegativeValue);
        }

        let sum: f64 = vector.values().copied().sum();
        if sum == 0.0 {
            return Err(BuildError::ZeroSum);
        }

        let mut vector = vector.clone();
        vector.mapv_inplace(|x| x / sum);

        Ok(Self { vector })
    }

    /// To vector
    pub fn to_vector(&self) -> &Vector<X> {
        &self.vector
    }

    /// Get P[X = x] if `x` is known; otherwise None.
    pub fn prob(&self, x: &X) -> Option<f64> {
        self.vector.get(x)
    }

    /// Convert to a Vector.
    pub fn to_vec(&self) -> crate::vector::Vector<X> {
        self.vector.clone()
    }

    /// Enumerate all (label, probability) pairs.
    pub fn enumerate(&self) -> impl Iterator<Item = (X, f64)> + '_ {
        self.vector.enumerate()
    }

    /// Compute Shannon entropy using natural logarithm.
    pub fn entropy(&self) -> f64 {
        self.vector
            .values()
            .map(|&p| if p > 0.0 { -(p * p.ln()) } else { 0.0 })
            .fold(0.0, |acc, x| acc + x)
    }

    /// Compute the effective number of states.
    pub fn effective_states(&self) -> f64 {
        self.entropy().exp()
    }
}

// Implement Dot<Prob> for Prob: vector · vector -> scalar
impl<X> Dot<Prob<X>> for Prob<X>
where
    X: Ord + Clone + std::fmt::Debug,
{
    type Output = f64;
    fn dot(&self, rhs: &Prob<X>) -> f64 {
        self.vector.dot(&rhs.vector)
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
