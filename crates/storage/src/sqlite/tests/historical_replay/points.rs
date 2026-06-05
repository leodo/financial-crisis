use chrono::Utc;

use crate::sqlite::tests::historical_replay::fixtures::{
    assessment_point, model_release, replay_run,
};
use crate::sqlite::tests::in_memory_store;

#[tokio::test]
async fn sqlite_store_round_trips_historical_assessment_points() {
    let store = in_memory_store().await;
    let created_at = Utc::now();
    let release = model_release(created_at);
    store.upsert_model_release(&release).await.unwrap();

    let run = replay_run(created_at);
    store.upsert_historical_replay_run(&run).await.unwrap();

    let point = assessment_point(created_at, &run.replay_run_id);
    store
        .replace_historical_assessment_points(&run.replay_run_id, &[point])
        .await
        .unwrap();

    let points = store
        .list_historical_assessment_points(
            Some("replay-1"),
            Some("financial_system"),
            Some("release-1"),
            Some(run.from_date),
            Some(chrono::NaiveDate::from_ymd_opt(2026, 5, 31).unwrap()),
            Some(10),
        )
        .await
        .unwrap();
    assert_eq!(points.len(), 1);
    assert_eq!(points[0].posture, "prepare");
    assert_eq!(points[0].actionability_prepare, 0.61);
    assert_eq!(
        points[0].posture_trigger_codes,
        vec!["prepare_p60d_structural".to_string()]
    );
    assert_eq!(
        points[0].probability_diagnostics.horizon_overlays[0].final_probability,
        0.21
    );
    assert_eq!(
        points[0].probability_diagnostics.horizon_overlays[0].contributions[0].family_id,
        "jpy_carry"
    );
}
