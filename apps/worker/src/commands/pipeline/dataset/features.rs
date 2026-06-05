use std::collections::BTreeMap;

use fc_domain::PredictionSnapshotRecord;

pub(crate) fn transitional_feature_names() -> Vec<String> {
    crate::TRANSITIONAL_PROBABILITY_BUNDLE_FEATURES
        .iter()
        .map(|feature| (*feature).to_string())
        .collect()
}

pub(crate) fn formal_feature_names() -> Vec<String> {
    crate::FORMAL_PROBABILITY_BUNDLE_FEATURES
        .iter()
        .map(|feature| (*feature).to_string())
        .collect()
}

pub(crate) fn pipeline_features_from_snapshot(
    snapshot: &PredictionSnapshotRecord,
) -> BTreeMap<String, f64> {
    BTreeMap::from([
        (
            crate::FEATURE_OVERALL_SCORE.to_string(),
            (snapshot.overall_score / 100.0).clamp(0.0, 1.0),
        ),
        (
            crate::FEATURE_EXTERNAL_SHOCK_SCORE.to_string(),
            (snapshot.external_shock_score / 100.0).clamp(0.0, 1.0),
        ),
        (
            crate::FEATURE_HEURISTIC_P_5D.to_string(),
            snapshot.raw_p_5d.clamp(0.0, 0.99),
        ),
        (
            crate::FEATURE_HEURISTIC_P_20D.to_string(),
            snapshot.raw_p_20d.clamp(0.0, 0.99),
        ),
        (
            crate::FEATURE_HEURISTIC_P_60D.to_string(),
            snapshot.raw_p_60d.clamp(0.0, 0.99),
        ),
        (
            crate::FEATURE_COVERAGE_SCORE.to_string(),
            snapshot.coverage_score.clamp(0.0, 1.0),
        ),
        (
            crate::FEATURE_BUCKET_MONTHS_OR_HIGHER.to_string(),
            matches!(
                snapshot.time_to_risk_bucket.as_str(),
                "months" | "weeks" | "now"
            ) as u8 as f64,
        ),
        (
            crate::FEATURE_BUCKET_WEEKS_OR_HIGHER.to_string(),
            matches!(snapshot.time_to_risk_bucket.as_str(), "weeks" | "now") as u8 as f64,
        ),
        (
            crate::FEATURE_BUCKET_NOW.to_string(),
            matches!(snapshot.time_to_risk_bucket.as_str(), "now") as u8 as f64,
        ),
        (
            crate::FEATURE_FRESHNESS_DELAYED_OR_WORSE.to_string(),
            matches!(
                snapshot.freshness_status.as_str(),
                "delayed" | "stale" | "missing"
            ) as u8 as f64,
        ),
        (
            crate::FEATURE_FRESHNESS_STALE_OR_MISSING.to_string(),
            matches!(snapshot.freshness_status.as_str(), "stale" | "missing") as u8 as f64,
        ),
    ])
}
