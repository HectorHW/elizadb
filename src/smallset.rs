use std::usize;

#[derive(Debug)]
pub struct Smallset<'data, const SIZE: usize> {
    backing_storage: &'data mut [u8; SIZE]
}

pub const EMPTY_SLOT: u8 = 0;
pub const TOMBSTONE: u8 = 0xff;

impl<'data, const SIZE: usize> Smallset<'data, SIZE> {
    /// Construct a set over backing storage, clearing it in the process
    pub fn new_empty(backing_storage: &'data mut [u8; SIZE]) -> Self {
        backing_storage.fill(EMPTY_SLOT);
        Smallset{backing_storage}
    }

    /// Construct a set from existing storage. Storage is not changed in any way and MUST come from Smallset
    pub fn reiterpret(backing_storage: &'data mut [u8; SIZE]) -> Self {
        Smallset{backing_storage}
    }



    fn hash(data: u8) -> usize {
        data as usize % SIZE
    }

    fn probe(previous_index: usize) -> usize {
        (previous_index + 1) % SIZE
    }

    /// Check if this value is stored in the set
    pub fn contains(&self, data: u8) -> bool {
        let Some((slot, _)) = self.locate_slot(data) else {
            return false;
        };
        *slot == data
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
    fn locate_slot(&self, data: u8) -> Option<(&u8, usize)> {
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
    pub fn insert(&mut self, data: u8) -> Result<bool, u8> {
        assert!(data != EMPTY_SLOT && data != TOMBSTONE);
        let (slot, _) = self.locate_slot_mut(data).ok_or(data)?;
        if *slot == data {
            return Ok(false);
        }
        *slot = data;
        Ok(true)
    }

    /// Remove value from set, returning bool if it was here
    pub fn remove(&mut self, data: u8) -> bool {
        assert!(data != EMPTY_SLOT && data != TOMBSTONE);
        let Some((slot, index)) = self.locate_slot(data) else {
            return false;
        };
        if *slot == EMPTY_SLOT || *slot == TOMBSTONE {
            return false;
        }
        let next_position = Self::probe(index);
        self.backing_storage[index] =if self.backing_storage[next_position] == EMPTY_SLOT {
             EMPTY_SLOT
        }else{
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
        self.backing_storage.iter().copied().filter(|&item| item != EMPTY_SLOT && item !=TOMBSTONE).count()
    }

    /// Number of elements this set can store
    pub fn capacity(&self) -> usize {
        SIZE
    }

    /// Iterator over elements of the set
    pub fn iter(&self) -> impl Iterator<Item=u8> + '_ {
        self.backing_storage.iter().cloned().filter(|&item| item != EMPTY_SLOT && item != TOMBSTONE)
    }

    /// Clone self into compatible set, getting rid of any tombstones in the process
    pub fn compact<const OTHERSIZE: usize>(&self, target: &mut Smallset<'_, OTHERSIZE>) {
        target.backing_storage.fill(EMPTY_SLOT);
        for item in self.iter() {
            target.insert(item).unwrap();
        }
    }
}


#[cfg(test)]
mod tests{
    use super::Smallset;


    #[test]
    fn cannot_locate_item_in_empty_set() {
        let mut data = [0;8];
        let set = Smallset::new_empty(&mut data);
        assert!(!set.contains(2))
    }

    #[test]
    fn can_locate_item_after_insertion() {
        let mut data = [0;8];
        let mut set = Smallset::new_empty(&mut data);
        set.insert(2).unwrap();
        assert!(set.contains(2));
    }

    #[test]
    fn can_remove() {
        let mut data = [0;8];
        let mut set = Smallset::new_empty(&mut data);
        set.insert(2).unwrap();
        assert!(set.remove(2));
        assert!(!set.contains(2));
    }

    #[test]
    fn collisions_are_resolved() {

    }

}