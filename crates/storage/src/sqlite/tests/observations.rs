use chrono::NaiveDate;

use crate::sqlite::tests::in_memory_store;
use crate::sqlite::FRED_DATASET_ID;

#[tokio::test]
async fn sqlite_store_round_trips_seeded_observations() {
    let store = in_memory_store().await;
    store.seed_fred_metadata().await.unwrap();

    let indicators = store.load_indicators().await.unwrap();
    assert!(indicators.len() >= 10);

    let indicator = indicators
        .iter()
        .find(|indicator| indicator.indicator_id == "us_market_vix_close")
        .unwrap()
        .clone();
    let observation = fc_domain::Observation {
        indicator_id: indicator.indicator_id,
        entity_id: "us".to_string(),
        as_of_date: NaiveDate::from_ymd_opt(2020, 3, 16).unwrap(),
        period_start: Some(NaiveDate::from_ymd_opt(2020, 3, 16).unwrap()),
        period_end: Some(NaiveDate::from_ymd_opt(2020, 3, 16).unwrap()),
        frequency: indicator.frequency,
        value: 82.69,
        unit: indicator.unit,
        source_id: "fred".to_string(),
        dataset_id: FRED_DATASET_ID.to_string(),
        revision_time: None,
        publication_time: None,
        quality_score: 95.0,
        quality_flags: Vec::new(),
    };
    store.insert_observations(&[observation]).await.unwrap();
    let observations = store
        .load_observations("us", NaiveDate::from_ymd_opt(2020, 3, 17).unwrap())
        .await
        .unwrap();

    assert_eq!(observations.len(), 1);
    assert_eq!(observations[0].value, 82.69);
}
