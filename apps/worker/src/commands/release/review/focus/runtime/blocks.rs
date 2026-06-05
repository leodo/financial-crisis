use std::collections::{BTreeMap, BTreeSet};

use fc_domain::{AssessmentHistoryPoint, DecisionPosture, TimeToRiskBucket};

use super::signals::{
    release_review_has_strong_prepare_trigger_code, release_review_hits_runtime_floor,
    release_review_is_actionable_warning_point,
};

pub(super) fn release_review_actionable_diagnostic(
    point: &AssessmentHistoryPoint,
    use_transitional_bridge: bool,
    thresholds: Option<&crate::RuntimeThresholdDiagnosticsWire>,
) -> String {
    if release_review_is_actionable_warning_point(point, use_transitional_bridge) {
        return "actionable".to_string();
    }

    let runtime_floor_hit = release_review_hits_runtime_floor(point, thresholds);
    let mut review_gate_gaps = Vec::new();
    if point.p_20d < 0.18 {
        review_gate_gaps.push(format!("p20d {} < 18%", crate::format_pct(point.p_20d)));
    }
    if point.p_60d < 0.45 {
        review_gate_gaps.push(format!("p60d {} < 45%", crate::format_pct(point.p_60d)));
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

pub(super) fn release_review_runtime_actionable_block_category(
    point: &AssessmentHistoryPoint,
    use_transitional_bridge: bool,
    thresholds: Option<&crate::RuntimeThresholdDiagnosticsWire>,
) -> Option<&'static str> {
    if release_review_is_actionable_warning_point(point, use_transitional_bridge)
        || !release_review_hits_runtime_floor(point, thresholds)
    {
        return None;
    }

    if point.p_20d < 0.18 || point.p_60d < 0.45 {
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

pub(super) fn release_review_runtime_actionable_block_reason(
    point: &AssessmentHistoryPoint,
    use_transitional_bridge: bool,
    thresholds: Option<&crate::RuntimeThresholdDiagnosticsWire>,
) -> Option<String> {
    release_review_runtime_actionable_block_category(point, use_transitional_bridge, thresholds)
        .map(|_| release_review_actionable_diagnostic(point, use_transitional_bridge, thresholds))
}

pub(super) fn release_review_runtime_block_counts(
    baseline_points: &[&AssessmentHistoryPoint],
    baseline_use_transitional_bridge: bool,
    baseline_thresholds: Option<&crate::RuntimeThresholdDiagnosticsWire>,
    candidate_points: &[&AssessmentHistoryPoint],
    candidate_use_transitional_bridge: bool,
    candidate_thresholds: Option<&crate::RuntimeThresholdDiagnosticsWire>,
) -> Vec<crate::ReleaseReviewRuntimeBlockCount> {
    let collect_counts =
        |points: &[&AssessmentHistoryPoint],
         use_transitional_bridge: bool,
         thresholds: Option<&crate::RuntimeThresholdDiagnosticsWire>| {
            points
                .iter()
                .fold(BTreeMap::<String, u32>::new(), |mut acc, point| {
                    if let Some(category) = release_review_runtime_actionable_block_category(
                        point,
                        use_transitional_bridge,
                        thresholds,
                    ) {
                        *acc.entry(category.to_string()).or_default() += 1;
                    }
                    acc
                })
        };

    let baseline_counts = collect_counts(
        baseline_points,
        baseline_use_transitional_bridge,
        baseline_thresholds,
    );
    let candidate_counts = collect_counts(
        candidate_points,
        candidate_use_transitional_bridge,
        candidate_thresholds,
    );
    let categories = baseline_counts
        .keys()
        .chain(candidate_counts.keys())
        .cloned()
        .collect::<BTreeSet<_>>();

    categories
        .into_iter()
        .map(|category| {
            let baseline_count = baseline_counts.get(&category).copied().unwrap_or_default();
            let candidate_count = candidate_counts.get(&category).copied().unwrap_or_default();
            crate::ReleaseReviewRuntimeBlockCount {
                category,
                baseline_count,
                candidate_count,
                delta: i64::from(candidate_count) - i64::from(baseline_count),
            }
        })
        .collect()
}

pub(super) fn release_review_runtime_dominant_categories(
    counts: &[crate::ReleaseReviewRuntimeBlockCount],
) -> crate::ReleaseReviewRuntimeDominantCategories {
    let baseline_count = counts
        .iter()
        .map(|row| row.baseline_count)
        .max()
        .unwrap_or(0);
    let candidate_count = counts
        .iter()
        .map(|row| row.candidate_count)
        .max()
        .unwrap_or(0);

    crate::ReleaseReviewRuntimeDominantCategories {
        baseline_categories: if baseline_count == 0 {
            Vec::new()
        } else {
            counts
                .iter()
                .filter(|row| row.baseline_count == baseline_count)
                .map(|row| row.category.clone())
                .collect()
        },
        baseline_count,
        candidate_categories: if candidate_count == 0 {
            Vec::new()
        } else {
            counts
                .iter()
                .filter(|row| row.candidate_count == candidate_count)
                .map(|row| row.category.clone())
                .collect()
        },
        candidate_count,
    }
}

pub(super) fn release_review_primary_failure_mode(
    dominant_blocks: &[String],
    dominant_block_count: u32,
    dominant_facets: &[String],
    dominant_facet_count: u32,
) -> Option<String> {
    if dominant_block_count > 0 {
        if dominant_blocks
            .iter()
            .any(|category| category == "review_gate_gap")
        {
            return Some("strict_gate_mismatch".to_string());
        }
        if dominant_blocks
            .iter()
            .any(|category| category == "posture_bucket_normal")
        {
            return Some("posture_continuity_failure".to_string());
        }
        if dominant_blocks
            .iter()
            .any(|category| category.ends_with("score_confirmation"))
        {
            return Some("score_confirmation_failure".to_string());
        }
        if dominant_blocks
            .iter()
            .any(|category| category.ends_with("bridge_not_armed"))
        {
            return Some("transitional_bridge_failure".to_string());
        }
        return Some("residual_review_l3_failure".to_string());
    }

    if dominant_facet_count > 0 {
        if dominant_facets
            .iter()
            .any(|facet| facet == "posture:normal")
            || dominant_facets.iter().any(|facet| facet == "bucket:normal")
            || dominant_facets.iter().any(|facet| facet == "trigger:none")
        {
            return Some("posture_continuity_failure".to_string());
        }
        if dominant_facets
            .iter()
            .any(|facet| facet.starts_with("gate_gap:") && facet != "gate_gap:none")
        {
            return Some("strict_gate_mismatch".to_string());
        }
        if dominant_facets.iter().any(|facet| {
            facet.starts_with("confirmation:") && facet != "confirmation:ok_or_not_needed"
        }) {
            return Some("score_confirmation_failure".to_string());
        }
        return Some("runtime_continuity_failure".to_string());
    }

    None
}

pub(super) fn release_review_runtime_continuity_facet_counts(
    baseline_points: &[&AssessmentHistoryPoint],
    baseline_use_transitional_bridge: bool,
    baseline_thresholds: Option<&crate::RuntimeThresholdDiagnosticsWire>,
    candidate_points: &[&AssessmentHistoryPoint],
    candidate_use_transitional_bridge: bool,
    candidate_thresholds: Option<&crate::RuntimeThresholdDiagnosticsWire>,
) -> Vec<crate::ReleaseReviewRuntimeBlockCount> {
    let collect_counts =
        |points: &[&AssessmentHistoryPoint],
         use_transitional_bridge: bool,
         thresholds: Option<&crate::RuntimeThresholdDiagnosticsWire>| {
            points
                .iter()
                .fold(BTreeMap::<String, u32>::new(), |mut acc, point| {
                    for facet in release_review_runtime_continuity_facets(
                        point,
                        use_transitional_bridge,
                        thresholds,
                    ) {
                        *acc.entry(facet).or_default() += 1;
                    }
                    acc
                })
        };

    let baseline_counts = collect_counts(
        baseline_points,
        baseline_use_transitional_bridge,
        baseline_thresholds,
    );
    let candidate_counts = collect_counts(
        candidate_points,
        candidate_use_transitional_bridge,
        candidate_thresholds,
    );
    let categories = baseline_counts
        .keys()
        .chain(candidate_counts.keys())
        .cloned()
        .collect::<BTreeSet<_>>();

    categories
        .into_iter()
        .map(|category| {
            let baseline_count = baseline_counts.get(&category).copied().unwrap_or_default();
            let candidate_count = candidate_counts.get(&category).copied().unwrap_or_default();
            crate::ReleaseReviewRuntimeBlockCount {
                category,
                baseline_count,
                candidate_count,
                delta: i64::from(candidate_count) - i64::from(baseline_count),
            }
        })
        .collect()
}

pub(super) fn release_review_posture_name(point: &AssessmentHistoryPoint) -> &'static str {
    match point.posture {
        DecisionPosture::Normal => "normal",
        DecisionPosture::Prepare => "prepare",
        DecisionPosture::Hedge => "hedge",
        DecisionPosture::Defend => "defend",
    }
}

pub(super) fn release_review_time_bucket_name(point: &AssessmentHistoryPoint) -> &'static str {
    match point.time_to_risk_bucket {
        TimeToRiskBucket::Normal => "normal",
        TimeToRiskBucket::Months => "months",
        TimeToRiskBucket::Weeks => "weeks",
        TimeToRiskBucket::Now => "now",
    }
}

fn release_review_runtime_gate_gap_facet(point: &AssessmentHistoryPoint) -> &'static str {
    match (point.p_20d < 0.18, point.p_60d < 0.45) {
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

fn release_review_runtime_continuity_facets(
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
        format!("gate_gap:{}", release_review_runtime_gate_gap_facet(point)),
        format!(
            "confirmation:{}",
            release_review_runtime_confirmation_facet(point, use_transitional_bridge)
        ),
    ]
}
