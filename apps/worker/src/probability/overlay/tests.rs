use std::collections::BTreeMap;

use chrono::NaiveDate;

use super::{
    audit::{
        family_overlay_audit_specs, family_overlay_has_minimum_support, FamilyOverlayAuditSpec,
    },
    split::split_family_overlay_dataset_rows,
};

fn overlay_row(
    day_index: i64,
    scenario_id: Option<&str>,
    scenario_family: Option<&str>,
    gate_feature: &str,
    gate_value: f64,
    label_20d: u8,
    regime_20d: crate::ProbabilityTrainingRegime,
) -> crate::ProbabilityTrainingRow {
    let mut features = BTreeMap::new();
    features.insert(gate_feature.to_string(), gate_value);
    crate::ProbabilityTrainingRow {
        as_of_date: NaiveDate::from_ymd_opt(2000, 1, 1)
            .unwrap()
            .checked_add_signed(chrono::Duration::days(day_index))
            .unwrap(),
        market_scope: "financial_system".to_string(),
        release_id: None,
        probability_mode: Some("formal_bundle_v1".to_string()),
        freshness_status: Some("a".to_string()),
        time_to_risk_bucket: None,
        split_name: None,
        features,
        primary_scenario_id: scenario_id.map(str::to_string),
        scenario_family: scenario_family.map(str::to_string),
        scenario_training_role: scenario_family.map(|_| "mandatory".to_string()),
        days_to_primary_crisis_start: None,
        primary_scenario_supports_5d: true,
        primary_scenario_supports_20d: true,
        primary_scenario_supports_60d: true,
        label_5d: 0,
        label_20d,
        label_60d: 0,
        regime_5d: crate::ProbabilityTrainingRegime::Normal,
        regime_20d,
        regime_60d: crate::ProbabilityTrainingRegime::Normal,
        action_label_5d: 0,
        action_label_20d: 0,
        action_label_60d: 0,
        prepare_episode_label: 0,
        hedge_episode_label: 0,
        defend_episode_label: 0,
        primary_action_level: None,
        action_episode_id: None,
        action_episode_phase: "outside".to_string(),
        protected_action_window: false,
    }
}

fn systemic_credit_spec() -> FamilyOverlayAuditSpec {
    family_overlay_audit_specs()
        .into_iter()
        .find(|spec| spec.family_id == "systemic_credit")
        .expect("systemic credit spec exists")
}

#[test]
fn family_overlay_minimum_support_uses_aggregate_support_not_original_split_shape() {
    let spec = systemic_credit_spec();
    let audit = fc_domain::ProbabilityFamilyOverlayAudit {
        family_id: "systemic_credit".to_string(),
        gate_feature: spec.gate_feature.to_string(),
        gate_active_threshold: spec.gate_active_threshold,
        scenario_count: 2,
        train_row_count: 621,
        calibration_row_count: 1,
        evaluation_row_count: 118,
        train_gate_active_row_count: 239,
        calibration_gate_active_row_count: 0,
        evaluation_gate_active_row_count: 484,
        positive_label_count: 40,
        early_warning_row_count: 30,
        protected_action_window_count: 0,
        avg_gate_value: 0.11,
        max_gate_value: 0.64,
        note: "test".to_string(),
    };
    assert!(family_overlay_has_minimum_support(&audit, &spec));

    let zero_gate_audit = fc_domain::ProbabilityFamilyOverlayAudit {
        train_gate_active_row_count: 0,
        calibration_gate_active_row_count: 0,
        evaluation_gate_active_row_count: 0,
        ..audit
    };
    assert!(!family_overlay_has_minimum_support(&zero_gate_audit, &spec));
}

