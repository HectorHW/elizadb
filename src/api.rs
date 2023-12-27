use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post, Router},
    Json,
};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::{
    query::Query,
    storage::{Database, Key},
};

type DBState = Arc<RwLock<Database<8>>>;

pub fn build_router(state: DBState) -> axum::Router {
    Router::new()
        .route("/terms", get(list_terms).post(create_term))
        .route("/items", get(list_items).post(create_item))
        .route(
            "/items/:key",
            get(make_horizontal_query).post(add_term_to_key),
        )
        .route("/query", post(make_vertical_query))
        .route("/bulk/items", post(allocate_items_bulk))
        .route("/bulk/keys", post(set_keys_bulk))
        .with_state(state)
}

async fn create_term(
    State(db): State<DBState>,
    term: Json<String>,
) -> Result<(StatusCode, Json<impl Serialize>), (StatusCode, Json<impl Serialize>)> {
    let mut db = db.write().await;

    if db.get_term_id(&term).is_some() {
        return Err((StatusCode::CONFLICT, Json("term already exists")));
    }

    match db.add_term(&term) {
        Ok(new_index) => Ok((StatusCode::CREATED, Json(new_index))),
        Err(_) => Err((StatusCode::BAD_REQUEST, Json("term database is full"))),
    }
}

async fn list_terms(State(db): State<DBState>) -> Json<Vec<String>> {
    let db = db.read().await;
    Json(db.terms.lefts().cloned().collect())
}

async fn create_item(State(db): State<DBState>, Json(key): Json<Key>) -> StatusCode {
    let mut db = db.write().await;
    if db.create_record(key) {
        StatusCode::CREATED
    } else {
        StatusCode::CONFLICT
    }
}

async fn allocate_items_bulk(
    State(db): State<DBState>,
    Json(items): Json<Vec<Key>>,
) -> Result<StatusCode, (StatusCode, Json<Vec<Key>>)> {
    let mut db = db.write().await;
    let mut existing_keys = vec![];
    for item in items {
        if !db.create_record(item) {
            existing_keys.push(item);
        }
    }
    if existing_keys.is_empty() {
        Ok(StatusCode::CREATED)
    } else {
        Err((StatusCode::CONFLICT, Json(existing_keys)))
    }
}

async fn list_items(State(db): State<DBState>) -> Json<Vec<Key>> {
    let db = db.read().await;

    Json(db.list_keys().collect())
}

async fn add_term_to_key(
    State(db): State<DBState>,
    Path(key): Path<Key>,
    Json(term): Json<String>,
) -> Result<StatusCode, (StatusCode, Json<&'static str>)> {
    let mut db = db.write().await;

    match db.set_flag(key, &term) {
        Ok(_) => Ok(StatusCode::CREATED),
        Err(_) => Err((
            StatusCode::CONFLICT,
            Json("term database is full and cannot take more terms"),
        )),
    }
}

#[derive(Clone, Debug, Deserialize)]
struct SetKeysBulk {
    term: String,
    keys: Vec<Key>,
}

async fn set_keys_bulk(State(db): State<DBState>, Json(request): Json<SetKeysBulk>) -> StatusCode {
    let mut db = db.write().await;
    if db.add_term(&request.term).is_err() {
        return StatusCode::CONFLICT;
    }
    for key in request.keys {
        db.set_flag(key, &request.term).unwrap();
    }

    StatusCode::OK
}

async fn make_horizontal_query(
    State(db): State<DBState>,
    Path(key): Path<Key>,
) -> Result<(StatusCode, Json<Vec<String>>), (StatusCode, Json<&'static str>)> {
    let db = db.read().await;
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
    let db = db.read().await;
    match db.vertical_query(&query) {
        Ok(items) => (StatusCode::OK, Ok(Json(items))),
        Err(message) => (StatusCode::BAD_REQUEST, Err(Json(message))),
    }
}
