use std::collections::BTreeMap;

use anyhow::{bail, Context};
use fc_domain::{FormalDatasetRowRecord, PredictionSnapshotRecord};
use fc_storage::SqliteStore;

use super::{PipelineDatasetSource, PipelineTrainOptions};

pub(crate) fn transitional_feature_names() -> Vec<String> {
    crate::TRANSITIONAL_PROBABILITY_BUNDLE_FEATURES
        .iter()
        .map(|feature| (*feature).to_string())
        .collect()
}

pub(crate) async fn resolve_formal_dataset_key(
    store: &SqliteStore,
    dataset_key: Option<&str>,
    dataset_id: &str,
    dataset_version: Option<&str>,
    market_scope: Option<&str>,
) -> anyhow::Result<String> {
    if let Some(dataset_key) = dataset_key {
        return Ok(dataset_key.to_string());
    }
    if let Some(dataset_version) = dataset_version {
        return Ok(crate::formal_dataset_key(dataset_id, dataset_version));
    }

    let market_scope = market_scope.unwrap_or("financial_system");
    let latest = store
        .list_formal_datasets(Some(market_scope), Some(dataset_id), Some(1))
        .await?
        .into_iter()
        .next()
        .with_context(|| {
            format!(
                "no persisted formal dataset found for market scope {market_scope} and dataset id {dataset_id}"
            )
        })?;
    Ok(crate::formal_dataset_key(
        &latest.manifest.dataset_id,
        &latest.manifest.dataset_version,
    ))
}

pub(crate) async fn load_probability_training_input(
    store: &SqliteStore,
    options: &PipelineTrainOptions,
) -> anyhow::Result<crate::ProbabilityTrainingInput> {
    match options.dataset_source {
        PipelineDatasetSource::Formal => load_formal_training_dataset(store, options).await,
        PipelineDatasetSource::Snapshot => load_snapshot_training_dataset(store, options).await,
    }
}

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

fn formal_feature_names() -> Vec<String> {
    crate::FORMAL_PROBABILITY_BUNDLE_FEATURES
        .iter()
        .map(|feature| (*feature).to_string())
        .collect()
}

async fn resolve_formal_training_dataset_key(
    store: &SqliteStore,
    options: &PipelineTrainOptions,
) -> anyhow::Result<String> {
    resolve_formal_dataset_key(
        store,
        options.dataset_key.as_deref(),
        &options.dataset_id,
        options.dataset_version.as_deref(),
        options.query.market_scope.as_deref(),
    )
    .await
}

