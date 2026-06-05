use super::*;

#[test]
fn best_effort_visibility_uses_release_rule_not_backfill_fetch_time_for_fred() {
    let observation = observation(
        "fred",
        Frequency::Monthly,
        NaiveDate::from_ymd_opt(2020, 1, 31).unwrap(),
        Some(Utc.with_ymd_and_hms(2026, 5, 31, 0, 0, 0).single().unwrap()),
    );

    assert!(!observation_is_visible_for_date(
        &observation,
        NaiveDate::from_ymd_opt(2020, 2, 14).unwrap(),
        PointInTimeMode::BestEffort
    ));
    assert!(observation_is_visible_for_date(
        &observation,
        NaiveDate::from_ymd_opt(2020, 2, 15).unwrap(),
        PointInTimeMode::BestEffort
    ));
}

#[test]
fn strict_visibility_requires_timestamp_to_arrive_before_cutoff() {
    let observation = observation(
        "sec_edgar",
        Frequency::Daily,
        NaiveDate::from_ymd_opt(2020, 1, 2).unwrap(),
        Some(Utc.with_ymd_and_hms(2020, 1, 2, 23, 0, 0).single().unwrap()),
    );

    assert!(!observation_is_visible_for_date(
        &observation,
        NaiveDate::from_ymd_opt(2020, 1, 2).unwrap(),
        PointInTimeMode::Strict
    ));
    assert!(observation_is_visible_for_date(
        &observation,
        NaiveDate::from_ymd_opt(2020, 1, 3).unwrap(),
        PointInTimeMode::Strict
    ));
}
