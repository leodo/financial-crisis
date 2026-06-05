use std::fs;

use chrono::Utc;
use fc_domain::{
    probability_feature_names_for_transform, ActionabilityBundle, ModelReleaseManifest,
    ModelReleaseRecord, ProbabilityBundle, ProbabilityBundleEvaluation, ProbabilityHorizonBundle,
};
use fc_storage::SqliteStore;
use serde::Serialize;

use crate::commands::{PipelineDatasetSource, PipelineTrainOptions};

use super::types::PipelineArtifacts;
use super::{
    training_rows_support_label_mode, ProbabilityTargetLabelMode, ProbabilityTrainingInput,
};

#[derive(Debug, Clone, Serialize)]
struct PipelineEvaluationReport {
    release_id: String,
    dataset_source: String,
    dataset_label: String,
    model_family: String,
    feature_transform: String,
    target_label_mode: ProbabilityTargetLabelMode,
    market_scope: String,
    feature_names: Vec<String>,
    training_samples: usize,
    calibration_samples: usize,
    evaluation_samples: usize,
    horizons: Vec<ProbabilityHorizonBundle>,
    actionability: Option<ActionabilityBundle>,
    summary: Option<ProbabilityBundleEvaluation>,
}

pub(crate) async fn train_probability_pipeline(
    store: &SqliteStore,
    options: &PipelineTrainOptions,
) -> anyhow::Result<PipelineArtifacts> {
    let generated_at = Utc::now();
    let training =
        crate::commands::pipeline::load_probability_training_input(store, options).await?;
    let bundle_feature_names = probability_feature_names_for_transform(
        &training.feature_names,
        options.model_shape.feature_transform(),
    );
    let crisis_prior_label_mode = ProbabilityTargetLabelMode::ForwardCrisis;
    let horizons = [5_u32, 20_u32, 60_u32]
        .into_iter()
        .map(|horizon| {
            let base_feature_names = probability_feature_names_for_transform(
                &training.feature_names,
                options
                    .model_shape
                    .base_feature_transform_for_horizon(horizon),
            );
            let overlay_feature_names = probability_feature_names_for_transform(
                &training.feature_names,
                options
                    .model_shape
                    .overlay_feature_transform_for_horizon(horizon),
            );
            crate::train_horizon_bundle(
                &training.train_rows,
                &training.calibration_rows,
                &training.evaluation_rows,
                &base_feature_names,
                &overlay_feature_names,
                horizon,
                crisis_prior_label_mode,
            )
        })
        .collect::<anyhow::Result<Vec<_>>>()?;

    let actionability = maybe_train_actionability_bundle(&training, &generated_at)?;
    let aggregate_evaluation = crate::summarize_bundle_evaluation(&horizons);
    let release_suffix = generated_at.format("%Y%m%dT%H%M%S").to_string();
    let release_id = format!("{}_{}", options.release_prefix, release_suffix);
    let bundle_note = bundle_note(&training, options, actionability.is_some());
    let bundle = ProbabilityBundle {
        bundle_id: release_id.clone(),
        market_scope: training.market_scope.clone(),
        probability_mode: "formal_bundle_v1".to_string(),
        model_family: options.model_shape.as_str().to_string(),
        feature_transform: options.model_shape.feature_transform().to_string(),
        created_at: generated_at,
        feature_names: bundle_feature_names.clone(),
        monotonic_min_gap_5d_to_20d: 0.02,
        monotonic_min_gap_20d_to_60d: 0.03,
        note: bundle_note.clone(),
        horizons: horizons.clone(),
        evaluation: Some(aggregate_evaluation.clone()),
        actionability: actionability.clone(),
    };

    let bundle_path = options.output_dir.join(format!("{release_id}.json"));
    let manifest_dir = options.manifest_dir.clone();
    let manifest_path = manifest_dir.join(format!("{release_id}.json"));
    let evaluation_path = options
        .output_dir
        .join(format!("{release_id}-evaluation.json"));
    fs::create_dir_all(&options.output_dir)?;
    fs::create_dir_all(&manifest_dir)?;

    let release = ModelReleaseRecord {
        manifest: ModelReleaseManifest {
            release_id: release_id.clone(),
            market_scope: bundle.market_scope.clone(),
            status: "approved".to_string(),
            probability_mode: bundle.probability_mode.clone(),
            serving_status: "healthy".to_string(),
            bundle_uri: bundle_path.to_string_lossy().replace('\\', "/"),
            feature_set_version: training.feature_set_version.clone(),
            label_version: training.label_version.clone(),
            prob_model_version: format!("prob_{}_{}", options.model_shape.as_str(), release_suffix),
            calibration_version: format!("platt_{release_suffix}"),
            posture_policy_version: "posture_v1_20260530".to_string(),
            action_playbook_version: "action_playbook_v1_20260531".to_string(),
            point_in_time_mode: training.point_in_time_mode.clone(),
            training_range_start: training.train_rows.first().map(|row| row.as_of_date),
            training_range_end: training.train_rows.last().map(|row| row.as_of_date),
            calibration_range_start: training.calibration_rows.first().map(|row| row.as_of_date),
            calibration_range_end: training.calibration_rows.last().map(|row| row.as_of_date),
            evaluation_range_start: training.evaluation_rows.first().map(|row| row.as_of_date),
            evaluation_range_end: training.evaluation_rows.last().map(|row| row.as_of_date),
            brier_score: bundle
                .evaluation
                .as_ref()
                .map(|summary| summary.brier_score),
            log_loss: bundle.evaluation.as_ref().map(|summary| summary.log_loss),
            ece: bundle.evaluation.as_ref().map(|summary| summary.ece),
            note: format!(
                "Generated by `research pipeline train-probability` from {} dataset {} with model_shape={}.",
                training.dataset_source.as_str(),
                training.dataset_label,
                options.model_shape.as_str()
            ),
        },
        created_at: generated_at,
        activated_at: None,
        retired_at: None,
    };

    let evaluation_report = PipelineEvaluationReport {
        release_id: release_id.clone(),
        dataset_source: training.dataset_source.as_str().to_string(),
        dataset_label: training.dataset_label.clone(),
        model_family: options.model_shape.as_str().to_string(),
        feature_transform: options.model_shape.feature_transform().to_string(),
        target_label_mode: crisis_prior_label_mode,
        market_scope: release.manifest.market_scope.clone(),
        feature_names: bundle_feature_names.clone(),
        training_samples: training.train_rows.len(),
        calibration_samples: training.calibration_rows.len(),
        evaluation_samples: training.evaluation_rows.len(),
        horizons,
        actionability,
        summary: bundle.evaluation.clone(),
    };

    fs::write(&bundle_path, serde_json::to_string_pretty(&bundle)?)?;
    fs::write(
        &manifest_path,
        serde_json::to_string_pretty(&release.manifest)?,
    )?;
    fs::write(
        &evaluation_path,
        serde_json::to_string_pretty(&evaluation_report)?,
    )?;

    Ok(PipelineArtifacts {
        release,
        bundle,
        bundle_path,
        manifest_path,
        evaluation_path,
        dataset_source: training.dataset_source.as_str().to_string(),
        dataset_label: training.dataset_label,
    })
}

