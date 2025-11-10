/// Simple bidirectional map between values and indices.
/// Values are stored in sorted order and lookups use binary search.
#[derive(Debug, Clone)]
pub struct IxMap<T> {
    values: Vec<T>,
}

impl<T: Ord + Clone> IxMap<T> {
    pub fn len(&self) -> usize {
        self.values.len()
    }
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    pub fn index_of(&self, x: &T) -> Option<usize> {
        self.values.binary_search(x).ok()
    }
    pub fn value_of(&self, i: usize) -> Option<&T> {
        self.values.get(i)
    }

    /// Build from a pre-sorted iterator of distinct values.
    /// The input MUST be sorted and contain no duplicates.
    pub fn from_distinct_sorted<I: IntoIterator<Item = T>>(
        vals: I,
    ) -> Self {
        Self {
            values: vals.into_iter().collect(),
        }
    }

    /// Iterator over all (index, value) pairs
    pub fn iter(&self) -> impl Iterator<Item = (usize, &T)> {
        self.values.iter().enumerate()
    }
}
