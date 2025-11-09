use std::collections::HashMap;
use std::hash::Hash;

/// Simple bidirectional map between values and indices.
/// Order is assigned by first-seen in the input you provide to builders.
#[derive(Debug, Clone)]
pub struct IxMap<T> {
    index_of: HashMap<T, usize>,
    value_of: Vec<T>,
}

impl<T: Eq + Hash + Clone> IxMap<T> {
    pub fn len(&self) -> usize {
        self.value_of.len()
    }
    pub fn is_empty(&self) -> bool {
        self.value_of.is_empty()
    }

    pub fn index_of(&self, x: &T) -> Option<usize> {
        self.index_of.get(x).copied()
    }
    pub fn value_of(&self, i: usize) -> Option<&T> {
        self.value_of.get(i)
    }

    /// Build from an iterator of distinct values (first-seen order).
    pub fn from_distinct<I: IntoIterator<Item = T>>(vals: I) -> Self {
        let mut index_of = HashMap::new();
        let mut value_of = Vec::new();
        for v in vals {
            if let std::collections::hash_map::Entry::Vacant(e) =
                index_of.entry(v.clone())
            {
                e.insert(value_of.len());
                value_of.push(v);
            }
        }
        Self { index_of, value_of }
    }
}
