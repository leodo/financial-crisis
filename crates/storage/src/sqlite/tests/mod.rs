use crate::SqliteStore;

mod formal_datasets;
mod historical_replay;
mod observations;
mod operational;
mod prediction_snapshots;
mod releases;

pub(super) async fn in_memory_store() -> SqliteStore {
    let store = SqliteStore::connect_url("sqlite::memory:").await.unwrap();
    store.migrate().await.unwrap();
    store
}
