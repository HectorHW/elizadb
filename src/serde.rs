use std::{
    collections::{HashMap, HashSet},
    marker::PhantomData,
};

use crate::{
    doublemap::DoubleMap,
    storage::{Database, IndexLocation, Key},
};

impl<const SMALLSIZE: usize> Database<SMALLSIZE> {
    pub fn new(
        terms: HashMap<String, u8>,
        small_keys: Vec<Key>,
        small_storage: Vec<u8>,
        big_storage: HashMap<Key, HashSet<u8>>,
    ) -> Self {
        let index = Self::build_index(small_keys.as_ref(), &big_storage);

        Self {
            terms: DoubleMap::try_from(terms).unwrap(),
            index,
            holes: Default::default(),
            small_keys: small_keys.into_iter().map(Option::Some).collect(),
            small_storage,
            storage_type: PhantomData,
            big_storage,
        }
    }

    fn build_index(
        small_keys: &[Key],
        big_storage: &HashMap<Key, HashSet<u8>>,
    ) -> HashMap<Key, IndexLocation> {
        let mut result = HashMap::new();
        for (i, key) in small_keys.iter().enumerate() {
            result.insert(*key, IndexLocation::Small(i));
        }
        for key in big_storage.keys() {
            result.insert(*key, IndexLocation::Big);
        }

        result
    }
}