#[test]
fn family_overlay_split_recovers_positive_and_early_warning_support_across_scenarios() {
    let spec = systemic_credit_spec();
    let rows = (0..150)
        .map(|index| match index {
            30..=41 => overlay_row(
                index,
                Some("scenario_a"),
                Some("systemic_credit_banking_crisis"),
                spec.gate_feature,
                0.92,
                0,
                crate::ProbabilityTrainingRegime::Normal,
            ),
            42..=49 => overlay_row(
                index,
                Some("scenario_a"),
                Some("systemic_credit_banking_crisis"),
                spec.gate_feature,
                0.92,
                0,
                crate::ProbabilityTrainingRegime::PreWarningBuffer,
            ),
            50..=59 => overlay_row(
                index,
                Some("scenario_a"),
                Some("systemic_credit_banking_crisis"),
                spec.gate_feature,
                0.92,
                1,
                crate::ProbabilityTrainingRegime::PositiveWindow,
            ),
            70..=75 => overlay_row(
                index,
                None,
                None,
                spec.gate_feature,
                0.75,
                0,
                crate::ProbabilityTrainingRegime::Normal,
            ),
            90..=101 => overlay_row(
                index,
                Some("scenario_b"),
                Some("systemic_credit_banking_crisis"),
                spec.gate_feature,
                0.95,
                0,
                crate::ProbabilityTrainingRegime::Normal,
            ),
            102..=109 => overlay_row(
                index,
                Some("scenario_b"),
                Some("systemic_credit_banking_crisis"),
                spec.gate_feature,
                0.95,
                0,
                crate::ProbabilityTrainingRegime::PreWarningBuffer,
            ),
            110..=119 => overlay_row(
                index,
                Some("scenario_b"),
                Some("systemic_credit_banking_crisis"),
                spec.gate_feature,
                0.95,
                1,
                crate::ProbabilityTrainingRegime::PositiveWindow,
            ),
            125..=130 => overlay_row(
                index,
                None,
                None,
                spec.gate_feature,
                0.72,
                0,
                crate::ProbabilityTrainingRegime::Normal,
            ),
            _ => overlay_row(
                index,
                None,
                None,
                spec.gate_feature,
                0.02,
                0,
                crate::ProbabilityTrainingRegime::Normal,
            ),
        })
        .collect::<Vec<_>>();

    let split = split_family_overlay_dataset_rows(
        &rows,
        &spec,
        20,
        crate::ProbabilityTargetLabelMode::ForwardCrisis,
    )
    .expect("family-aware split should succeed");

    assert_eq!(split.strategy, "family_aware");
    assert!(split.train_rows.iter().any(|row| {
        row.label_for_horizon(crate::ProbabilityTargetLabelMode::ForwardCrisis, 20) > 0.0
    }));
    assert!(split.calibration_rows.iter().any(|row| {
        row.label_for_horizon(crate::ProbabilityTargetLabelMode::ForwardCrisis, 20) > 0.0
    }));
    assert!(split.evaluation_rows.iter().any(|row| {
        row.label_for_horizon(crate::ProbabilityTargetLabelMode::ForwardCrisis, 20) > 0.0
    }));
    assert!(split.calibration_rows.iter().any(
        |row| row.regime_for_horizon(20) == crate::ProbabilityTrainingRegime::PreWarningBuffer
    ));
    assert!(split
        .calibration_rows
        .iter()
        .chain(split.evaluation_rows.iter())
        .any(
            |row| row.regime_for_horizon(20) == crate::ProbabilityTrainingRegime::PreWarningBuffer
        ));
}

#[test]
fn family_overlay_split_balanced_fallback_recovers_sparse_topology() {
    let spec = systemic_credit_spec();
    let rows = (0..140)
        .map(|index| match index {
            28..=39 => overlay_row(
                index,
                Some("scenario_a"),
                Some("systemic_credit_banking_crisis"),
                spec.gate_feature,
                0.91,
                0,
                crate::ProbabilityTrainingRegime::Normal,
            ),
            40..=47 => overlay_row(
                index,
                Some("scenario_a"),
                Some("systemic_credit_banking_crisis"),
                spec.gate_feature,
                0.91,
                0,
                crate::ProbabilityTrainingRegime::PreWarningBuffer,
            ),
            48..=57 => overlay_row(
                index,
                Some("scenario_a"),
                Some("systemic_credit_banking_crisis"),
                spec.gate_feature,
                0.91,
                1,
                crate::ProbabilityTrainingRegime::PositiveWindow,
            ),
            70..=81 => overlay_row(
                index,
                Some("scenario_b"),
                Some("systemic_credit_banking_crisis"),
                spec.gate_feature,
                0.88,
                0,
                crate::ProbabilityTrainingRegime::Normal,
            ),
            82..=87 => overlay_row(
                index,
                Some("scenario_b"),
                Some("systemic_credit_banking_crisis"),
                spec.gate_feature,
                0.88,
                0,
                crate::ProbabilityTrainingRegime::PreWarningBuffer,
            ),
            94..=100 => overlay_row(
                index,
                None,
                None,
                spec.gate_feature,
                0.72,
                0,
                crate::ProbabilityTrainingRegime::Normal,
            ),
            112..=118 => overlay_row(
                index,
                None,
                None,
                spec.gate_feature,
                0.68,
                0,
                crate::ProbabilityTrainingRegime::Normal,
            ),
            _ => overlay_row(
                index,
                None,
                None,
                spec.gate_feature,
                0.02,
                0,
                crate::ProbabilityTrainingRegime::Normal,
            ),
        })
        .collect::<Vec<_>>();

    let split = split_family_overlay_dataset_rows(
        &rows,
        &spec,
        20,
        crate::ProbabilityTargetLabelMode::ForwardCrisis,
    )
    .expect("balanced fallback should succeed");

    assert_eq!(split.strategy, "balanced");
    assert!(split.train_rows.iter().any(|row| {
        row.label_for_horizon(crate::ProbabilityTargetLabelMode::ForwardCrisis, 20) > 0.0
    }));
    assert!(split.calibration_rows.iter().any(|row| {
        row.label_for_horizon(crate::ProbabilityTargetLabelMode::ForwardCrisis, 20) > 0.0
    }));
    assert!(split.evaluation_rows.iter().any(|row| {
        row.label_for_horizon(crate::ProbabilityTargetLabelMode::ForwardCrisis, 20) > 0.0
    }));
}
