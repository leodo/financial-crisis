use fc_domain::{AssessmentHistoryPoint, DecisionPosture, TimeToRiskBucket};

use super::super::signals::{
    release_review_has_prepare_weeks_score_confirmation_gap,
    release_review_has_strong_prepare_trigger_code, release_review_runtime_floor_hits,
    release_review_strict_prepare_p20d_threshold, release_review_strict_prepare_p60d_threshold,
};
use super::gating::release_review_runtime_actionable_block_category;

pub(in super::super) fn release_review_posture_name(
    point: &AssessmentHistoryPoint,
) -> &'static str {
    match point.posture {
        DecisionPosture::Normal => "normal",
        DecisionPosture::Prepare => "prepare",
        DecisionPosture::Hedge => "hedge",
        DecisionPosture::Defend => "defend",
    }
}

pub(in super::super) fn release_review_time_bucket_name(
    point: &AssessmentHistoryPoint,
) -> &'static str {
    match point.time_to_risk_bucket {
        TimeToRiskBucket::Normal => "normal",
        TimeToRiskBucket::Months => "months",
        TimeToRiskBucket::Weeks => "weeks",
        TimeToRiskBucket::Now => "now",
    }
}

fn release_review_runtime_gate_gap_facet(
    point: &AssessmentHistoryPoint,
    thresholds: Option<&crate::RuntimeThresholdDiagnosticsWire>,
) -> &'static str {
    let defend_only_runtime_floor_hit = release_review_runtime_floor_hits(point, thresholds)
        .is_some_and(|hits| hits.defend && !hits.hedge && !hits.prepare);
    if defend_only_runtime_floor_hit {
        return "none";
    }
    let strict_prepare_p20d_threshold = release_review_strict_prepare_p20d_threshold(thresholds);
    let strict_prepare_p60d_threshold = release_review_strict_prepare_p60d_threshold(thresholds);
    match (
        point.p_20d < strict_prepare_p20d_threshold,
        point.p_60d < strict_prepare_p60d_threshold,
    ) {
        (true, true) => "p20d_and_p60d",
        (true, false) => "p20d_only",
        (false, true) => "p60d_only",
        (false, false) => "none",
    }
}

fn release_review_trigger_family(point: &AssessmentHistoryPoint) -> &'static str {
    if point.posture_trigger_codes.is_empty() {
        return "none";
    }

    let mut has_prepare = false;
    let mut has_hedge = false;
    let mut has_defend = false;
    let mut has_other = false;
    for code in &point.posture_trigger_codes {
        if code.starts_with("prepare_") {
            has_prepare = true;
        } else if code.starts_with("hedge_") {
            has_hedge = true;
        } else if code.starts_with("defend_") {
            has_defend = true;
        } else {
            has_other = true;
        }
    }

    let family_count = [has_prepare, has_hedge, has_defend, has_other]
        .into_iter()
        .filter(|present| *present)
        .count();
    if family_count > 1 {
        return "mixed";
    }
    if has_prepare {
        "prepare"
    } else if has_hedge {
        "hedge"
    } else if has_defend {
        "defend"
    } else {
        "other"
    }
}

fn release_review_runtime_confirmation_facet(
    point: &AssessmentHistoryPoint,
    use_transitional_bridge: bool,
    thresholds: Option<&crate::RuntimeThresholdDiagnosticsWire>,
) -> &'static str {
    if matches!(point.time_to_risk_bucket, TimeToRiskBucket::Months)
        && point.overall_score < 62.0
        && point.external_shock_score < 48.0
    {
        return "months_score_low";
    }

    if matches!(point.posture, DecisionPosture::Prepare)
        && point.overall_score < 60.0
        && point.external_shock_score < 46.0
        && !release_review_has_strong_prepare_trigger_code(point)
    {
        return "prepare_score_low";
    }

    if release_review_has_prepare_weeks_score_confirmation_gap(point, thresholds) {
        return "prepare_weeks_score_low";
    }

    if use_transitional_bridge
        && matches!(point.posture, DecisionPosture::Prepare)
        && point.overall_score < 58.0
    {
        return "prepare_bridge_low";
    }

    if use_transitional_bridge
        && matches!(point.time_to_risk_bucket, TimeToRiskBucket::Months)
        && point.overall_score < 58.0
    {
        return "months_bridge_low";
    }

    "ok_or_not_needed"
}

pub(super) fn release_review_runtime_continuity_facets(
    point: &AssessmentHistoryPoint,
    use_transitional_bridge: bool,
    thresholds: Option<&crate::RuntimeThresholdDiagnosticsWire>,
) -> Vec<String> {
    if release_review_runtime_actionable_block_category(point, use_transitional_bridge, thresholds)
        .is_none()
    {
        return Vec::new();
    }

    vec![
        format!("posture:{}", release_review_posture_name(point)),
        format!("bucket:{}", release_review_time_bucket_name(point)),
        format!("trigger:{}", release_review_trigger_family(point)),
        format!(
            "gate_gap:{}",
            release_review_runtime_gate_gap_facet(point, thresholds)
        ),
        format!(
            "confirmation:{}",
            release_review_runtime_confirmation_facet(
                point,
                use_transitional_bridge,
                thresholds
            )
        ),
    ]
}
