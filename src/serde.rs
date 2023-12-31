use std::{
    collections::{HashMap, HashSet},
    io::{BufReader, BufWriter, Read, Write},
};

use serde::{Deserialize, Serialize};

use crate::{
    doublemap::DoubleMap,
    smallset::Smallset,
    storage::{Database, IndexLocation, Key},
};

impl<const SMALLSIZE: usize> Database<SMALLSIZE> {
    fn from_existing_data(
        terms: HashMap<String, u8>,
        small_keys: Vec<Key>,
        small_storage: Vec<Smallset<SMALLSIZE>>,
        big_storage: HashMap<Key, HashSet<u8>>,
    ) -> Self {
        let index = Self::build_index(small_keys.as_ref(), &big_storage);

        Self {
            terms: DoubleMap::try_from(terms).unwrap(),
            index,
            holes: Default::default(),
            small_keys: small_keys.into_iter().map(Option::Some).collect(),
            small_storage,
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

    fn compact_small_items(&self) -> (Vec<Key>, Vec<Smallset<SMALLSIZE>>) {
        let (mut keys, mut values) = (
            Vec::with_capacity(self.small_keys.len()),
            Vec::with_capacity(self.small_keys.len()),
        );

        for (&key, &value) in self.small_keys.iter().zip(self.small_storage.iter()) {
            let Some(key) = key else {
                continue;
            };
            keys.push(key);
            values.push(value);
        }

        (keys, values)
    }

    fn compact_terms(&self) -> Vec<String> {
        let mut items = self.terms.left_items().collect::<Vec<_>>();
        items.sort_unstable_by_key(|(_, &idx)| idx);
        items
            .into_iter()
            .map(|(term, _idx)| term)
            .cloned()
            .collect()
    }

    pub fn dump(&self, buffer: &mut impl Write) -> Result<(), rmp_serde::encode::Error> {
        let (small_keys, small_storage) = self.compact_small_items();

        let terms = self.compact_terms();

        let serde = SerializationScheme {
            terms,
            small_keys,
            small_storage,
            big_storage: self.big_storage.clone(),
        };

        rmp_serde::encode::write(buffer, &serde)
    }

    pub fn load(buffer: &mut impl Read) -> Result<Self, rmp_serde::decode::Error> {
        let serde: SerializationScheme<SMALLSIZE> = rmp_serde::decode::from_read(buffer)?;

        let terms = serde
            .terms
            .into_iter()
            .enumerate()
            // v + 1 because 0 is used as nieche for NO_VALUE
            .map(|(v, k)| (k, (v + 1) as u8))
            .collect();

        Ok(Self::from_existing_data(
            terms,
            serde.small_keys,
            serde.small_storage,
            serde.big_storage,
        ))
    }
}

pub fn two_phase_save<const SMALLSIZE: usize>(
    state: &Database<SMALLSIZE>,
    save_path: impl AsRef<std::path::Path>,
) -> Result<(), Box<dyn std::error::Error>> {
    let final_path = save_path.as_ref();
    let temp_path = get_temp_filename(final_path);

    write_to_file(state, &temp_path)?;

    std::fs::rename(temp_path, final_path)?;

    Ok(())
}

fn get_temp_filename(save_path: &std::path::Path) -> std::path::PathBuf {
    let mut temp_filename = save_path.to_owned();
    add_extension(&mut temp_filename, "tmp");
    temp_filename
}

fn write_to_file<const SMALLSIZE: usize>(
    state: &Database<SMALLSIZE>,
    path: impl AsRef<std::path::Path>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut temp_file = BufWriter::new(std::fs::File::create(path.as_ref())?);
    state.dump(&mut temp_file)?;
    Ok(())
}

fn add_extension(path: &mut std::path::PathBuf, extension: impl AsRef<std::path::Path>) {
    match path.extension() {
        Some(ext) => {
            let mut ext = ext.to_os_string();
            ext.push(".");
            ext.push(extension.as_ref());
            path.set_extension(ext)
        }
        None => path.set_extension(extension.as_ref()),
    };
}

pub static DEFAULT_SAVE_PATH: &str = "state.elizadb";

pub fn load_possibly_missing<const SMALLSIZE: usize>(
    path: impl AsRef<std::path::Path>,
) -> Result<Database<SMALLSIZE>, Box<dyn std::error::Error>> {
    let path = path.as_ref();
    if !path.exists() {
        return Ok(Default::default());
    }
    let mut input_file = BufReader::new(std::fs::File::open(path)?);

    let state = Database::<SMALLSIZE>::load(&mut input_file)?;
    Ok(state)
}

#[derive(Serialize, Deserialize)]
struct SerializationScheme<const SMALLSIZE: usize> {
    terms: Vec<String>,
    small_keys: Vec<Key>,
    small_storage: Vec<Smallset<SMALLSIZE>>,
    big_storage: HashMap<Key, HashSet<u8>>,
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use crate::storage::{Database, Key};

    #[test]
    fn state_is_stored_and_loaded() {
        let mut db = Database::<8>::default();

        let key = Key::try_from(1).unwrap();

        db.add_term("term").unwrap();
        db.add_term("term2").unwrap();
        db.create_record(key);
        db.set_flag(key, "term").unwrap();

        let mut storage = vec![];
        db.dump(&mut storage).unwrap();

        let mut reader = storage.as_slice();

        let db = Database::<8>::load(&mut reader).unwrap();

        assert_eq!(
            db.horizontal_query(&key),
            Some({
                let mut set = HashSet::new();
                set.insert("term");
                set
            })
        )
    }
}
