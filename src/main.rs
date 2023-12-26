use storage::{Database, Key};

mod storage;
mod serde;
mod smallset;
mod doublemap;
mod query;


fn main() {
    let mut database = Database::<8>::default();

    database.add_term("kms").unwrap();
    let key = Key::new(1).unwrap();
    database.create_record(key);

    database.set_flag(key, "kms").unwrap();

    println!("{:?}", database.horizontal_query(&key));
}
