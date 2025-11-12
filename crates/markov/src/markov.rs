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
            if matches!(
                w.partial_cmp(&N::zero()),
                Some(Ordering::Less)
            ) {
                return Err(BuildError::Negative);
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
            if !matches!(
                s.partial_cmp(&N::zero()),
                Some(Ordering::Greater)
            ) {
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

    /// Get a column as a Vector<A, N>.
    /// Can be implemented efficiently by multiplying with a basis vector.
    pub fn get_column(
        &self,
        b: &B,
    ) -> Option<crate::vector::Vector<A, N>>
    where
        N: ndarray::ScalarOperand + 'static + std::ops::AddAssign,
        for<'r> &'r N: std::ops::Mul<&'r N, Output = N>,
    {
        let j = self.cols.index_of(b)?;
        let m = self.rows.len();
        let mut result_vec = vec![N::zero(); m];

        // Extract column j from CSC matrix
        if let Some(col) = self.csc.outer_view(j) {
            for (row_idx, &val) in
                col.indices().iter().zip(col.data().iter())
            {
                result_vec[*row_idx] = val;
            }
        }

        let result_values = ndarray::Array1::from(result_vec);

        Some(crate::vector::Vector {
            values: result_values,
            map: self.rows.clone(),
        })
    }

    /// Get a row as a Prob<B, N>.
    /// Returns the probability distribution for a given row label.
    pub fn get_row(&self, a: &A) -> Option<crate::prob::Prob<B, N>>
    where
        N: ndarray::ScalarOperand,
    {
        let i = self.rows.index_of(a)?;
        let n = self.cols.len();
        let mut result_vec = vec![N::zero(); n];

        // Convert to CSR to efficiently access row i
        let csr = self.csc.to_csr();
        if let Some(row) = csr.outer_view(i) {
            for (col_idx, &val) in
                row.indices().iter().zip(row.data().iter())
            {
                result_vec[*col_idx] = val;
            }
        }

        let result_probs = ndarray::Array1::from(result_vec);

        Some(crate::prob::Prob {
            probs: result_probs,
            map: self.cols.clone(),
        })
    }

    /// Enumerate all (row_label, col_label, value) triplets.
    /// Returns an iterator over (A, B, N) tuples for non-zero entries.
    pub fn enumerate(&self) -> impl Iterator<Item = (A, B, N)> + '_
    where
        N: Copy,
    {
        // Use CSC format to iterate through all non-zero entries
        self.csc.iter().filter_map(
            move |(val, (row_idx, col_idx))| {
                let row_label = self.rows.value_of(row_idx)?;
                let col_label = self.cols.value_of(col_idx)?;
                Some((row_label.clone(), col_label.clone(), *val))
            },
        )
    }
}

// Implement equilibrium computation for square matrices (A = B)
impl<A, N> Markov<A, A, N>
where
    A: Ord + Clone + std::fmt::Debug,
    N: Float
        + Default
        + ndarray::ScalarOperand
        + 'static
        + std::ops::AddAssign,
    for<'r> &'r N: std::ops::Mul<&'r N, Output = N>,
{
    /// Compute equilibrium distribution using power iteration.
    ///
    /// Repeatedly applies: p_new = p · M until convergence.
    /// Convergence is reached when max|p_new - p| < tolerance.
    ///
    /// This method is only available for square matrices (where row and column labels are the same type).
    ///
    /// # Arguments
    /// * `initial` - Starting probability distribution
    /// * `tolerance` - Maximum absolute difference for convergence (e.g., 1e-4)
    /// * `max_iterations` - Maximum number of iterations (e.g., 100)
    ///
    /// # Returns
    /// The equilibrium distribution (or best approximation after max_iterations)
    pub fn compute_equilibrium(
        &self,
        initial: &crate::prob::Prob<A, N>,
        tolerance: N,
        max_iterations: usize,
    ) -> crate::prob::Prob<A, N> {
        let mut current = initial.clone();

        for _ in 0..max_iterations {
            // p_new = p · M (left multiplication)
            let next = current.dot(self);

            // Check convergence
            let diff = current.max_difference(&next);
            if diff < tolerance {
                return next;
            }

            current = next;
        }

        // Return best approximation after max iterations
        current
    }

    /// Compute the entropy rate of the Markov chain using natural logarithm.
    /// H_rate = -Σ_i π_i Σ_j P_ij ln(P_ij)
    /// Uses the convention that 0·ln(0) = 0.
    ///
    /// # Arguments
    /// * `stationary` - The stationary distribution π
    ///
    /// # Returns
    /// The entropy rate, measuring the average unpredictability per transition
    pub fn entropy_rate(
        &self,
        stationary: &crate::prob::Prob<A, N>,
    ) -> N {
        let csr = self.csc.to_csr();
        let mut total = N::zero();

        for i in 0..self.rows.len() {
            let pi = stationary.probs[i];
            if pi <= N::zero() {
                continue;
            }

            if let Some(row) = csr.outer_view(i) {
                for &val in row.data().iter() {
                    if val > N::zero() {
                        total = total + pi * val * val.ln();
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
        stationary: &crate::prob::Prob<A, N>,
    ) -> N {
        let csr = self.csc.to_csr();
        let mut total = N::zero();
        let two = N::one() + N::one();

        // Iterate over all pairs (i, j)
        for i in 0..self.rows.len() {
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
                    total = total + diff;
                }
            }
        }

        // Divide by 2 since we count each pair twice
        total / two
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

// Implement Dot<Vector> for Markov: matrix · vector -> vector
impl<A, B, N> Dot<crate::vector::Vector<B, N>> for Markov<A, B, N>
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
    type Output = crate::vector::Vector<A, N>;

    /// Matrix-vector dot product: self · rhs (right multiplication)
    /// Treats rhs as a column vector with B labels.
    /// Returns Vector<A, N> with row labels.
    ///
    /// Computes: result[a] = sum_b matrix[a, b] * rhs[b]
    /// Uses sprs optimized CSC matrix-vector product via prod module
    fn dot(
        &self,
        rhs: &crate::vector::Vector<B, N>,
    ) -> crate::vector::Vector<A, N> {
        // Use sprs optimized CSC matrix · vector multiplication
        let m = self.rows.len();
        let mut result_vec = vec![N::zero(); m];
        prod::mul_acc_mat_vec_csc(
            self.csc.view(),
            rhs.values.as_slice().unwrap(),
            &mut result_vec,
        );
        let result_values = ndarray::Array1::from(result_vec);

        crate::vector::Vector {
            values: result_values,
            map: self.rows.clone(),
        }
    }
}
