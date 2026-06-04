use std::{collections::BTreeMap, fs, path::PathBuf};

use anyhow::bail;
use chrono::{Duration, NaiveDate, Utc};
use fc_domain::{
    probability_feature_names_for_transform, ActionabilityBundle, ModelReleaseManifest,
    ModelReleaseRecord, ProbabilityBundle, ProbabilityBundleEvaluation, ProbabilityHorizonBundle,
};
use fc_storage::SqliteStore;
use serde::Serialize;

use crate::commands::{PipelineDatasetSource, PipelineTrainOptions};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ProbabilityTrainingRegime {
    Normal,
    PositiveWindow,
    PreWarningBuffer,
    InCrisis,
    PostCrisisCooldown,
}

pub(crate) fn probability_training_regime_name(regime: ProbabilityTrainingRegime) -> &'static str {
    match regime {
        ProbabilityTrainingRegime::Normal => "normal",
        ProbabilityTrainingRegime::PositiveWindow => "positive_window",
        ProbabilityTrainingRegime::PreWarningBuffer => "pre_warning_buffer",
        ProbabilityTrainingRegime::InCrisis => "in_crisis",
        ProbabilityTrainingRegime::PostCrisisCooldown => "post_crisis_cooldown",
    }
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ProbabilityTrainingRow {
    pub(crate) as_of_date: NaiveDate,
    pub(crate) market_scope: String,
    pub(crate) release_id: Option<String>,
    pub(crate) probability_mode: Option<String>,
    pub(crate) freshness_status: Option<String>,
    pub(crate) time_to_risk_bucket: Option<String>,
    pub(crate) split_name: Option<String>,
    pub(crate) features: BTreeMap<String, f64>,
    pub(crate) primary_scenario_id: Option<String>,
    pub(crate) scenario_family: Option<String>,
    pub(crate) scenario_training_role: Option<String>,
    pub(crate) days_to_primary_crisis_start: Option<i64>,
    pub(crate) primary_scenario_supports_5d: bool,
    pub(crate) primary_scenario_supports_20d: bool,
    pub(crate) primary_scenario_supports_60d: bool,
    pub(crate) label_5d: u8,
    pub(crate) label_20d: u8,
    pub(crate) label_60d: u8,
    pub(crate) regime_5d: crate::ProbabilityTrainingRegime,
    pub(crate) regime_20d: crate::ProbabilityTrainingRegime,
    pub(crate) regime_60d: crate::ProbabilityTrainingRegime,
    pub(crate) action_label_5d: u8,
    pub(crate) action_label_20d: u8,
    pub(crate) action_label_60d: u8,
    pub(crate) prepare_episode_label: u8,
    pub(crate) hedge_episode_label: u8,
    pub(crate) defend_episode_label: u8,
    pub(crate) primary_action_level: Option<String>,
    pub(crate) action_episode_id: Option<String>,
    pub(crate) action_episode_phase: String,
    pub(crate) protected_action_window: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[allow(dead_code)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ProbabilityTargetLabelMode {
    ForwardCrisis,
    ActionWindow,
    ActionEpisode,
}

impl ProbabilityTargetLabelMode {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::ForwardCrisis => "forward_crisis",
            Self::ActionWindow => "action_window",
            Self::ActionEpisode => "action_episode",
        }
    }
}

impl ProbabilityTrainingRow {
    pub(crate) fn label_for_horizon(
        &self,
        label_mode: ProbabilityTargetLabelMode,
        horizon_days: u32,
    ) -> f64 {
        match (label_mode, horizon_days) {
            (ProbabilityTargetLabelMode::ForwardCrisis, 5) => self.label_5d as f64,
            (ProbabilityTargetLabelMode::ForwardCrisis, 20) => self.label_20d as f64,
            (ProbabilityTargetLabelMode::ForwardCrisis, 60) => self.label_60d as f64,
            (ProbabilityTargetLabelMode::ActionWindow, 5) => self.action_label_5d as f64,
            (ProbabilityTargetLabelMode::ActionWindow, 20) => self.action_label_20d as f64,
            (ProbabilityTargetLabelMode::ActionWindow, 60) => self.action_label_60d as f64,
            (ProbabilityTargetLabelMode::ActionEpisode, 5) => self.defend_episode_label as f64,
            (ProbabilityTargetLabelMode::ActionEpisode, 20) => self.hedge_episode_label as f64,
            (ProbabilityTargetLabelMode::ActionEpisode, 60) => self.prepare_episode_label as f64,
            _ => 0.0,
        }
    }

