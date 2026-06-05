use super::*;

#[test]
fn release_review_backtest_comparison_marks_lost_timely_warning() {
    let baseline = vec![
        synthetic_backtest_summary("scenario_a", "Scenario A", Some(20), Some(14), 0),
        synthetic_backtest_summary("scenario_b", "Scenario B", Some(9), None, 1),
    ];
    let candidate = vec![
        synthetic_backtest_summary("scenario_a", "Scenario A", Some(18), None, 2),
        synthetic_backtest_summary("scenario_b", "Scenario B", Some(9), Some(5), 1),
    ];

    let rows = build_release_review_backtest_scenario_comparisons(&baseline, &candidate);

    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].scenario_id, "scenario_a");
    assert_eq!(rows[0].outcome, "timely_to_missed");
    assert_eq!(rows[0].actionable_delta_days, None);
    assert_eq!(rows[1].scenario_id, "scenario_b");
    assert_eq!(rows[1].outcome, "missed_to_late_only");
}
