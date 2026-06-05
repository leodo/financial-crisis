use std::{collections::BTreeMap, path::PathBuf};

use chrono::NaiveDate;
use fc_domain::{ModelReleaseRecord, ProbabilityBundle};
use serde::Serialize;

use crate::commands::PipelineDatasetSource;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ProbabilityTrainingRegime {
    Normal,
    PositiveWindow,
    PreWarningBuffer,
    InCrisis,
    PostCrisisCooldown,
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
    pub(crate) regime_5d: ProbabilityTrainingRegime,
    pub(crate) regime_20d: ProbabilityTrainingRegime,
    pub(crate) regime_60d: ProbabilityTrainingRegime,
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

    pub(crate) fn regime_for_horizon(&self, horizon_days: u32) -> ProbabilityTrainingRegime {
        match horizon_days {
            5 => self.regime_5d,
            20 => self.regime_20d,
            60 => self.regime_60d,
            _ => ProbabilityTrainingRegime::Normal,
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
