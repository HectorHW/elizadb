use std::collections::HashSet;

use serde::Deserialize;

use super::{Database, Key};

#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "type")]
pub enum Query {
    Simple { term: String },
    KofN { terms: Vec<String>, bound: usize },
}

impl<const SMALLSIZE: usize> Database<SMALLSIZE> {
    pub fn explain_term_id(&self, term_id: u8) -> Option<&'_ str> {
        self.terms
            .get_backward(&term_id)
            .map(|entry| entry.as_str())
    }

    pub fn horizontal_query(&self, key: &Key) -> Option<HashSet<&'_ str>> {
        let location = self.index.get(key)?;
        match location {
            &super::storage::IndexLocation::Small(location) => {
                let set = *self.get_smallset(location)?;
                Some(
                    set.iter()
                        .filter_map(|item| self.explain_term_id(item))
                        .collect(),
                )
            }
            super::storage::IndexLocation::Big => Some(
                self.big_storage
                    .get(key)?
                    .iter()
                    .cloned()
                    .filter_map(|item| self.explain_term_id(item))
                    .collect(),
            ),
        }
    }

    pub fn vertical_query(&self, query: &Query) -> Result<Vec<Key>, String> {
        match query {
            Query::Simple { term } => {
                let Some(term_id) = self.get_term_id(term) else {
                    return Err(format!("unknown term {}", term));
                };
                Ok(self.simple_vertical_query(term_id))
            }
            Query::KofN { terms, bound } => {
                let resolved_terms = terms
                    .iter()
                    .map(|term| self.get_term_id(term).ok_or(term))
                    .collect::<Result<Vec<u8>, &String>>()?;
                Ok(self.k_of_n_query(&resolved_terms, *bound))
            }
        }
    }

    fn simple_vertical_query(&self, term_id: u8) -> Vec<Key> {
        self.small_keys
            .iter()
            .zip(self.small_storage.iter())
            .filter_map(|(key, set)| {
                let &Some(key) = key else {
                    return None;
                };
                if set.contains(term_id) {
                    Some(key)
                } else {
                    None
                }
            })
            .chain(self.big_storage.iter().filter_map(|(&key, set)| {
                if set.contains(&term_id) {
                    Some(key)
                } else {
                    None
                }
            }))
            .collect()
    }

    fn k_of_n_query(&self, terms: &[u8], bound: usize) -> Vec<Key> {
        self.small_keys
            .iter()
            .zip(self.small_storage.iter())
            .filter_map(|(key, set)| {
                let &Some(key) = key else {
                    return None;
                };
                let mut total = 0;
                for &item in terms {
                    if set.contains(item) {
                        total += 1;
                    }
                    if total >= bound {
                        return Some(key);
                    }
                }
                None
            })
            .chain(self.big_storage.iter().filter_map(|(&key, set)| {
                let mut total = 0;
                for item in terms {
                    if set.contains(item) {
                        total += 1;
                    }
                    if total >= bound {
                        return Some(key);
                    }
                }
                None
            }))
            .collect()
    }
}