fn maybe_train_actionability_bundle(
    training: &ProbabilityTrainingInput,
    generated_at: &chrono::DateTime<Utc>,
) -> anyhow::Result<Option<ActionabilityBundle>> {
    if !matches!(training.dataset_source, PipelineDatasetSource::Formal)
        || !training_rows_support_label_mode(
            &training.train_rows,
            &training.calibration_rows,
            &training.evaluation_rows,
            ProbabilityTargetLabelMode::ActionEpisode,
        )
    {
        return Ok(None);
    }

    let candidate = crate::train_actionability_bundle(
        &training.train_rows,
        &training.calibration_rows,
        &training.evaluation_rows,
        &training.feature_names,
        &generated_at.format("%Y%m%dT%H%M%S").to_string(),
    )?;
    let guard_regressions = crate::actionability_bundle_quality_regressions(&candidate);
    if guard_regressions.is_empty() {
        Ok(Some(candidate))
    } else {
        println!("Actionability head disabled for this release:");
        for regression in &guard_regressions {
            println!("  - {regression}");
        }
        Ok(None)
    }
}

fn bundle_note(
    training: &ProbabilityTrainingInput,
    options: &PipelineTrainOptions,
    actionability_enabled: bool,
) -> String {
    match training.dataset_source {
        PipelineDatasetSource::Formal => format!(
            "Formal bundle trained from persisted formal dataset {} built from raw observations -> feature snapshots -> scenario labels; model_shape={} feature_transform={}; crisis-prior head uses forward-crisis labels, and {}.",
            training.dataset_label,
            options.model_shape.as_str(),
            options.model_shape.feature_transform(),
            if actionability_enabled {
                "actionability head uses episode-native prepare/hedge/defend labels when quality gates pass"
            } else {
                "independent actionability head was omitted because evaluation quality gates did not pass, so runtime falls back to probability-context fusion"
            }
        ),
        PipelineDatasetSource::Snapshot => {
            "Transitional formal bundle trained from persisted heuristic prediction snapshots, calibrated with chronological holdout slices, and reweighted toward positive warning windows under severe class imbalance.".to_string()
        }
    }
}
