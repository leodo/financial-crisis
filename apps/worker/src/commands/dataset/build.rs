use chrono::{NaiveDate, Utc};
use fc_domain::{FeatureSnapshotRecord, FormalDatasetRowRecord};

use super::{load_formal_dataset_scenario_sets, split::assign_formal_dataset_splits};

pub(super) fn build_main_formal_dataset_rows_with_catalog(
    dataset_key: &str,
    snapshots: &[FeatureSnapshotRecord],
    point_in_time_mode: &str,
    label_version: &str,
    scenario_set_version: &str,
) -> anyhow::Result<Vec<FormalDatasetRowRecord>> {
    let scenario_sets = load_formal_dataset_scenario_sets(scenario_set_version, label_version)?;
    let positive_scenarios = scenario_sets.positive_scenarios;
    let context_scenarios = scenario_sets.context_scenarios;
    let min_date = formal_dataset_min_date(label_version);
    let mut rows = snapshots
        .iter()
        .filter(|snapshot| snapshot.as_of_date >= min_date)
        .filter(|snapshot| formal_dataset_snapshot_is_usable(snapshot, label_version))
        .map(|snapshot| {
            let scenario_labels = crate::derive_scenario_label_snapshot(
                snapshot.as_of_date,
                &positive_scenarios,
                &context_scenarios,
            );
            FormalDatasetRowRecord {
                dataset_key: dataset_key.to_string(),
                split_name: String::new(),
                entity_id: snapshot.entity_id.clone(),
                market_scope: snapshot.market_scope.clone(),
                as_of_date: snapshot.as_of_date,
                point_in_time_mode: point_in_time_mode.to_string(),
                latest_visible_at: snapshot.latest_visible_at,
                coverage_score: snapshot.coverage_score,
                core_feature_coverage: snapshot.core_feature_coverage,
                trigger_feature_coverage: snapshot.trigger_feature_coverage,
                external_feature_coverage: snapshot.external_feature_coverage,
                sample_quality_grade: crate::feature_quality_grade(snapshot.coverage_score)
                    .to_string(),
                primary_scenario_id: scenario_labels.primary_scenario_id,
                scenario_family: scenario_labels.scenario_family,
                scenario_training_role: scenario_labels.scenario_training_role,
                label_5d: scenario_labels.label_5d,
                label_20d: scenario_labels.label_20d,
                label_60d: scenario_labels.label_60d,
                regime_5d: crate::probability_training_regime_name(scenario_labels.regime_5d)
                    .to_string(),
                regime_20d: crate::probability_training_regime_name(scenario_labels.regime_20d)
                    .to_string(),
                regime_60d: crate::probability_training_regime_name(scenario_labels.regime_60d)
                    .to_string(),
                action_label_5d: scenario_labels.action_label_5d,
                action_label_20d: scenario_labels.action_label_20d,
                action_label_60d: scenario_labels.action_label_60d,
                prepare_episode_label: scenario_labels.prepare_episode_label,
                hedge_episode_label: scenario_labels.hedge_episode_label,
                defend_episode_label: scenario_labels.defend_episode_label,
                primary_action_level: scenario_labels.primary_action_level,
                action_episode_id: scenario_labels.action_episode_id,
                action_episode_phase: scenario_labels.action_episode_phase,
                protected_action_window: scenario_labels.protected_action_window,
                features: snapshot.features.clone(),
                created_at: Utc::now(),
            }
        })
        .collect::<Vec<_>>();
    rows.sort_by_key(|row| row.as_of_date);
    assign_formal_dataset_splits(&mut rows, &context_scenarios, label_version);
    Ok(rows)
}

pub(crate) fn formal_dataset_min_date(label_version: &str) -> NaiveDate {
    match label_version {
        "formal_label_v1_ext_acute" => NaiveDate::from_ymd_opt(1987, 1, 1).expect("valid date"),
        _ => NaiveDate::from_ymd_opt(1990, 1, 2).expect("valid date"),
    }
}

pub(crate) fn formal_dataset_snapshot_is_usable(
    snapshot: &FeatureSnapshotRecord,
    label_version: &str,
) -> bool {
    match label_version {
        "formal_label_v1_ext_stress" => {
            snapshot.visibility_status == crate::FEATURE_SNAPSHOT_STATUS_READY
                && snapshot.coverage_score >= 0.75
                && snapshot.core_feature_coverage >= 0.85
                && snapshot.trigger_feature_coverage >= 0.80
                && snapshot.external_feature_coverage >= 0.50
                && crate::has_main_dataset_core_features(&snapshot.features)
        }
        "formal_label_v1_ext_acute" => {
            matches!(
                snapshot.visibility_status.as_str(),
                crate::FEATURE_SNAPSHOT_STATUS_READY
                    | crate::FEATURE_SNAPSHOT_STATUS_COVERAGE_OR_VISIBILITY_FAILED
            ) && snapshot.coverage_score >= 0.55
                && snapshot.core_feature_coverage >= 0.60
                && snapshot.trigger_feature_coverage >= 0.50
                && snapshot.external_feature_coverage >= 0.50
                && crate::has_extension_acute_core_features(&snapshot.features)
        }
        _ => {
            snapshot.visibility_status == crate::FEATURE_SNAPSHOT_STATUS_READY
                && snapshot.coverage_score >= 0.85
                && snapshot.core_feature_coverage >= 0.90
                && snapshot.trigger_feature_coverage >= 0.75
                && snapshot.external_feature_coverage >= 0.70
                && crate::has_main_dataset_core_features(&snapshot.features)
        }
    }
}
