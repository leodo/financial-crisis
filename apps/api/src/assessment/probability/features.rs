use std::collections::BTreeMap;

use fc_domain::{
    formal_observation_feature_value, DataTrust, FreshnessStatus, JpyCarrySnapshot,
    KeyIndicatorStatus, Observation, RiskDimension, RiskSnapshot, TimeToRiskBucket,
    FEATURE_BUCKET_MONTHS_OR_HIGHER, FEATURE_BUCKET_NOW, FEATURE_BUCKET_WEEKS_OR_HIGHER,
    FEATURE_COVERAGE_SCORE, FEATURE_EXTERNAL_DIMENSION_SCORE, FEATURE_EXTERNAL_SHOCK_SCORE,
    FEATURE_FRESHNESS_DELAYED_OR_WORSE, FEATURE_FRESHNESS_STALE_OR_MISSING,
    FEATURE_HEURISTIC_P_20D, FEATURE_HEURISTIC_P_5D, FEATURE_HEURISTIC_P_60D,
    FEATURE_OVERALL_SCORE, FEATURE_STRUCTURAL_SCORE, FEATURE_TRIGGER_SCORE,
    FORMAL_OBSERVATION_FEATURE_SPECS,
};

use super::super::{
    build_time_to_risk_bucket, high_risk_breadth, round6, ProbabilityActionThresholds,
};
use fc_domain::ProbabilityBlock;

pub(super) fn build_probability_feature_map(
    snapshot: &RiskSnapshot,
    observations: &[Observation],
    external_shock_score: f64,
    data_trust: &DataTrust,
    jpy_carry: &JpyCarrySnapshot,
    heuristic_probabilities: &ProbabilityBlock,
    key_indicators: &[KeyIndicatorStatus],
) -> BTreeMap<String, f64> {
    let heuristic_bucket = build_time_to_risk_bucket(
        heuristic_probabilities,
        None,
        None,
        None,
        snapshot.overall_score,
        snapshot.structural_score,
        snapshot.trigger_score,
        external_shock_score,
        high_risk_breadth(snapshot),
        jpy_carry,
        ProbabilityActionThresholds::legacy(),
    );
    let freshness_status = worst_key_indicator_freshness(key_indicators);
    let mut features = BTreeMap::from([
        (
            FEATURE_OVERALL_SCORE.to_string(),
            (snapshot.overall_score / 100.0).clamp(0.0, 1.0),
        ),
        (
            FEATURE_EXTERNAL_SHOCK_SCORE.to_string(),
            (external_shock_score / 100.0).clamp(0.0, 1.0),
        ),
        (
            FEATURE_HEURISTIC_P_5D.to_string(),
            super::super::clamp_probability(heuristic_probabilities.p_5d),
        ),
        (
            FEATURE_HEURISTIC_P_20D.to_string(),
            super::super::clamp_probability(heuristic_probabilities.p_20d),
        ),
        (
            FEATURE_HEURISTIC_P_60D.to_string(),
            super::super::clamp_probability(heuristic_probabilities.p_60d),
        ),
        (
            FEATURE_COVERAGE_SCORE.to_string(),
            data_trust.coverage_score.clamp(0.0, 1.0),
        ),
        (
            FEATURE_BUCKET_MONTHS_OR_HIGHER.to_string(),
            matches!(
                heuristic_bucket,
                TimeToRiskBucket::Months | TimeToRiskBucket::Weeks | TimeToRiskBucket::Now
            ) as u8 as f64,
        ),
        (
            FEATURE_BUCKET_WEEKS_OR_HIGHER.to_string(),
            matches!(
                heuristic_bucket,
                TimeToRiskBucket::Weeks | TimeToRiskBucket::Now
            ) as u8 as f64,
        ),
        (
            FEATURE_BUCKET_NOW.to_string(),
            matches!(heuristic_bucket, TimeToRiskBucket::Now) as u8 as f64,
        ),
        (
            FEATURE_FRESHNESS_DELAYED_OR_WORSE.to_string(),
            matches!(
                freshness_status,
                FreshnessStatus::Delayed | FreshnessStatus::Stale | FreshnessStatus::Missing
            ) as u8 as f64,
        ),
        (
            FEATURE_FRESHNESS_STALE_OR_MISSING.to_string(),
            matches!(
                freshness_status,
                FreshnessStatus::Stale | FreshnessStatus::Missing
            ) as u8 as f64,
        ),
    ]);
    features.extend(build_formal_probability_feature_map(
        snapshot,
        observations,
        data_trust,
    ));
    features
}

fn build_formal_probability_feature_map(
    snapshot: &RiskSnapshot,
    observations: &[Observation],
    data_trust: &DataTrust,
) -> BTreeMap<String, f64> {
    let mut features = BTreeMap::from([
        (
            FEATURE_STRUCTURAL_SCORE.to_string(),
            round6((snapshot.structural_score / 100.0).clamp(0.0, 1.0)),
        ),
        (
            FEATURE_TRIGGER_SCORE.to_string(),
            round6((snapshot.trigger_score / 100.0).clamp(0.0, 1.0)),
        ),
        (
            FEATURE_EXTERNAL_DIMENSION_SCORE.to_string(),
            round6(
                (dimension_score(snapshot, RiskDimension::ExternalSector) / 100.0).clamp(0.0, 1.0),
            ),
        ),
        (
            FEATURE_COVERAGE_SCORE.to_string(),
            data_trust.coverage_score.clamp(0.0, 1.0),
        ),
    ]);
    let as_of_date = snapshot.as_of_date;

    for spec in FORMAL_OBSERVATION_FEATURE_SPECS {
        if let Some(value) = formal_observation_feature_value(observations, spec, as_of_date) {
            features.insert(spec.feature_name.to_string(), round6(value));
        }
    }

    features
}

fn dimension_score(snapshot: &RiskSnapshot, dimension: RiskDimension) -> f64 {
    snapshot
        .dimensions
        .iter()
        .find(|score| score.dimension == dimension)
        .map(|score| score.score)
        .unwrap_or(0.0)
}

pub(super) fn worst_key_indicator_freshness(
    key_indicators: &[KeyIndicatorStatus],
) -> FreshnessStatus {
    key_indicators
        .iter()
        .map(|indicator| indicator.status)
        .max_by_key(|status| freshness_rank(*status))
        .unwrap_or(FreshnessStatus::Missing)
}

fn freshness_rank(status: FreshnessStatus) -> u8 {
    match status {
        FreshnessStatus::Fresh => 0,
        FreshnessStatus::Delayed => 1,
        FreshnessStatus::Stale => 2,
        FreshnessStatus::Missing => 3,
    }
}
