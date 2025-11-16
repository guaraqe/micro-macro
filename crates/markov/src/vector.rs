use ndarray::{linalg::Dot, Array1};
use std::collections::BTreeMap;
use std::ops::{Add, Div, Mul, Sub};
use std::rc::Rc;

use crate::ix_map::IxMap;


//##########################################################
// Struct
//##########################################################

/// Vector with a bidirectional map for labels X.
#[derive(Debug, Clone)]
pub struct Vector<X> {
    pub values: Array1<f64>,
    pub ix_map: Rc<IxMap<X>>,
}

//##########################################################
// Impls
//##########################################################

impl<X> Vector<X>
where
    X: Ord + Clone,
{
    // Build from an association list
    pub fn from_assoc(assoc: impl IntoIterator<Item = (X, f64)>) -> Self {
        let mut map: BTreeMap<X, f64> = BTreeMap::new();

        for (x, n) in assoc.into_iter() {
            *map.entry(x).or_insert(0.0) += n;
        }

        let size = map.len();
        let mut keys = Vec::new();
        let mut values = Array1::zeros(size);

        for (i, (x, n)) in map.into_iter().enumerate() {
            keys.push(x.clone());
            values[i] = n;
        }

        let ix_map = IxMap::from_distinct_sorted(keys);

        Self { values, ix_map: Rc::new(ix_map) }
    }

    // Build from an manual association list
    pub fn unsafe_from_assoc<'a>(
        ix_map: &Rc<IxMap<X>>,
        ixes: impl IntoIterator<Item = &'a usize>,
        vals: impl IntoIterator<Item = &'a f64>,
    ) -> Self {
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
    pub fn get(&self, x: &X) -> Option<f64> {
        self.ix_map
            .index_of(x)
            .and_then(|i| self.values.get(i))
            .copied()
    }

    // Map each value, mutably
    pub fn mapv_inplace<F>(&mut self, f: F)
    where
        F: FnMut(f64) -> f64,
    {
        self.values.mapv_inplace(f);
    }

    /// Enumerate all values.
    pub fn values(&self) -> impl Iterator<Item = &f64> + '_ {
        self.values.iter()
    }

    /// Enumerate all (label, value) pairs.
    pub fn enumerate(&self) -> impl Iterator<Item = (X, f64)> + '_ {
        (0..self.values.len()).filter_map(move |i| {
            self.ix_map
                .value_of(i)
                .map(|x| (x.clone(), self.values[i]))
        })
    }

    pub fn norm(&self) -> f64 {
        self.values.iter().map(|x| *x * *x).sum::<f64>().sqrt()
    }

    pub fn normalize(&mut self) {
        let norm = self.norm();
        if norm == 0.0 {
            return;
        }
        self.mapv_inplace(|x| x / norm)
    }
}

/// Compute the maximum absolute difference between two vectors.
pub fn max_difference<X>(v1: &Vector<X>, v2: &Vector<X>) -> f64
where
    X: Clone + Ord,
{
    let pairs = v1.values().zip(v2.values());

    pairs
        .map(|(a, b)| (*a - *b).abs())
        .fold(0.0, |acc, x| if x > acc { x } else { acc })
}

pub fn orthonormalize<X>(
    vectors: Vec<Vector<X>>,
) -> Vec<Vector<X>>
where
    X: Clone + Ord,
{
    let mut bases: Vec<Vector<X>> = Vec::new();
    for vector in vectors.iter() {
        let mut result: Vector<X> = vector.clone();
        for base in bases.iter() {
            let proj: f64 = result.dot(base);
            result = &result - &(base * proj);
        }
        result.normalize();
        bases.push(result);
    }
    bases
}

pub fn rank<X>(vectors: Vec<Vector<X>>) -> usize
where
    X: Clone + Ord,
{
    let mut rank = 0;
    let bases = orthonormalize(vectors);
    for base in bases.iter() {
        if base.norm() > 1e-10 {
            rank += 1;
        }
    }
    rank
}

//##########################################################
// Traits
//##########################################################

impl<X> Dot<Vector<X>> for Vector<X>
where
    X: Ord,
{
    type Output = f64;
    fn dot(&self, rhs: &Vector<X>) -> f64 {
        self.values.dot(&rhs.values)
    }
}

impl<'b, X> Add<&'b Vector<X>> for &Vector<X>
where
    X: Clone,
{
    type Output = Vector<X>;
    fn add(self, rhs: &'b Vector<X>) -> Self::Output {
        Vector {
            ix_map: self.ix_map.clone(),
            values: &self.values + &rhs.values,
        }
    }
}

impl<'b, X> Sub<&'b Vector<X>> for &Vector<X>
where
    X: Clone,
{
    type Output = Vector<X>;
    fn sub(self, rhs: &'b Vector<X>) -> Self::Output {
        Vector {
            ix_map: self.ix_map.clone(),
            values: &self.values - &rhs.values,
        }
    }
}

impl<'b, X> Mul<&'b Vector<X>> for &Vector<X>
where
    X: Clone,
{
    type Output = Vector<X>;
    fn mul(self, rhs: &'b Vector<X>) -> Self::Output {
        Vector {
            ix_map: self.ix_map.clone(),
            values: &self.values * &rhs.values,
        }
    }
}

impl<X> Mul<f64> for &Vector<X>
where
    X: Clone,
{
    type Output = Vector<X>;
    fn mul(self, rhs: f64) -> Self::Output {
        Vector {
            ix_map: self.ix_map.clone(),
            values: &self.values * rhs,
        }
    }
}

impl<X> Div<f64> for &Vector<X>
where
    X: Clone,
{
    type Output = Vector<X>;
    fn div(self, rhs: f64) -> Self::Output {
        Vector {
            ix_map: self.ix_map.clone(),
            values: &self.values / rhs,
        }
    }
}

//##########################################################
// Tests
//##########################################################

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


