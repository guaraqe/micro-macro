use ndarray::{linalg::Dot, Array1};
use num_traits::Float;
use std::collections::BTreeMap;

use crate::ix_map::IxMap;

/// Vector with a bidirectional map for labels X.
#[derive(Debug, Clone)]
pub struct Vector<X, N> {
    pub values: Array1<N>,
    pub ix_map: IxMap<X>,
}

impl<X, N> Vector<X, N>
where
    X: Ord + Clone,
    N: Float,
{
    // Build from an association list
    pub fn from_assoc(assoc: impl IntoIterator<Item = (X, N)>) -> Self
    where
        N: std::ops::AddAssign,
    {
        let mut map: BTreeMap<X, N> = BTreeMap::new();

        for (x, n) in assoc.into_iter() {
            *map.entry(x).or_insert(N::zero()) += n;
        }

        let size = map.len();
        let mut keys = Vec::new();
        let mut values = Array1::zeros(size);

        for (i, (x, n)) in map.into_iter().enumerate() {
            keys.push(x.clone());
            values[i] = n;
        }

        let ix_map = IxMap::from_distinct_sorted(keys);

        Self { values, ix_map }
    }

    // Build from an manual association list
    pub fn unsafe_from_assoc<'a>(
        ix_map: &IxMap<X>,
        ixes: impl IntoIterator<Item = &'a usize>,
        vals: impl IntoIterator<Item = &'a N>,
    ) -> Self
    where
      N: 'a
    {
        let mut values = Array1::zeros(ix_map.len());

        for (r, v) in ixes.into_iter().zip(vals.into_iter()) {
            values[*r] = *v;
        }

        Self { values, ix_map: ix_map.clone() }
    }

    /// Lenght.
    pub fn len(&self) -> usize {
        self.values.len()
    }

    /// Is empty
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    /// Get value at label x if `x` is known; otherwise None.
    pub fn get(&self, x: &X) -> Option<N> {
        self.ix_map
            .index_of(x)
            .and_then(|i| self.values.get(i))
            .copied()
    }

    /// Element-wise multiplication with another vector.
    pub fn mul(&self, other: &Vector<X, N>) -> Vector<X, N> {
        let values = &self.values * &other.values;
        Vector {
            values,
            ix_map: self.ix_map.clone(),
        }
    }

    // Map each value, mutably
    pub fn mapv_inplace<F>(&mut self, f: F)
    where
        F: FnMut(N) -> N,
    {
        self.values.mapv_inplace(f);
    }

    /// Enumerate all values.
    pub fn values(&self) -> impl Iterator<Item = &N> + '_
    where
        N: Copy,
    {
        self.values.iter()
    }

    /// Enumerate all (label, value) pairs.
    pub fn enumerate(&self) -> impl Iterator<Item = (X, N)> + '_
    where
        N: Copy,
    {
        (0..self.values.len()).filter_map(move |i| {
            self.ix_map
                .value_of(i)
                .map(|x| (x.clone(), self.values[i]))
        })
    }
}

impl<X, N> Dot<Vector<X, N>> for Vector<X, N>
where
    X: Ord,
    N: Float + ndarray::ScalarOperand,
{
    type Output = N;
    fn dot(&self, rhs: &Vector<X, N>) -> N {
        self.values.dot(&rhs.values)
    }
}

/// Compute the maximum absolute difference between two vectors.
pub fn max_difference<X, N>(v1: &Vector<X, N>, v2: &Vector<X, N>) -> N
where
    X: Clone + Ord,
    N: Float + Ord,
{
    let pairs = v1.values().zip(v2.values());

    pairs
        .map(|(a, b)| (*a - *b).abs())
        .max()
        .unwrap_or(N::zero())
}
