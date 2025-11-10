use ndarray::linalg::Dot;
use num_traits::Float;
use sprs::{prod, CsMat, TriMat};

use crate::ix_map::IxMap;
use crate::prob::BuildError;
use std::cmp::Ordering;

/// Row-stochastic Markov kernel with CSC storage:
/// rows correspond to `A`, columns to `B`.
#[derive(Debug, Clone)]
pub struct Markov<A, B, N> {
    /// Sparse matrix (m x n), each *row* sums to 1.
    /// Stored as CSC for your requested layout.
    pub csc: CsMat<N>,
    /// Row labels (A) <-> row indices
    pub rows: IxMap<A>,
    /// Column labels (B) <-> column indices
    pub cols: IxMap<B>,
}

impl<A, B, N> Markov<A, B, N>
where
    A: Ord + Clone + std::fmt::Debug,
    B: Ord + Clone + std::fmt::Debug,
    N: Float + Default,
{
    pub fn from_assoc(
        m: usize,
        n: usize,
        assoc: impl IntoIterator<Item = (A, B, N)>,
    ) -> Result<Self, BuildError> {
        if m == 0 || n == 0 {
            return Err(BuildError::EmptyMatrix);
        }

        // Collect and sort triplets by (A, B)
        let mut triplets: Vec<(A, B, N)> =
            assoc.into_iter().collect();
        triplets.sort_by(|a, b| (&a.0, &a.1).cmp(&(&b.0, &b.1)));

        // Check positivity and aggregate duplicates
        let mut aggregated: Vec<(A, B, N)> = Vec::new();
        for (a, b, w) in triplets {
            if !matches!(w.partial_cmp(&N::zero()), Some(Ordering::Greater)) {
                return Err(BuildError::NonPositive);
            }
            if let Some(last) = aggregated.last_mut() {
                if last.0 == a && last.1 == b {
                    last.2 = last.2 + w;
                    continue;
                }
            }
            aggregated.push((a, b, w));
        }

        // Extract sorted distinct row and column labels
        let mut row_labels: Vec<A> = Vec::new();
        let mut col_labels: Vec<B> = Vec::new();
        for (a, b, _) in &aggregated {
            if row_labels.last() != Some(a) {
                row_labels.push(a.clone());
            }
            if col_labels.last() != Some(b) {
                col_labels.push(b.clone());
            }
        }
        row_labels.sort();
        row_labels.dedup();
        col_labels.sort();
        col_labels.dedup();

        let row_map = IxMap::from_distinct_sorted(row_labels);
        let col_map = IxMap::from_distinct_sorted(col_labels);

        if row_map.len() != m {
            return Err(BuildError::SizeMismatch(m, row_map.len()));
        }
        if col_map.len() != n {
            return Err(BuildError::SizeMismatch(n, col_map.len()));
        }

        // Convert labels to indices and build buckets
        let mut buckets: Vec<((usize, usize), N)> = Vec::new();
        for (a, b, w) in aggregated {
            let i = row_map.index_of(&a).expect("row label in map");
            let j = col_map.index_of(&b).expect("col label in map");
            buckets.push(((i, j), w));
        }

        // Compute row sums
        let mut row_sums = vec![N::zero(); m];
        for &((i, _j), w) in &buckets {
            row_sums[i] = row_sums[i] + w;
        }
        for (i, s) in row_sums.iter().enumerate() {
            if !matches!(s.partial_cmp(&N::zero()), Some(Ordering::Greater)) {
                let name = row_map
                    .value_of(i)
                    .map(|x| format!("{x:?}"))
                    .unwrap_or_else(|| i.to_string());
                return Err(BuildError::EmptyRow(name));
            }
        }

        // Build a CSR via TriMat, normalized per row, then convert to CSC
        let mut tri: TriMat<N> =
            TriMat::with_capacity((m, n), buckets.len());
        for ((i, j), w) in buckets {
            tri.add_triplet(i, j, w / row_sums[i]);
        }

        let csr: CsMat<N> = tri.to_csr();
        let csc: CsMat<N> = csr.to_csc();

        Ok(Self {
            csc,
            rows: row_map,
            cols: col_map,
        })
    }

    /// Get P[B = b | A = a] if both labels are known; otherwise None.
    /// Storage is CSC, so we read the column for `b` and lookup row `a`.
    pub fn p(&self, a: &A, b: &B) -> Option<N> {
        let i = self.rows.index_of(a)?;
        let j = self.cols.index_of(b)?;
        // Column view (since CSC). Then get row i in that sparse column.
        self.csc.outer_view(j)?.get(i).copied()
    }

    /// Same as `p` but returns 0 if (a,b) is absent or unknown.
    pub fn p0(&self, a: &A, b: &B) -> N {
        self.p(a, b).unwrap_or(N::zero())
    }

    /// Row index / column index helpers (handy to cache when looping).
    pub fn row_index(&self, a: &A) -> Option<usize> {
        self.rows.index_of(a)
    }
    pub fn col_index(&self, b: &B) -> Option<usize> {
        self.cols.index_of(b)
    }

    /// Get a sparse row as (col_index, value) pairs.
    /// Efficient by converting to a CSR *view* and reading row i.
    pub fn row_entries_by_index(
        &self,
        i: usize,
    ) -> Option<Vec<(usize, N)>> {
        let csr = self.csc.to_csr(); // cheap (shared data) conversion
        let row = csr.outer_view(i)?;
        Some(
            row.indices()
                .iter()
                .zip(row.data().iter().copied())
                .map(|(&j, v)| (j, v))
                .collect(),
        )
    }

    /// Convenience: get all (B, value) pairs for a given A.
    pub fn row_entries(&self, a: &A) -> Option<Vec<(&B, N)>> {
        let i = self.rows.index_of(a)?;
        let pairs = self.row_entries_by_index(i)?;
        Some(
            pairs
                .into_iter()
                .filter_map(|(j, v)| {
                    self.cols.value_of(j).map(|b| (b, v))
                })
                .collect(),
        )
    }
}

// Implement Dot<Prob> for Markov: matrix · vector -> vector
impl<A, B, N> Dot<crate::prob::Prob<B, N>> for Markov<A, B, N>
where
    A: Ord + Clone + std::fmt::Debug,
    B: Ord + Clone + std::fmt::Debug,
    N: Float
        + Default
        + ndarray::ScalarOperand
        + 'static
        + std::ops::AddAssign,
    for<'r> &'r N: std::ops::Mul<&'r N, Output = N>,
{
    type Output = crate::prob::Prob<A, N>;

    /// Matrix-vector dot product: self · rhs (right multiplication)
    /// Treats rhs as a column vector with B labels.
    /// Returns Prob<A, N> with row labels.
    ///
    /// Computes: result[a] = sum_b matrix[a, b] * rhs[b]
    /// Uses sprs CSC matrix-vector product via prod module
    fn dot(
        &self,
        rhs: &crate::prob::Prob<B, N>,
    ) -> crate::prob::Prob<A, N> {
        // Use sprs optimized CSC matrix · vector multiplication
        let m = self.rows.len();
        let mut result_vec = vec![N::zero(); m];
        prod::mul_acc_mat_vec_csc(
            self.csc.view(),
            rhs.probs.as_slice().unwrap(),
            &mut result_vec,
        );
        let result_probs = ndarray::Array1::from(result_vec);

        crate::prob::Prob {
            probs: result_probs,
            map: self.rows.clone(),
        }
    }
}
