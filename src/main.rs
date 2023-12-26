use std::sync::{Arc, Mutex};

use storage::{Database, Key};

mod api;
mod doublemap;
mod query;
mod serde;
mod smallset;
mod storage;

#[tokio::main]
async fn main() {
    let database = Arc::new(Mutex::new(Database::<8>::default()));
    let router = api::build_router(database);
    let bind_string = "0.0.0.0:4200";
    println!("{}", bind_string);
    let listener = tokio::net::TcpListener::bind(bind_string).await.unwrap();
    axum::serve(listener, router).await.unwrap();
}
