use ndarray::{linalg::Dot, Array1};
use num_traits::Float;
use std::collections::BTreeMap;

use crate::ix_map::IxMap;
use crate::markov::Markov;

/// Vector with a bidirectional map for labels X.
#[derive(Debug, Clone)]
pub struct Vector<X, N> {
    pub values: Array1<N>,
    pub map: IxMap<X>,
}

impl<X, N> Vector<X, N>
where
    X: Ord + Clone,
    N: Float + std::ops::AddAssign,
{
    // Build from an association list
    pub fn from_assoc(
        assoc: impl IntoIterator<Item = (X, N)>,
    ) -> Self {
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

        let map = IxMap::from_distinct_sorted(keys);

        Self { values, map }
    }

    /// Get value at label x if `x` is known; otherwise None.
    pub fn get(&self, x: &X) -> Option<N> {
        self.map
            .index_of(x)
            .and_then(|i| self.values.get(i))
            .copied()
    }

    /// Element-wise multiplication with another vector.
    pub fn mul(&self, other: &Vector<X, N>) -> Vector<X, N> {
        let values = &self.values * &other.values;
        Vector {
            values,
            map: self.map.clone(),
        }
    }

    /// Element-wise multiplication with another vector.
    pub fn mapv_inplace<F>(&mut self, f: F)
    where F: FnMut(N) -> N,
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
            self.map.value_of(i).map(|x| (x.clone(), self.values[i]))
        })
    }
}

impl<X, N> Dot<Vector<X, N>> for Vector<X, N>
where
    X: Ord + Clone + std::fmt::Debug,
    N: Float + Default + ndarray::ScalarOperand + 'static,
{
    type Output = N;
    fn dot(&self, rhs: &Vector<X, N>) -> N {
        self.values.dot(&rhs.values)
    }
}

// Implement Dot<Markov> for Vector: vector · matrix -> vector
impl<X, B, N> Dot<Markov<X, B, N>> for Vector<X, N>
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
    type Output = Vector<B, N>;

    /// Vector-matrix dot product: self · matrix (left multiplication)
    /// Treats self as a row vector, returns Vector<B, N> with column labels.
    ///
    /// Computes: result[b] = sum_x self[x] * matrix[x, b]
    /// Uses sprs CSC format to efficiently access columns
    fn dot(&self, matrix: &Markov<X, B, N>) -> Vector<B, N> {
        // For vector · matrix, compute dot product of vector with each column
        let n = matrix.cols.len();
        let mut result_vec = vec![N::zero(); n];

        for (j, acc) in result_vec.iter_mut().enumerate() {
            if let Some(col) = matrix.csc.outer_view(j) {
                for (row_idx, &val) in
                    col.indices().iter().zip(col.data().iter())
                {
                    *acc += self.values[*row_idx] * val;
                }
            }
        }

        let result_values = ndarray::Array1::from(result_vec);

        Vector {
            values: result_values,
            map: matrix.cols.clone(),
        }
    }
}
