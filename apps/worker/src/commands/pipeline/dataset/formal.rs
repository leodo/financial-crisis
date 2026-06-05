use std::collections::BTreeMap;

use anyhow::{bail, Context};
use fc_domain::FormalDatasetRowRecord;
use fc_storage::SqliteStore;

use super::super::{PipelineDatasetSource, PipelineTrainOptions};
use super::features::formal_feature_names;

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

pub(super) async fn load_formal_training_dataset(
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
            "formal dataset {} is too small after filters: {} rows found across {} dataset(s), at least 90 are required; backfill more free historical observations and rebuild the formal dataset, or use --dataset-source snapshot only for transitional research runs (never for bootstrap-formal-release)",
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

    let mut train_rows = combined_rows
        .iter()
        .filter(|row| row.split_name == "train")
        .map(|row| {
            formal_row_to_training_row(
                row,
                &positive_scenarios,
                &context_scenarios,
                &scenario_by_id,
            )
        })
        .collect::<Vec<_>>();
    let mut calibration_rows = combined_rows
        .iter()
        .filter(|row| row.split_name == "calibration")
        .map(|row| {
            formal_row_to_training_row(
                row,
                &positive_scenarios,
                &context_scenarios,
                &scenario_by_id,
            )
        })
        .collect::<Vec<_>>();
    let mut evaluation_rows = combined_rows
        .iter()
        .filter(|row| row.split_name == "evaluation")
        .map(|row| {
            formal_row_to_training_row(
                row,
                &positive_scenarios,
                &context_scenarios,
                &scenario_by_id,
            )
        })
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

fn formal_row_to_training_row(
    row: &FormalDatasetRowRecord,
    positive_scenarios: &[crate::CrisisScenario],
    context_scenarios: &[crate::CrisisScenario],
    scenario_by_id: &BTreeMap<String, crate::CrisisScenario>,
) -> crate::ProbabilityTrainingRow {
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
            positive_scenarios,
            context_scenarios,
            5,
        ),
        regime_20d: crate::forward_crisis_training_regime_with_context(
            row.as_of_date,
            positive_scenarios,
            context_scenarios,
            20,
        ),
        regime_60d: crate::forward_crisis_training_regime_with_context(
            row.as_of_date,
            positive_scenarios,
            context_scenarios,
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
}
