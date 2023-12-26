use std::collections::HashSet;

use super::{Database, Key};

impl<const SMALLSIZE: usize> Database<SMALLSIZE> {
    pub fn explain_term_id(&self, term_id: u8) -> Option<&'_ str> {
        self.terms.get_backward(&term_id).map(|entry| entry.as_str())
    }

    pub fn horizontal_query(&self, key: &Key) -> Option<HashSet<&'_ str>> {
        let location = self.index.get(key)?;
        match location {
            &super::storage::IndexLocation::Small(location) => {
                let set = self.get_view(location)?;
                let items = set.iter().cloned().collect::<Vec<_>>();
                Some(items.into_iter().filter_map(|item| {
                    self.explain_term_id(item)
                }).collect())
            }
            super::storage::IndexLocation::Big => {
                Some(self.big_storage.get(key)?.iter().cloned().filter_map(|item| self.explain_term_id(item)).collect())
            }
        }
    }
}