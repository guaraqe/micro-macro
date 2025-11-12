// -------------------------------------------------------------------
// Versioned
// -------------------------------------------------------------------

#[derive(Clone)]
pub struct Versioned<T> {
    version: u64,
    data: T,
}
impl<T> Versioned<T> {
    pub fn new(data: T) -> Self {
        Self { version: 0, data }
    }
    pub fn get(&self) -> &T {
        &self.data
    }
    pub fn get_mut(&mut self) -> &mut T {
        self.version = self.version.wrapping_add(1);
        &mut self.data
    }
    pub fn set(&mut self, data: T) {
        self.data = data;
        self.version = self.version.wrapping_add(1);
    }
    pub fn version(&self) -> u64 {
        self.version
    }
}

// -------------------------------------------------------------------
// Memoized
// -------------------------------------------------------------------

pub struct Memoized<S, K, V> {
    version: u64,
    last_key: Option<K>,
    last_value: Option<V>,
    get_key: Box<dyn Fn(&S) -> K>,
    calc: Box<dyn Fn(&S) -> V>,
}

impl<S, K, V> Memoized<S, K, V>
where
    K: PartialEq,
{
    pub fn new(
        get_key: impl Fn(&S) -> K + 'static,
        calc: impl Fn(&S) -> V + 'static,
    ) -> Self {
        Self {
            version: 0,
            last_key: None,
            last_value: None,
            get_key: Box::new(get_key),
            calc: Box::new(calc),
        }
    }

    /// Recompute only if the key changed; return a reference to the cached value.
    pub fn get<'a>(&'a mut self, store: &S) -> &'a V {
        let key = (self.get_key)(store);
        let changed = match &self.last_key {
            Some(k) => *k != key,
            None => true,
        };
        if changed {
            let val = (self.calc)(store);
            self.last_key = Some(key);
            self.last_value = Some(val);
            self.version = self.version.wrapping_add(1);
        }
        self.last_value.as_ref().unwrap()
    }

    /// Get mutable reference to the cached value without recomputation.
    /// This allows modifying the cached value (e.g., node positions) without triggering recalculation.
    pub fn get_mut<'a>(&'a mut self, store: &S) -> &'a mut V {
        // Ensure value is computed
        let key = (self.get_key)(store);
        let changed = match &self.last_key {
            Some(k) => *k != key,
            None => true,
        };
        if changed {
            let val = (self.calc)(store);
            self.last_key = Some(key);
            self.last_value = Some(val);
            self.version = self.version.wrapping_add(1);
        }
        self.last_value.as_mut().unwrap()
    }

    /// Get the version of the cached value.
    /// This increments each time the value is recomputed.
    pub fn version(&self) -> u64 {
        self.version
    }
}
