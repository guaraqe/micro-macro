use std::collections::HashMap;
use std::hash::Hash;

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
    pub probs: Vec<N>,
    /// Labels <-> indices
    pub map: IxMap<X>,
}

impl<X, N> Prob<X, N>
where
    X: Eq + Hash + Clone + std::fmt::Debug,
    N: Float,
{
    pub fn from_assoc(
        size: usize,
        assoc: impl IntoIterator<Item = (X, N)>,
    ) -> Result<Self, BuildError> {
        if size == 0 {
            return Err(BuildError::SizeMismatch(0, 0));
        }

        // Sum duplicates and check positivity for provided values
        let mut sums: HashMap<X, N> = HashMap::new();
        for (x, w) in assoc {
            if !(w > N::zero()) {
                return Err(BuildError::NonPositive);
            }
            *sums.entry(x).or_insert(N::zero()) =
                *sums.get(&x).unwrap_or(&N::zero()) + w;
        }

        // Build label order (first-seen over the map iteration is arbitrary; if you want
        // deterministic order, feed a predefined list instead).
        let map = IxMap::from_distinct(sums.keys().cloned());
        if map.len() > size {
            return Err(BuildError::SizeMismatch(size, map.len()));
        }

        // Place values into fixed-size vector
        let mut probs = vec![N::zero(); size];
        let mut total = N::zero();
        for (x, w) in sums {
            let i = map.index_of(&x).expect("built from same keys");
            probs[i] = w;
            total = total + w;
        }
        if !(total > N::zero()) {
            return Err(BuildError::NonPositive);
        }

        // Normalize
        for p in &mut probs {
            *p = *p / total;
        }

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
