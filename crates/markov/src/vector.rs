use ndarray::{linalg::Dot, Array1};
use num_traits::Float;

use crate::ix_map::IxMap;

/// Vector with a bidirectional map for labels X.
/// Similar to Prob but without normalization requirement.
#[derive(Debug, Clone)]
pub struct Vector<X, N> {
    /// Dense vector of length n (not necessarily normalized).
    pub values: Array1<N>,
    /// Labels <-> indices
    pub map: IxMap<X>,
}

impl<X, N> Vector<X, N>
where
    X: Ord + Clone + std::fmt::Debug,
    N: Float + ndarray::ScalarOperand,
{
    pub fn from_assoc(
        size: usize,
        assoc: impl IntoIterator<Item = (X, N)>,
    ) -> Self {
        // Collect and sort pairs by key
        let mut pairs: Vec<(X, N)> = assoc.into_iter().collect();
        pairs.sort_by(|a, b| a.0.cmp(&b.0));

        // Aggregate duplicates
        let mut aggregated: Vec<(X, N)> = Vec::new();
        for (x, w) in pairs {
            if let Some(last) = aggregated.last_mut() {
                if last.0 == x {
                    last.1 = last.1 + w;
                    continue;
                }
            }
            aggregated.push((x, w));
        }

        // Extract sorted distinct keys for IxMap
        let keys: Vec<X> =
            aggregated.iter().map(|(x, _)| x.clone()).collect();
        let map = IxMap::from_distinct_sorted(keys);

        // Place values into fixed-size array
        let mut values = Array1::zeros(size);
        for (x, w) in aggregated {
            let i = map.index_of(&x).expect("built from same keys");
            values[i] = w;
        }

        Self { values, map }
    }

    /// Get value at label x if `x` is known; otherwise None.
    pub fn get(&self, x: &X) -> Option<N> {
        self.map
            .index_of(x)
            .and_then(|i| self.values.get(i))
            .copied()
    }

    /// Get value at label x, returning 0 if `x` is unknown.
    pub fn get0(&self, x: &X) -> N {
        self.get(x).unwrap_or(N::zero())
    }

    /// Direct access by index.
    pub fn get_at(&self, i: usize) -> Option<N> {
        self.values.get(i).copied()
    }

    /// Index of label.
    pub fn index_of(&self, x: &X) -> Option<usize> {
        self.map.index_of(x)
    }

    /// Element-wise multiplication with another vector.
    /// Returns a new vector with the same labels.
    pub fn mul_elementwise(
        &self,
        other: &Vector<X, N>,
    ) -> Vector<X, N> {
        let values = &self.values * &other.values;
        Vector {
            values,
            map: self.map.clone(),
        }
    }

    /// Enumerate all (label, value) pairs.
    /// Returns an iterator over (X, N) tuples.
    pub fn enumerate(&self) -> impl Iterator<Item = (X, N)> + '_
    where
        N: Copy,
    {
        (0..self.values.len()).filter_map(move |i| {
            self.map.value_of(i).map(|x| (x.clone(), self.values[i]))
        })
    }
}

// Import Markov for the cross-type dot method
use crate::markov::Markov;

// Implement Dot<Vector> for Vector: vector · vector -> scalar
impl<X, N> Dot<Vector<X, N>> for Vector<X, N>
where
    X: Ord + Clone + std::fmt::Debug,
    N: Float + Default + ndarray::ScalarOperand + 'static,
{
    type Output = N;

    /// Vector-vector dot product: self · other
    /// Uses ndarray's optimized dot product implementation
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
