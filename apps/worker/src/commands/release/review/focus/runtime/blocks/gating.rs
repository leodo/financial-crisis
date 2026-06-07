use fc_domain::{AssessmentHistoryPoint, DecisionPosture, TimeToRiskBucket};

use super::super::signals::{
    release_review_has_prepare_weeks_score_confirmation_gap,
    release_review_has_strong_prepare_trigger_code, release_review_hits_runtime_floor,
    release_review_is_actionable_warning_point, release_review_is_weak_defend_only_runtime_floor,
    release_review_runtime_floor_hits, release_review_strict_prepare_p20d_threshold,
    release_review_strict_prepare_p60d_threshold,
};

pub(in super::super) fn release_review_actionable_diagnostic(
    point: &AssessmentHistoryPoint,
    use_transitional_bridge: bool,
    thresholds: Option<&crate::RuntimeThresholdDiagnosticsWire>,
) -> String {
    if release_review_is_actionable_warning_point(point, use_transitional_bridge, thresholds) {
        return "actionable".to_string();
    }
    if release_review_is_weak_defend_only_runtime_floor(point, thresholds) {
        return "weak defend-only runtime floor blip ignored for continuity audit".to_string();
    }

    let runtime_floor_hit = release_review_hits_runtime_floor(point, thresholds);
    let defend_only_runtime_floor_hit = release_review_runtime_floor_hits(point, thresholds)
        .is_some_and(|hits| hits.defend && !hits.hedge && !hits.prepare);
    let strict_prepare_p20d_threshold = release_review_strict_prepare_p20d_threshold(thresholds);
    let strict_prepare_p60d_threshold = release_review_strict_prepare_p60d_threshold(thresholds);
    let mut review_gate_gaps = Vec::new();
    if !defend_only_runtime_floor_hit {
        if point.p_20d < strict_prepare_p20d_threshold {
            review_gate_gaps.push(format!(
                "p20d {} < {}",
                crate::format_pct(point.p_20d),
                crate::format_pct(strict_prepare_p20d_threshold)
            ));
        }
        if point.p_60d < strict_prepare_p60d_threshold {
            review_gate_gaps.push(format!(
                "p60d {} < {}",
                crate::format_pct(point.p_60d),
                crate::format_pct(strict_prepare_p60d_threshold)
            ));
        }
    }
    if !review_gate_gaps.is_empty() {
        let joined = review_gate_gaps.join(", ");
        return if runtime_floor_hit {
            format!("hit runtime floor, but review gate still needs {joined}")
        } else {
            joined
        };
    }

    if matches!(point.posture, DecisionPosture::Normal)
        && matches!(point.time_to_risk_bucket, TimeToRiskBucket::Normal)
    {
        return if runtime_floor_hit {
            "hit runtime floor, but posture/bucket stayed normal".to_string()
        } else {
            "posture/bucket stayed normal".to_string()
        };
    }

    if matches!(point.time_to_risk_bucket, TimeToRiskBucket::Months)
        && point.overall_score < 62.0
        && point.external_shock_score < 48.0
    {
        return "months setup lacked score confirmation".to_string();
    }

    if matches!(point.posture, DecisionPosture::Prepare)
        && point.overall_score < 60.0
        && point.external_shock_score < 46.0
        && !release_review_has_strong_prepare_trigger_code(point)
    {
        return "prepare setup lacked confirmation".to_string();
    }

    if release_review_has_prepare_weeks_score_confirmation_gap(point, thresholds) {
        return format!(
            "prepare/weeks trigger setup stayed below strict score confirmation (overall {} < 53.0)",
            point.overall_score
        );
    }

    if use_transitional_bridge
        && matches!(point.posture, DecisionPosture::Prepare)
        && point.overall_score < 58.0
    {
        return "prepare bridge not armed".to_string();
    }

    if use_transitional_bridge
        && matches!(point.time_to_risk_bucket, TimeToRiskBucket::Months)
        && point.overall_score < 58.0
    {
        return "months bridge not armed".to_string();
    }

    "review L3 gate not satisfied".to_string()
}

pub(in super::super) fn release_review_runtime_actionable_block_category(
    point: &AssessmentHistoryPoint,
    use_transitional_bridge: bool,
    thresholds: Option<&crate::RuntimeThresholdDiagnosticsWire>,
) -> Option<&'static str> {
    if release_review_is_actionable_warning_point(point, use_transitional_bridge, thresholds)
        || !release_review_hits_runtime_floor(point, thresholds)
        || release_review_is_weak_defend_only_runtime_floor(point, thresholds)
    {
        return None;
    }

    let strict_prepare_p20d_threshold = release_review_strict_prepare_p20d_threshold(thresholds);
    let strict_prepare_p60d_threshold = release_review_strict_prepare_p60d_threshold(thresholds);
    let defend_only_runtime_floor_hit = release_review_runtime_floor_hits(point, thresholds)
        .is_some_and(|hits| hits.defend && !hits.hedge && !hits.prepare);
    if !defend_only_runtime_floor_hit
        && (point.p_20d < strict_prepare_p20d_threshold
            || point.p_60d < strict_prepare_p60d_threshold)
    {
        return Some("review_gate_gap");
    }

    if matches!(point.posture, DecisionPosture::Normal)
        && matches!(point.time_to_risk_bucket, TimeToRiskBucket::Normal)
    {
        return Some("posture_bucket_normal");
    }

    if matches!(point.time_to_risk_bucket, TimeToRiskBucket::Months)
        && point.overall_score < 62.0
        && point.external_shock_score < 48.0
    {
        return Some("months_score_confirmation");
    }

    if matches!(point.posture, DecisionPosture::Prepare)
        && point.overall_score < 60.0
        && point.external_shock_score < 46.0
        && !release_review_has_strong_prepare_trigger_code(point)
    {
        return Some("prepare_score_confirmation");
    }

    if release_review_has_prepare_weeks_score_confirmation_gap(point, thresholds) {
        return Some("prepare_weeks_score_confirmation");
    }

    if use_transitional_bridge
        && matches!(point.posture, DecisionPosture::Prepare)
        && point.overall_score < 58.0
    {
        return Some("prepare_bridge_not_armed");
    }

    if use_transitional_bridge
        && matches!(point.time_to_risk_bucket, TimeToRiskBucket::Months)
        && point.overall_score < 58.0
    {
        return Some("months_bridge_not_armed");
    }

    Some("review_l3_gate_not_satisfied")
}

pub(in super::super) fn release_review_runtime_actionable_block_reason(
    point: &AssessmentHistoryPoint,
    use_transitional_bridge: bool,
    thresholds: Option<&crate::RuntimeThresholdDiagnosticsWire>,
) -> Option<String> {
    release_review_runtime_actionable_block_category(point, use_transitional_bridge, thresholds)
        .map(|_| release_review_actionable_diagnostic(point, use_transitional_bridge, thresholds))
}
