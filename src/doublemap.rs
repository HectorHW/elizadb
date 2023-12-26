use std::{
    borrow::Borrow,
    collections::{HashMap, HashSet},
};

pub struct DoubleMap<K, V> {
    forward: HashMap<K, V>,
    backward: HashMap<V, K>,
}

impl<K, V> Default for DoubleMap<K, V> {
    fn default() -> Self {
        Self {
            forward: Default::default(),
            backward: Default::default(),
        }
    }
}

impl<K, V> DoubleMap<K, V>
where
    K: std::hash::Hash + Eq + Clone,
    V: std::hash::Hash + Eq + Clone,
{
    pub fn new() -> Self {
        Default::default()
    }

    pub fn insert(&mut self, first: K, second: V) {
        self.forward.insert(first.clone(), second.clone());
        self.backward.insert(second, first);
    }

    pub fn get_forward<Q>(&self, key: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
        Q: std::hash::Hash + std::cmp::Eq + ?Sized,
    {
        self.forward.get(key)
    }

    pub fn get_backward<Q>(&self, key: &Q) -> Option<&K>
    where
        V: Borrow<Q>,
        Q: std::hash::Hash + std::cmp::Eq + ?Sized,
    {
        self.backward.get(key)
    }

    pub fn len(&self) -> usize {
        self.forward.len()
    }
}

impl<K, V> TryFrom<HashMap<K, V>> for DoubleMap<K, V>
where
    K: std::hash::Hash + std::cmp::Eq + Clone,
    V: std::hash::Hash + std::cmp::Eq + Clone,
{
    type Error = ();

    fn try_from(value: HashMap<K, V>) -> Result<Self, Self::Error> {
        let mut existing_backward = HashSet::new();
        let mut result = DoubleMap::new();
        for (k, v) in value {
            if !existing_backward.insert(v.clone()) {
                return Err(());
            }
            result.insert(k, v);
        }

        Ok(result)
    }
}
