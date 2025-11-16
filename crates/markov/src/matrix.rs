use ndarray::{linalg::Dot, Array1};
use sprs::binop::csmat_binop;
use sprs::{CsMat, TriMat};
use std::collections::BTreeMap;
use std::rc::Rc;

use crate::ix_map::IxMap;
use crate::vector::Vector;

/// Matrix in CSC storage
#[derive(Debug, Clone)]
pub struct Matrix<X, Y> {
    /// Stored as CSC for your requested layout.
    pub values: CsMat<f64>,
    /// Row labels (X) <-> row indices
    pub x_ix_map: Rc<IxMap<X>>,
    /// Column labels (Y) <-> column indices
    pub y_ix_map: Rc<IxMap<Y>>,
}

impl<X, Y> Matrix<X, Y>
where
    X: Ord + Clone,
    Y: Ord + Clone,
{
    pub fn from_assoc(
        assoc: impl IntoIterator<Item = (X, Y, f64)>,
    ) -> Self {
        let mut x_map: BTreeMap<X, Vec<(Y, f64)>> = BTreeMap::new();

        for (x, y, n) in assoc.into_iter() {
            x_map.entry(x).or_default().push((y, n));
        }

        let x_size: usize = x_map.len();
        let mut x_keys: Vec<X> = Vec::new();
        let mut x_values: Vec<(Y, usize, f64)> = Vec::new();

        for (i, (x, v)) in x_map.into_iter().enumerate() {
            x_keys.push(x.clone());
            let x_value: Vec<(Y, usize, f64)> =
                v.into_iter().map(|(y, n)| (y, i, n)).collect();
            x_values.extend(x_value);
        }

        let mut y_map: BTreeMap<Y, Vec<(usize, f64)>> = BTreeMap::new();

        for (y, i, n) in x_values.into_iter() {
            y_map.entry(y).or_default().push((i, n));
        }

        let y_size: usize = y_map.len();
        let mut y_keys: Vec<Y> = Vec::new();
        let mut triples: Vec<(usize, usize, f64)> = Vec::new();

        for (j, (y, v)) in y_map.into_iter().enumerate() {
            y_keys.push(y.clone());
            let triple: Vec<(usize, usize, f64)> =
                v.into_iter().map(|(i, n)| (i, j, n)).collect();
            triples.extend(triple);
        }

        let mut trimat = TriMat::new((x_size, y_size));

        for (i, j, v) in triples {
            trimat.add_triplet(i, j, v);
        }

        let values: CsMat<f64> = trimat.to_csc();

        let x_ix_map = IxMap::from_distinct_sorted(x_keys);
        let y_ix_map = IxMap::from_distinct_sorted(y_keys);

        Self {
            values,
            x_ix_map: Rc::new(x_ix_map),
            y_ix_map: Rc::new(y_ix_map),
        }
    }

    /// Get a column as a Vector<X>.
    pub fn get_column(&self, col_index: &Y) -> Option<Vector<X>> {
        let ix = self.y_ix_map.index_of(col_index)?;
        let vector =
            get_csmat_column(&self.values, &self.x_ix_map, ix);
        Some(vector)
    }

    /// Get columns as a Vec of Vector<X>.
    pub fn get_columns(&self) -> Vec<Vector<X>> {
        let mut columns = Vec::new();
        for ix in 0..self.y_ix_map.len() {
            let vector =
                get_csmat_column(&self.values, &self.x_ix_map, ix);
            columns.push(vector)
        }
        columns
    }

    pub fn get_rows_sums(&self) -> Vector<X> {
        let mut row_sums = Array1::zeros(self.x_ix_map.len());

        for col in self.values.outer_iterator() {
            for (&row, &val) in
                col.indices().iter().zip(col.data().iter())
            {
                row_sums[row] += val;
            }
        }

        Vector {
            ix_map: self.x_ix_map.clone(),
            values: row_sums,
        }
    }

    // Applies (m_ij, v_i) -> f(m_ij, v_i)
    pub fn map_rows<F: Fn(f64, f64) -> f64>(
        &self,
        vector: &Vector<X>,
        f: F,
    ) -> Matrix<X, Y> {
        let mut mat = self.values.clone();
        for mut col in mat.outer_iterator_mut() {
            for (row, val) in col.iter_mut() {
                *val = f(*val, vector.values[row]);
            }
        }
        Matrix {
            values: mat,
            x_ix_map: self.x_ix_map.clone(),
            y_ix_map: self.y_ix_map.clone(),
        }
    }

    pub fn transpose(&self) -> Matrix<Y, X> {
        let transpose = self.values.view().transpose_into().to_csc();
        Matrix {
            x_ix_map: self.y_ix_map.clone(),
            y_ix_map: self.x_ix_map.clone(),
            values: transpose,
        }
    }

    pub fn binop<F: Fn(f64, f64) -> f64>(
        &self,
        other: &Matrix<X, Y>,
        f: F,
    ) -> Matrix<X, Y> {
        Matrix {
            x_ix_map: self.x_ix_map.clone(),
            y_ix_map: self.y_ix_map.clone(),
            values: csmat_binop(
                self.values.view(),
                other.values.view(),
                |x, y| f(*x, *y),
            ),
        }
    }
}

fn get_csmat_column<X>(
    matrix: &CsMat<f64>,
    ix_map: &Rc<IxMap<X>>,
    ix: usize,
) -> Vector<X>
where
    X: Ord + Clone,
{
    let col_view = matrix.outer_view(ix).unwrap();
    Vector::unsafe_from_assoc(
        ix_map,
        col_view.indices(),
        col_view.data(),
    )
}

// Vector Dot Matrix
impl<X, Y> Dot<Matrix<X, Y>> for Vector<X>
where
    X: Ord,
    Y: Ord + Clone,
{
    type Output = Vector<Y>;

    fn dot(&self, matrix: &Matrix<X, Y>) -> Vector<Y> {
        Vector {
            values: matrix.values.transpose_view().dot(&self.values),
            ix_map: matrix.y_ix_map.clone(),
        }
    }
}

/// Matrix dot Vector
impl<X, Y> Dot<Vector<Y>> for Matrix<X, Y>
where
    X: Ord + Clone,
    Y: Ord,
{
    type Output = Vector<X>;

    fn dot(&self, vector: &Vector<Y>) -> Vector<X> {
        Vector {
            values: self.values.dot(&vector.values),
            ix_map: self.x_ix_map.clone(),
        }
    }
}
