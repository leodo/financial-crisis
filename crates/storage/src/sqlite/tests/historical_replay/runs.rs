use chrono::Utc;

use crate::sqlite::tests::historical_replay::fixtures::{model_release, replay_run};
use crate::sqlite::tests::in_memory_store;

#[tokio::test]
async fn sqlite_store_round_trips_historical_replay_run_lookup() {
    let store = in_memory_store().await;
    let created_at = Utc::now();
    let release = model_release(created_at);
    store.upsert_model_release(&release).await.unwrap();

    let run = replay_run(created_at);
    store.upsert_historical_replay_run(&run).await.unwrap();

    let loaded_run = store
        .load_latest_historical_replay_run(
            "financial_system",
            Some("release-1"),
            "history_cache_v3|release=release-1",
            run.from_date,
            run.to_date,
        )
        .await
        .unwrap()
        .unwrap();
    assert_eq!(loaded_run.replay_run_id, "replay-1");
    assert_eq!(loaded_run.point_count, 1);

    let runs = store
        .list_historical_replay_runs(
            Some("financial_system"),
            Some("release-1"),
            Some(run.from_date),
            Some(chrono::NaiveDate::from_ymd_opt(2026, 5, 31).unwrap()),
            Some(10),
        )
        .await
        .unwrap();
    assert_eq!(runs.len(), 1);
    assert_eq!(
        runs[0].history_cache_key,
        "history_cache_v3|release=release-1"
    );
}
