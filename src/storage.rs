use super::doublemap::DoubleMap;
use crate::smallset::Smallset;
use std::{
    collections::{HashMap, HashSet, VecDeque},
    num::NonZeroU64,
};

pub type Key = NonZeroU64;

const _: () = {
    assert!(
        std::mem::size_of::<Option<Key>>() == std::mem::size_of::<Key>(),
        "key must have a nieche"
    );
};

pub(super) enum IndexLocation {
    /// Offset in number of elements (must be multiplied by size if offsetting into bytes)
    Small(usize),
    Big,
}

#[derive(Default)]
pub struct Database<const SMALLSIZE: usize> {
    pub(super) terms: DoubleMap<String, u8>,
    pub(super) index: HashMap<Key, IndexLocation>,
    pub(super) holes: VecDeque<usize>,
    pub(super) small_keys: Vec<Option<Key>>,
    pub(super) small_storage: Vec<Smallset<SMALLSIZE>>,
    pub(super) big_storage: HashMap<Key, HashSet<u8>>,
}

impl<const SMALLSIZE: usize> Database<SMALLSIZE> {
    pub(super) fn get_smallset(&self, index: usize) -> Option<&Smallset<SMALLSIZE>> {
        self.small_storage.get(index)
    }

    pub(super) fn get_smallset_mut(&mut self, index: usize) -> Option<&mut Smallset<SMALLSIZE>> {
        self.small_storage.get_mut(index)
    }

    /// Creates new key, indicates if it was inserted
    pub fn create_record(&mut self, key: Key) -> bool {
        if self.index.contains_key(&key) {
            return false;
        }

        if let Some(hole) = self.holes.pop_back() {
            self.index.insert(key, IndexLocation::Small(hole));
            self.get_smallset_mut(hole).unwrap().clear();
            self.small_keys[hole] = Some(key);
        } else {
            self.index
                .insert(key, IndexLocation::Small(self.small_storage.len()));
            self.small_keys.push(Some(key));
            self.small_storage.push(Smallset::new_empty())
        }

        true
    }

    pub fn get_term_id(&self, term: &str) -> Option<u8> {
        self.terms.get_forward(term).cloned()
    }

    /// Tries to add Term, fails if it exeeds u8 capacity
    pub fn add_term(&mut self, term: &str) -> Result<u8, ()> {
        if let Some(loc) = self.terms.get_forward(term) {
            return Ok(*loc);
        }
        if self.terms.len() == 253 {
            return Err(());
        }
        let new_index = 1 + self.terms.len();
        self.terms.insert(term.to_string(), new_index as u8);
        Ok(*self.terms.get_forward(term).unwrap())
    }

    pub fn list_keys(&self) -> impl Iterator<Item = Key> + '_ {
        self.big_storage
            .keys()
            .cloned()
            .chain(self.small_keys.iter().filter_map(|item| *item))
    }

    /// Add boolean flag to key
    pub fn set_flag(&mut self, key: Key, term: &str) -> Result<bool, ()> {
        let term_index = self.add_term(term)?;
        self.create_record(key);

        match self.index.get(&key).unwrap() {
            &IndexLocation::Small(index) => {
                let small_record = self.get_smallset_mut(index).unwrap();
                match small_record.insert(term_index) {
                    Ok(exists) => Ok(exists),
                    Err(_) => {
                        self.evict_into_large(key);
                        self.set_flag(key, term)
                    }
                }
            }
            IndexLocation::Big => Ok(self.big_storage.entry(key).or_default().insert(term_index)),
        }
    }

    fn evict_into_large(&mut self, key: Key) {
        let small_index = match self.index.entry(key).or_insert(IndexLocation::Big) {
            IndexLocation::Small(value) => *value,
            IndexLocation::Big => return,
        };

        let current_state = *self.get_smallset(small_index).unwrap();

        let big_set = self.big_storage.entry(key).or_default();
        for item in current_state.iter() {
            big_set.insert(item);
        }
        self.index.insert(key, IndexLocation::Big);
        self.holes.push_back(small_index);
        self.small_keys[small_index] = None;
    }
}