async fn load_formal_training_dataset(
    store: &SqliteStore,
    options: &PipelineTrainOptions,
) -> anyhow::Result<crate::ProbabilityTrainingInput> {
    let primary_dataset_key = resolve_formal_training_dataset_key(store, options).await?;
    let mut dataset_keys = vec![primary_dataset_key.clone()];
    for dataset_key in &options.aux_dataset_keys {
        if !dataset_keys.contains(dataset_key) {
            dataset_keys.push(dataset_key.clone());
        }
    }

    let primary_dataset = store
        .load_formal_dataset(&primary_dataset_key)
        .await?
        .with_context(|| format!("formal dataset {primary_dataset_key} was not found in SQLite"))?;

    let mut combined_rows = Vec::<FormalDatasetRowRecord>::new();
    let mut positive_by_id = BTreeMap::<String, crate::CrisisScenario>::new();
    let mut context_by_id = BTreeMap::<String, crate::CrisisScenario>::new();

    for dataset_key in &dataset_keys {
        let dataset = store
            .load_formal_dataset(dataset_key)
            .await?
            .with_context(|| format!("formal dataset {dataset_key} was not found in SQLite"))?;
        if dataset.manifest.market_scope != primary_dataset.manifest.market_scope {
            bail!(
                "auxiliary formal dataset {dataset_key} has market scope {} but primary dataset {} uses {}; mixed-market training is not supported",
                dataset.manifest.market_scope,
                primary_dataset_key,
                primary_dataset.manifest.market_scope
            );
        }
        if dataset.manifest.point_in_time_mode != primary_dataset.manifest.point_in_time_mode {
            bail!(
                "auxiliary formal dataset {dataset_key} has point_in_time_mode {} but primary dataset {} uses {}; mixed PIT modes are not supported",
                dataset.manifest.point_in_time_mode,
                primary_dataset_key,
                primary_dataset.manifest.point_in_time_mode
            );
        }
        if dataset.manifest.feature_set_version != primary_dataset.manifest.feature_set_version {
            bail!(
                "auxiliary formal dataset {dataset_key} has feature_set_version {} but primary dataset {} uses {}; mixed feature sets are not supported",
                dataset.manifest.feature_set_version,
                primary_dataset_key,
                primary_dataset.manifest.feature_set_version
            );
        }

        let mut rows = store
            .list_formal_dataset_rows(dataset_key, None, None)
            .await?;
        if let Some(from) = options.query.from {
            rows.retain(|row| row.as_of_date >= from);
        }
        if let Some(to) = options.query.to {
            rows.retain(|row| row.as_of_date <= to);
        }
        if rows.is_empty() {
            bail!(
                "formal dataset {dataset_key} has no rows after the requested date filters; widen --from/--to or choose a different auxiliary dataset"
            );
        }
        combined_rows.extend(rows);

        let scenario_sets = crate::load_formal_dataset_scenario_sets(
            &dataset.manifest.scenario_set_version,
            &dataset.manifest.label_version,
        )?;
        for scenario in scenario_sets.positive_scenarios {
            positive_by_id.insert(scenario.scenario_id.clone(), scenario);
        }
        for scenario in scenario_sets.context_scenarios {
            context_by_id.insert(scenario.scenario_id.clone(), scenario);
        }
    }

    if combined_rows.len() < 90 {
        bail!(
            "formal dataset {} is too small after filters: {} rows found across {} dataset(s), at least 90 are required; backfill more free historical observations and rebuild the formal dataset, or use --dataset-source snapshot as a temporary fallback",
            primary_dataset_key,
            combined_rows.len(),
            dataset_keys.len()
        );
    }

    let positive_scenarios = positive_by_id.into_values().collect::<Vec<_>>();
    let context_scenarios = context_by_id.into_values().collect::<Vec<_>>();
    let scenario_by_id = context_scenarios
        .iter()
        .cloned()
        .map(|scenario| (scenario.scenario_id.clone(), scenario))
        .collect::<BTreeMap<_, _>>();

    let to_training_row = |row: &FormalDatasetRowRecord| {
        let primary_scenario = row
            .primary_scenario_id
            .as_ref()
            .and_then(|scenario_id| scenario_by_id.get(scenario_id));
        crate::ProbabilityTrainingRow {
            as_of_date: row.as_of_date,
            market_scope: row.market_scope.clone(),
            release_id: None,
            probability_mode: Some("formal_bundle_v1".to_string()),
            freshness_status: Some(row.sample_quality_grade.clone()),
            time_to_risk_bucket: row.primary_scenario_id.clone(),
            split_name: Some(row.split_name.clone()),
            features: row.features.clone(),
            primary_scenario_id: row.primary_scenario_id.clone(),
            scenario_family: row.scenario_family.clone(),
            scenario_training_role: row
                .scenario_training_role
                .clone()
                .or_else(|| primary_scenario.map(|scenario| scenario.training_role.clone())),
            days_to_primary_crisis_start: primary_scenario
                .map(|scenario| (scenario.crisis_start - row.as_of_date).num_days()),
            primary_scenario_supports_5d: primary_scenario
                .is_some_and(|scenario| crate::scenario_supports_horizon(scenario, 5)),
            primary_scenario_supports_20d: primary_scenario
                .is_some_and(|scenario| crate::scenario_supports_horizon(scenario, 20)),
            primary_scenario_supports_60d: primary_scenario
                .is_some_and(|scenario| crate::scenario_supports_horizon(scenario, 60)),
            label_5d: row.label_5d,
            label_20d: row.label_20d,
            label_60d: row.label_60d,
            regime_5d: crate::forward_crisis_training_regime_with_context(
                row.as_of_date,
                &positive_scenarios,
                &context_scenarios,
                5,
            ),
            regime_20d: crate::forward_crisis_training_regime_with_context(
                row.as_of_date,
                &positive_scenarios,
                &context_scenarios,
                20,
            ),
            regime_60d: crate::forward_crisis_training_regime_with_context(
                row.as_of_date,
                &positive_scenarios,
                &context_scenarios,
                60,
            ),
            action_label_5d: row.action_label_5d,
            action_label_20d: row.action_label_20d,
            action_label_60d: row.action_label_60d,
            prepare_episode_label: row.prepare_episode_label,
            hedge_episode_label: row.hedge_episode_label,
            defend_episode_label: row.defend_episode_label,
            primary_action_level: row.primary_action_level.clone(),
            action_episode_id: row.action_episode_id.clone(),
            action_episode_phase: row.action_episode_phase.clone(),
            protected_action_window: row.protected_action_window,
        }
    };

    let mut train_rows = combined_rows
        .iter()
        .filter(|row| row.split_name == "train")
        .map(to_training_row)
        .collect::<Vec<_>>();
    let mut calibration_rows = combined_rows
        .iter()
        .filter(|row| row.split_name == "calibration")
        .map(to_training_row)
        .collect::<Vec<_>>();
    let mut evaluation_rows = combined_rows
        .iter()
        .filter(|row| row.split_name == "evaluation")
        .map(to_training_row)
        .collect::<Vec<_>>();

    train_rows.sort_by_key(|row| row.as_of_date);
    calibration_rows.sort_by_key(|row| row.as_of_date);
    evaluation_rows.sort_by_key(|row| row.as_of_date);

    if train_rows.is_empty() || calibration_rows.is_empty() || evaluation_rows.is_empty() {
        bail!(
            "formal dataset {} is missing one or more required splits after filters (train={}, calibration={}, evaluation={}); rebuild it from a broader historical range before training the formal bundle",
            primary_dataset_key,
            train_rows.len(),
            calibration_rows.len(),
            evaluation_rows.len()
        );
    }

    let dataset_label = if dataset_keys.len() == 1 {
        primary_dataset_key.clone()
    } else {
        format!(
            "{} + aux({})",
            primary_dataset_key,
            dataset_keys[1..].join(", ")
        )
    };

    Ok(crate::ProbabilityTrainingInput {
        dataset_source: PipelineDatasetSource::Formal,
        dataset_label,
        market_scope: primary_dataset.manifest.market_scope.clone(),
        point_in_time_mode: primary_dataset.manifest.point_in_time_mode.clone(),
        feature_set_version: primary_dataset.manifest.feature_set_version.clone(),
        label_version: primary_dataset.manifest.label_version.clone(),
        feature_names: formal_feature_names(),
        train_rows,
        calibration_rows,
        evaluation_rows,
    })
}

async fn load_snapshot_training_dataset(
    store: &SqliteStore,
    options: &PipelineTrainOptions,
) -> anyhow::Result<crate::ProbabilityTrainingInput> {
    let snapshots = super::super::snapshot::load_training_snapshots(store, &options.query).await?;
    let dataset = build_pipeline_dataset_rows(&snapshots);
    if dataset.len() < 90 {
        bail!(
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

fn pipeline_features_from_snapshot(snapshot: &PredictionSnapshotRecord) -> BTreeMap<String, f64> {
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
