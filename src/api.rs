use std::sync::{Arc, Mutex};

use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post, Router},
    Json,
};
use serde::Serialize;

use crate::{
    query::Query,
    storage::{Database, Key},
};

type DBState = Arc<Mutex<Database<8>>>;

pub fn build_router(state: DBState) -> axum::Router {
    Router::new()
        .route("/terms", get(list_terms).post(create_term))
        .route("/items", get(list_items).post(create_item))
        .route(
            "/items/:key",
            get(make_horizontal_query).post(add_term_to_key),
        )
        .route("/query", post(make_vertical_query))
        .with_state(state)
}

async fn create_term(
    State(db): State<DBState>,
    term: Json<String>,
) -> Result<(StatusCode, Json<impl Serialize>), (StatusCode, Json<impl Serialize>)> {
    let mut db = db.lock().unwrap();

    if db.get_term_id(&term).is_some() {
        return Err((StatusCode::CONFLICT, Json("term already exists")));
    }

    match db.add_term(&term) {
        Ok(new_index) => Ok((StatusCode::CREATED, Json(new_index))),
        Err(_) => Err((StatusCode::BAD_REQUEST, Json("term database is full"))),
    }
}

async fn list_terms(State(db): State<DBState>) -> Json<Vec<String>> {
    let db = db.lock().unwrap();
    Json(db.terms.lefts().cloned().collect())
}

async fn create_item(State(db): State<DBState>, Json(key): Json<Key>) -> StatusCode {
    let mut db = db.lock().unwrap();
    if db.create_record(key) {
        StatusCode::CREATED
    } else {
        StatusCode::CONFLICT
    }
}

async fn list_items(State(db): State<DBState>) -> Json<Vec<Key>> {
    let db = db.lock().unwrap();

    Json(db.list_keys().collect())
}

async fn add_term_to_key(
    State(db): State<DBState>,
    Path(key): Path<Key>,
    Json(term): Json<String>,
) -> Result<StatusCode, (StatusCode, Json<&'static str>)> {
    let mut db = db.lock().unwrap();

    match db.set_flag(key, &term) {
        Ok(_) => Ok(StatusCode::CREATED),
        Err(_) => Err((
            StatusCode::CONFLICT,
            Json("term database is full and cannot take more terms"),
        )),
    }
}

async fn make_horizontal_query(
    State(db): State<DBState>,
    Path(key): Path<Key>,
) -> Result<(StatusCode, Json<Vec<String>>), (StatusCode, Json<&'static str>)> {
    let db = db.lock().unwrap();
    match db.horizontal_query(&key) {
        Some(items) => Ok((
            StatusCode::OK,
            Json(items.into_iter().map(String::from).collect()),
        )),
        None => Err((StatusCode::NOT_FOUND, Json("key does not exist"))),
    }
}

async fn make_vertical_query(
    State(db): State<DBState>,
    Json(query): Json<Query>,
) -> (StatusCode, Result<Json<Vec<Key>>, Json<String>>) {
    let db = db.lock().unwrap();
    match db.vertical_query(&query) {
        Ok(items) => (StatusCode::OK, Ok(Json(items))),
        Err(message) => (StatusCode::BAD_REQUEST, Err(Json(message))),
    }
}