    pub(crate) fn action_episode_phase_for_horizon(
        &self,
        horizon_days: u32,
    ) -> crate::ActionEpisodePhase {
        let Some(level) = crate::actionability_level_for_proxy_horizon(horizon_days) else {
            return crate::ActionEpisodePhase::Outside;
        };
        let Some(action_episode_id) = self.action_episode_id.as_deref() else {
            return crate::ActionEpisodePhase::Outside;
        };
        if !action_episode_id.ends_with(crate::actionability_level_text(level)) {
            return crate::ActionEpisodePhase::Outside;
        }
        match self.action_episode_phase.as_str() {
            "primary" => crate::ActionEpisodePhase::Primary,
            "late_validation" => crate::ActionEpisodePhase::LateValidation,
            "cooldown" => crate::ActionEpisodePhase::Cooldown,
            _ => crate::ActionEpisodePhase::Outside,
        }
    }

    pub(crate) fn primary_scenario_supports_horizon(&self, horizon_days: u32) -> Option<bool> {
        self.primary_scenario_id
            .as_ref()
            .map(|_| match horizon_days {
                5 => self.primary_scenario_supports_5d,
                20 => self.primary_scenario_supports_20d,
                60 => self.primary_scenario_supports_60d,
                _ => false,
            })
    }

