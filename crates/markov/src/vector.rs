use ndarray::{linalg::Dot, Array1, ScalarOperand};
use num_traits::Float;
use std::collections::BTreeMap;
use std::ops::{Add, AddAssign, Div, Mul, Sub};

use crate::ix_map::IxMap;

//##########################################################
// Struct
//##########################################################

/// Vector with a bidirectional map for labels X.
#[derive(Debug, Clone)]
pub struct Vector<X, N> {
    pub values: Array1<N>,
    pub ix_map: IxMap<X>,
}

//##########################################################
// Impls
//##########################################################

impl<X, N> Vector<X, N>
where
    X: Ord + Clone,
    N: Float,
{
    // Build from an association list
    pub fn from_assoc(assoc: impl IntoIterator<Item = (X, N)>) -> Self
    where
        N: AddAssign,
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
        N: 'a,
    {
        let mut values = Array1::zeros(ix_map.len());

        for (r, v) in ixes.into_iter().zip(vals.into_iter()) {
            values[*r] = *v;
        }

        Self {
            values,
            ix_map: ix_map.clone(),
        }
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

    pub fn norm(&self) -> N
    where
        N: Float + std::iter::Sum,
    {
        self.values.iter().map(|x| *x * *x).sum::<N>().sqrt()
    }

    pub fn normalize(&mut self)
    where
        N: Float + std::iter::Sum,
    {
        let norm = self.norm();
        if norm == N::zero() {
            return;
        }
        self.mapv_inplace(|x| x / norm)
    }
}

/// Compute the maximum absolute difference between two vectors.
pub fn max_difference<X, N>(v1: &Vector<X, N>, v2: &Vector<X, N>) -> N
where
    X: Clone + Ord,
    N: Float,
{
    let pairs = v1.values().zip(v2.values());

    pairs
        .map(|(a, b)| (*a - *b).abs())
        .fold(N::zero(), |acc, x| if x > acc { x } else { acc })
}

pub fn orthonormalize<X, N>(
    vectors: Vec<Vector<X, N>>,
) -> Vec<Vector<X, N>>
where
    X: Clone + Ord,
    N: Clone + Float + std::iter::Sum + ndarray::ScalarOperand,
{
    let mut bases: Vec<Vector<X, N>> = Vec::new();
    for vector in vectors.iter() {
        let mut result: Vector<X, N> = vector.clone();
        for base in bases.iter() {
            let proj: N = result.dot(base);
            result = &result - &(base * proj);
        }
        result.normalize();
        bases.push(result);
    }
    bases
}

pub fn rank<X, N>(vectors: Vec<Vector<X, N>>) -> usize
where
    X: Clone + Ord,
    N: Clone + Float + std::iter::Sum + ndarray::ScalarOperand,
{
    let mut rank = 0;
    let bases = orthonormalize(vectors);
    for base in bases.iter() {
        if base.norm() > N::from(1e-10).unwrap() {
            rank += 1;
        }
    }
    rank
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_orthonormalize_full() {
        let v1 = Vector::from_assoc(vec![(0, 5.0), (1, 0.0)]);
        let v2 = Vector::from_assoc(vec![(0, 4.0), (1, 3.0)]);
        let result = orthonormalize(vec![v1, v2]);

        let expected0 = Vector::from_assoc(vec![(0, 1.0), (1, 0.0)]);
        let expected1 = Vector::from_assoc(vec![(0, 0.0), (1, 1.0)]);

        assert!(max_difference(&result[0], &expected0) < 1e-10);
        assert!(max_difference(&result[1], &expected1) < 1e-10);
        assert_eq!(rank(result),2);
    }

    #[test]
    fn test_orthonormalize_not_full() {
        let v1 = Vector::from_assoc(vec![(0, 5.0), (1, 0.0)]);
        let v2 = Vector::from_assoc(vec![(0, 4.0), (1, 0.0)]);
        let result = orthonormalize(vec![v1, v2]);

        let expected0 = Vector::from_assoc(vec![(0, 1.0), (1, 0.0)]);
        let expected1 = Vector::from_assoc(vec![(0, 0.0), (1, 0.0)]);

        assert!(max_difference(&result[0], &expected0) < 1e-10);
        assert!(max_difference(&result[1], &expected1) < 1e-10);
        assert_eq!(rank(result),1);
    }
}

//##########################################################
// Traits
//##########################################################

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

impl<'a, 'b, X, N> Add<&'b Vector<X, N>> for &'a Vector<X, N>
where
    X: Clone,
    N: Float,
{
    type Output = Vector<X, N>;
    fn add(self, rhs: &'b Vector<X, N>) -> Self::Output {
        Vector {
            ix_map: self.ix_map.clone(),
            values: &self.values + &rhs.values,
        }
    }
}

impl<'a, 'b, X, N> Sub<&'b Vector<X, N>> for &'a Vector<X, N>
where
    X: Clone,
    N: Float,
{
    type Output = Vector<X, N>;
    fn sub(self, rhs: &'b Vector<X, N>) -> Self::Output {
        Vector {
            ix_map: self.ix_map.clone(),
            values: &self.values - &rhs.values,
        }
    }
}

impl<'a, 'b, X, N> Mul<&'b Vector<X, N>> for &'a Vector<X, N>
where
    X: Clone,
    N: Float,
{
    type Output = Vector<X, N>;
    fn mul(self, rhs: &'b Vector<X, N>) -> Self::Output {
        Vector {
            ix_map: self.ix_map.clone(),
            values: &self.values * &rhs.values,
        }
    }
}

impl<'a, X, N> Mul<N> for &'a Vector<X, N>
where
    X: Clone,
    N: Float + ScalarOperand,
{
    type Output = Vector<X, N>;

    fn mul(self, rhs: N) -> Self::Output {
        Vector {
            ix_map: self.ix_map.clone(),
            values: &self.values * rhs,
        }
    }
}

impl<'a, X, N> Div<N> for &'a Vector<X, N>
where
    X: Clone,
    N: Float + ScalarOperand,
{
    type Output = Vector<X, N>;

    fn div(self, rhs: N) -> Self::Output {
        Vector {
            ix_map: self.ix_map.clone(),
            values: &self.values / rhs,
        }
    }
}
