use std::usize;

use serde::{Deserialize, Serialize};
use serde_big_array::BigArray;

#[derive(Clone, Copy, Debug)]
#[repr(transparent)]
pub(super) struct SmallsetItem(u8);

impl From<SmallsetItem> for u8 {
    fn from(val: SmallsetItem) -> Self {
        val.0
    }
}

impl TryFrom<u8> for SmallsetItem {
    type Error = u8;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            EMPTY_SLOT | TOMBSTONE => Err(value),
            _any_other => Ok(SmallsetItem(value)),
        }
    }
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub struct Smallset<const SIZE: usize> {
    #[serde(with = "BigArray")]
    backing_storage: [u8; SIZE],
}

pub const EMPTY_SLOT: u8 = 0;
pub const TOMBSTONE: u8 = 0xff;

impl<const SIZE: usize> Smallset<SIZE> {
    /// Construct a new set without any elements
    pub fn new_empty() -> Self {
        let backing_storage = [EMPTY_SLOT; SIZE];
        Smallset { backing_storage }
    }

    /// Construct a set from existing storage. Storage is not changed in any way and MUST come from Smallset
    pub fn reiterpret(backing_storage: [u8; SIZE]) -> Self {
        Smallset { backing_storage }
    }

    fn hash(data: u8) -> usize {
        data as usize % SIZE
    }

    fn probe(previous_index: usize) -> usize {
        (previous_index + 1) % SIZE
    }

    /// Check if this value is stored in the set
    pub fn contains(&self, data: SmallsetItem) -> bool {
        let data = data.into();
        let hashcode = Self::hash(data);
        let mut look_position = hashcode;
        let mut attempt = 0;
        while attempt < SIZE {
            let value_in_slot = self.backing_storage[look_position];
            if value_in_slot == data {
                return true;
            }
            if value_in_slot == EMPTY_SLOT {
                return false;
            }

            look_position = Self::probe(look_position);
            attempt += 1;
        }
        false
    }

    /// Slot where this value could be written, None if map is full. Slot may contain value, contain tombstone or be empty
    fn locate_slot_mut(&mut self, data: u8) -> Option<(&mut u8, usize)> {
        let hashcode = Self::hash(data);
        let mut look_position = hashcode;
        let mut attempt = 0;
        while attempt < SIZE {
            let value_in_slot = self.backing_storage[look_position];
            if value_in_slot == data || value_in_slot == EMPTY_SLOT || value_in_slot == TOMBSTONE {
                return Some((&mut self.backing_storage[look_position], look_position));
            }

            look_position = Self::probe(look_position);
            attempt += 1;
        }

        None
    }

    /// Slot where this value could be written, None if map is full. Slot may contain value, contain tombstone or be empty
    fn locate_insertion_slot(&self, data: u8) -> Option<(&u8, usize)> {
        let hashcode = Self::hash(data);
        let mut look_position = hashcode;
        let mut attempt = 0;
        while attempt < SIZE {
            let value_in_slot = self.backing_storage[look_position];
            if value_in_slot == data || value_in_slot == EMPTY_SLOT || value_in_slot == TOMBSTONE {
                return Some((&self.backing_storage[look_position], look_position));
            }

            look_position = Self::probe(look_position);
            attempt += 1;
        }

        None
    }

    /// Insert this value into set and return bool indicating if it is new or error if set is full
    pub fn insert(&mut self, data: SmallsetItem) -> Result<bool, u8> {
        let data = data.into();
        let (slot, _) = self.locate_slot_mut(data).ok_or(data)?;
        if *slot == data {
            return Ok(false);
        }
        *slot = data;
        Ok(true)
    }

    /// Remove value from set, returning bool if it was here
    pub fn remove(&mut self, data: SmallsetItem) -> bool {
        let data = data.into();
        let Some((slot, index)) = self.locate_insertion_slot(data) else {
            return false;
        };
        if *slot == EMPTY_SLOT || *slot == TOMBSTONE {
            return false;
        }
        let next_position = Self::probe(index);
        self.backing_storage[index] = if self.backing_storage[next_position] == EMPTY_SLOT {
            EMPTY_SLOT
        } else {
            TOMBSTONE
        };
        true
    }

    /// Load factor computed as occupied / capacity
    pub fn load_factor(&self) -> f32 {
        let occupied_slots = self.size();

        occupied_slots as f32 / SIZE as f32
    }

    /// Number of elements stored in this set
    pub fn size(&self) -> usize {
        self.backing_storage
            .iter()
            .copied()
            .filter(|&item| item != EMPTY_SLOT && item != TOMBSTONE)
            .count()
    }

    /// Number of elements this set can store
    pub fn capacity(&self) -> usize {
        SIZE
    }

    /// Iterator over elements of the set
    pub fn iter(&self) -> impl Iterator<Item = u8> + '_ {
        self.backing_storage
            .iter()
            .cloned()
            .filter(|&item| item != EMPTY_SLOT && item != TOMBSTONE)
    }

    /// Clone self into compatible set, getting rid of any tombstones in the process
    pub fn compact<const OTHERSIZE: usize>(&self, target: &mut Smallset<OTHERSIZE>) {
        target.backing_storage.fill(EMPTY_SLOT);
        for item in self.iter() {
            target.insert(item.try_into().unwrap()).unwrap();
        }
    }

    pub fn clear(&mut self) {
        self.backing_storage.fill(EMPTY_SLOT);
    }
}

#[cfg(test)]
mod tests {
    use super::Smallset;

    type Small8 = Smallset<8>;

    macro_rules! item {
        ($x: expr) => {
            $x.try_into().unwrap()
        };
    }

    #[test]
    fn cannot_locate_item_in_empty_set() {
        let set = Small8::new_empty();
        assert!(!set.contains(item!(2)))
    }

    #[test]
    fn can_locate_item_after_insertion() {
        let mut set = Small8::new_empty();
        set.insert(item!(2)).unwrap();
        assert!(set.contains(item!(2)));
    }

    #[test]
    fn can_remove() {
        let mut set = Small8::new_empty();
        let item = item!(2);
        set.insert(item).unwrap();
        assert!(set.remove(item));
        assert!(!set.contains(item));
    }

    #[test]
    fn collisions_are_resolved() {
        let mut set = Small8::new_empty();
        set.insert(item!(2)).unwrap();
        set.insert(item!(10)).unwrap();

        assert!(set.contains(item!(2)));
        assert!(set.contains(item!(10)));
    }

    #[test]
    fn tombstones_are_placed_and_items_are_found_after_deletion() {
        let mut set = Small8::new_empty();
        set.insert(item!(2)).unwrap();
        set.insert(item!(10)).unwrap();
        set.remove(item!(2));

        assert!(!set.contains(item!(2)));
        assert!(set.contains(item!(10)));
    }
}