    pub(crate) fn regime_for_horizon(&self, horizon_days: u32) -> crate::ProbabilityTrainingRegime {
        match horizon_days {
            5 => self.regime_5d,
            20 => self.regime_20d,
            60 => self.regime_60d,
            _ => crate::ProbabilityTrainingRegime::Normal,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct ProbabilityTrainingInput {
    pub(crate) dataset_source: PipelineDatasetSource,
    pub(crate) dataset_label: String,
    pub(crate) market_scope: String,
    pub(crate) point_in_time_mode: String,
    pub(crate) feature_set_version: String,
    pub(crate) label_version: String,
    pub(crate) feature_names: Vec<String>,
    pub(crate) train_rows: Vec<ProbabilityTrainingRow>,
    pub(crate) calibration_rows: Vec<ProbabilityTrainingRow>,
    pub(crate) evaluation_rows: Vec<ProbabilityTrainingRow>,
}

#[derive(Debug, Clone)]
pub(crate) struct PipelineArtifacts {
    pub(crate) release: ModelReleaseRecord,
    pub(crate) bundle: ProbabilityBundle,
    pub(crate) bundle_path: PathBuf,
    pub(crate) manifest_path: PathBuf,
    pub(crate) evaluation_path: PathBuf,
    pub(crate) dataset_source: String,
    pub(crate) dataset_label: String,
}

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

pub(crate) fn chronological_split(
    dataset: &[ProbabilityTrainingRow],
) -> anyhow::Result<(
    Vec<ProbabilityTrainingRow>,
    Vec<ProbabilityTrainingRow>,
    Vec<ProbabilityTrainingRow>,
)> {
    let (train_end, calibration_end) = chronological_split_bounds(dataset.len())?;
    Ok((
        dataset[..train_end].to_vec(),
        dataset[train_end..calibration_end].to_vec(),
        dataset[calibration_end..].to_vec(),
    ))
}

pub(crate) fn validate_split_bounds(
    dataset_len: usize,
    train_end: usize,
    calibration_end: usize,
) -> anyhow::Result<()> {
    if dataset_len < 30 {
        bail!("dataset is too small for chronological split");
    }
    if train_end < 30 || calibration_end <= train_end + 10 || calibration_end >= dataset_len {
        bail!("unable to construct train/calibration/evaluation split");
    }
    if dataset_len.saturating_sub(calibration_end) < 10 {
        bail!("evaluation split would be too small");
    }
    Ok(())
}

pub(crate) fn chronological_split_bounds(dataset_len: usize) -> anyhow::Result<(usize, usize)> {
    let train_end = (dataset_len * 6 / 10)
        .max(30)
        .min(dataset_len.saturating_sub(20));
    let calibration_end = (dataset_len * 8 / 10)
        .max(train_end + 10)
        .min(dataset_len.saturating_sub(10));
    validate_split_bounds(dataset_len, train_end, calibration_end)?;
    Ok((train_end, calibration_end))
}

pub(crate) fn training_rows_support_label_mode(
    train_rows: &[ProbabilityTrainingRow],
    calibration_rows: &[ProbabilityTrainingRow],
    evaluation_rows: &[ProbabilityTrainingRow],
    label_mode: ProbabilityTargetLabelMode,
) -> bool {
    [5_u32, 20_u32, 60_u32].into_iter().all(|horizon_days| {
        train_rows
            .iter()
            .any(|row| row.label_for_horizon(label_mode, horizon_days) > 0.0)
            && calibration_rows
                .iter()
                .any(|row| row.label_for_horizon(label_mode, horizon_days) > 0.0)
            && evaluation_rows
                .iter()
                .any(|row| row.label_for_horizon(label_mode, horizon_days) > 0.0)
    })
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

    let actionability = if matches!(training.dataset_source, PipelineDatasetSource::Formal)
        && training_rows_support_label_mode(
            &training.train_rows,
            &training.calibration_rows,
            &training.evaluation_rows,
            ProbabilityTargetLabelMode::ActionEpisode,
        ) {
        let candidate = crate::train_actionability_bundle(
            &training.train_rows,
            &training.calibration_rows,
            &training.evaluation_rows,
            &training.feature_names,
            &generated_at.format("%Y%m%dT%H%M%S").to_string(),
        )?;
        let guard_regressions = crate::actionability_bundle_quality_regressions(&candidate);
        if guard_regressions.is_empty() {
            Some(candidate)
        } else {
            println!("Actionability head disabled for this release:");
            for regression in &guard_regressions {
                println!("  - {regression}");
            }
            None
        }
    } else {
        None
    };

    let aggregate_evaluation = crate::summarize_bundle_evaluation(&horizons);
    let release_suffix = generated_at.format("%Y%m%dT%H%M%S").to_string();
    let release_id = format!("{}_{}", options.release_prefix, release_suffix);
    let bundle_note = match training.dataset_source {
        PipelineDatasetSource::Formal => format!(
            "Formal bundle trained from persisted formal dataset {} built from raw observations -> feature snapshots -> scenario labels; model_shape={} feature_transform={}; crisis-prior head uses forward-crisis labels, and {}.",
            training.dataset_label,
            options.model_shape.as_str(),
            options.model_shape.feature_transform(),
            if actionability.is_some() {
                "actionability head uses episode-native prepare/hedge/defend labels when quality gates pass"
            } else {
                "independent actionability head was omitted because evaluation quality gates did not pass, so runtime falls back to probability-context fusion"
            }
        ),
        PipelineDatasetSource::Snapshot => {
            "Transitional formal bundle trained from persisted heuristic prediction snapshots, calibrated with chronological holdout slices, and reweighted toward positive warning windows under severe class imbalance.".to_string()
        }
    };
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

pub(crate) fn forward_crisis_label(
    as_of_date: NaiveDate,
    scenarios: &[crate::CrisisScenario],
    horizon_days: i64,
) -> u8 {
    let horizon_days_u32 = horizon_days as u32;
    scenarios.iter().any(|scenario| {
        let anchor_date = if crate::scenario_supports_horizon(scenario, horizon_days_u32) {
            crate::label_anchor_date(scenario, horizon_days_u32)
        } else {
            scenario.crisis_start
        };
        let lead_days = (anchor_date - as_of_date).num_days();
        (1..=horizon_days).contains(&lead_days)
    }) as u8
}

pub(crate) fn post_crisis_cooldown_days(horizon_days: u32) -> i64 {
    match horizon_days {
        5 => 14,
        20 => 30,
        60 => 45,
        _ => horizon_days as i64,
    }
}

pub(crate) fn forward_crisis_training_regime(
    as_of_date: NaiveDate,
    scenarios: &[crate::CrisisScenario],
    horizon_days: u32,
) -> ProbabilityTrainingRegime {
    if forward_crisis_label(as_of_date, scenarios, horizon_days as i64) > 0 {
        return ProbabilityTrainingRegime::PositiveWindow;
    }

    let positive_buffer = scenarios.iter().any(|scenario| {
        let anchor_date = if crate::scenario_supports_horizon(scenario, horizon_days) {
            crate::label_anchor_date(scenario, horizon_days)
        } else {
            scenario.crisis_start
        };
        let positive_start = anchor_date
            .checked_sub_signed(Duration::days(horizon_days as i64))
            .unwrap_or(anchor_date);
        as_of_date >= crate::action_window_start_date(scenario, horizon_days)
            && as_of_date < positive_start
    });
    if positive_buffer {
        return ProbabilityTrainingRegime::PreWarningBuffer;
    }

    if scenarios
        .iter()
        .any(|scenario| as_of_date >= scenario.crisis_start && as_of_date <= scenario.crisis_end)
    {
        return ProbabilityTrainingRegime::InCrisis;
    }

    let cooldown = scenarios.iter().any(|scenario| {
        let cooldown_end = scenario
            .crisis_end
            .checked_add_signed(Duration::days(post_crisis_cooldown_days(horizon_days)))
            .unwrap_or(scenario.crisis_end);
        as_of_date > scenario.crisis_end && as_of_date <= cooldown_end
    });
    if cooldown {
        return ProbabilityTrainingRegime::PostCrisisCooldown;
    }

    ProbabilityTrainingRegime::Normal
}

pub(crate) fn forward_crisis_training_regime_with_context(
    as_of_date: NaiveDate,
    positive_scenarios: &[crate::CrisisScenario],
    context_scenarios: &[crate::CrisisScenario],
    horizon_days: u32,
) -> ProbabilityTrainingRegime {
    let base_regime = forward_crisis_training_regime(as_of_date, positive_scenarios, horizon_days);
    if !matches!(base_regime, ProbabilityTrainingRegime::Normal) || horizon_days < 20 {
        return base_regime;
    }

    match crate::protected_context_phase_for_date(as_of_date, positive_scenarios, context_scenarios)
    {
        Some(crate::ActionEpisodePhase::Primary | crate::ActionEpisodePhase::LateValidation) => {
            ProbabilityTrainingRegime::PreWarningBuffer
        }
        Some(crate::ActionEpisodePhase::Cooldown) => ProbabilityTrainingRegime::PostCrisisCooldown,
        _ => base_regime,
    }
}

pub(crate) fn ensure_positive_labels(
    rows: &[ProbabilityTrainingRow],
    horizon_days: u32,
    split_name: &str,
    label_mode: ProbabilityTargetLabelMode,
) -> anyhow::Result<()> {
    let positives = rows
        .iter()
        .filter(|row| row.label_for_horizon(label_mode, horizon_days) > 0.0)
        .count();
    if positives == 0 {
        bail!(
            "no positive {horizon_days}d {} labels found in the {split_name} split",
            label_mode.as_str()
        );
    }
    Ok(())
}
