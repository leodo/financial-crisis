use fc_domain::PredictionSnapshotRecord;
use fc_storage::SqliteStore;

use super::super::{PipelineDatasetSource, PipelineTrainOptions};
use super::features::{pipeline_features_from_snapshot, transitional_feature_names};

pub(crate) fn build_pipeline_dataset_rows(
    snapshots: &[PredictionSnapshotRecord],
) -> Vec<crate::ProbabilityTrainingRow> {
    let scenario_sets = crate::load_formal_dataset_scenario_sets(
        crate::DEFAULT_FORMAL_SCENARIO_SET_VERSION,
        crate::DEFAULT_FORMAL_LABEL_VERSION,
    )
    .expect("default scenario catalog must contain the main training label set");
    let positive_scenarios = scenario_sets.positive_scenarios;
    let context_scenarios = scenario_sets.context_scenarios;
    let mut rows = snapshots
        .iter()
        .map(|snapshot| {
            let features = pipeline_features_from_snapshot(snapshot);
            let scenario_labels = crate::derive_scenario_label_snapshot(
                snapshot.as_of_date,
                &positive_scenarios,
                &context_scenarios,
            );
            crate::ProbabilityTrainingRow {
                as_of_date: snapshot.as_of_date,
                market_scope: snapshot.market_scope.clone(),
                release_id: snapshot.release_id.clone(),
                probability_mode: Some(snapshot.probability_mode.clone()),
                freshness_status: Some(snapshot.freshness_status.clone()),
                time_to_risk_bucket: Some(snapshot.time_to_risk_bucket.clone()),
                split_name: None,
                features,
                primary_scenario_id: scenario_labels.primary_scenario_id,
                scenario_family: scenario_labels.scenario_family,
                scenario_training_role: scenario_labels.scenario_training_role,
                days_to_primary_crisis_start: scenario_labels.days_to_primary_crisis_start,
                primary_scenario_supports_5d: scenario_labels.primary_scenario_supports_5d,
                primary_scenario_supports_20d: scenario_labels.primary_scenario_supports_20d,
                primary_scenario_supports_60d: scenario_labels.primary_scenario_supports_60d,
                label_5d: scenario_labels.label_5d,
                label_20d: scenario_labels.label_20d,
                label_60d: scenario_labels.label_60d,
                regime_5d: scenario_labels.regime_5d,
                regime_20d: scenario_labels.regime_20d,
                regime_60d: scenario_labels.regime_60d,
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
            }
        })
        .collect::<Vec<_>>();
    rows.sort_by_key(|row| row.as_of_date);
    rows
}

pub(super) async fn load_snapshot_training_dataset(
    store: &SqliteStore,
    options: &PipelineTrainOptions,
) -> anyhow::Result<crate::ProbabilityTrainingInput> {
    let snapshots =
        super::super::super::snapshot::load_training_snapshots(store, &options.query).await?;
    let dataset = build_pipeline_dataset_rows(&snapshots);
    if dataset.len() < 90 {
        anyhow::bail!(
            "training dataset is too small: {} rows found, at least 90 are required",
            dataset.len()
        );
    }

    let (train_rows, calibration_rows, evaluation_rows) = crate::chronological_split(&dataset)?;
    let market_scope = train_rows
        .first()
        .map(|row| row.market_scope.clone())
        .unwrap_or_else(|| "financial_system".to_string());
    let dataset_label = train_rows
        .first()
        .and_then(|row| row.release_id.clone())
        .unwrap_or_else(|| "heuristic_prediction_snapshots".to_string());

    Ok(crate::ProbabilityTrainingInput {
        dataset_source: PipelineDatasetSource::Snapshot,
        dataset_label,
        market_scope,
        point_in_time_mode: "best_effort".to_string(),
        feature_set_version: "feature_prob_meta_v1".to_string(),
        label_version: "label_forward_crisis_v1".to_string(),
        feature_names: transitional_feature_names(),
        train_rows,
        calibration_rows,
        evaluation_rows,
    })
}
