use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

use crate::{
    AlertType, DataQualitySummary, ProbabilityFamilyOverlayAudit, ProbabilityOverlayContribution,
    QualityGrade, RiskContributor, RiskLevel,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TimeToRiskBucket {
    Normal,
    Months,
    Weeks,
    Now,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DecisionPosture {
    Normal,
    Prepare,
    Hedge,
    Defend,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JpyCarryState {
    Quiet,
    Building,
    Stress,
    Unwind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DataMode {
    Demo,
    Sqlite,
    Postgres,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FreshnessStatus {
    Fresh,
    Delayed,
    Stale,
    Missing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventConfirmationState {
    Quiet,
    Watching,
    Confirmed,
    Escalating,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UserRiskProfile {
    Conservative,
    Neutral,
    Aggressive,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProbabilityBlock {
    pub p_5d: f64,
    pub p_20d: f64,
    pub p_60d: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionabilityBlock {
    pub prepare: f64,
    pub hedge: f64,
    pub defend: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProbabilityHorizonOverlayDiagnostics {
    pub horizon_days: u32,
    pub raw_probability: f64,
    pub calibrated_probability: f64,
    pub final_probability: f64,
    #[serde(default)]
    pub runtime_final_probability: Option<f64>,
    #[serde(default)]
    pub monotonic_lift: f64,
    pub configured_overlay_count: u32,
    #[serde(default)]
    pub contributions: Vec<ProbabilityOverlayContribution>,
    #[serde(default)]
    pub overlay_audits: Vec<ProbabilityFamilyOverlayAudit>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProbabilityDiagnostics {
    #[serde(default)]
    pub horizon_overlays: Vec<ProbabilityHorizonOverlayDiagnostics>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssessmentScores {
    pub overall_score: f64,
    pub structural_score: f64,
    pub trigger_score: f64,
    pub external_shock_score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoricalAnalog {
    pub scenario_id: String,
    pub name: String,
    pub similarity_score: f64,
    pub reference_phase: String,
    pub note: String,
    pub peak_score: f64,
    pub lead_time_days: Option<i64>,
    pub actionable_lead_time_days: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataTrust {
    pub coverage_score: f64,
    pub core_feature_coverage: f64,
    pub trigger_feature_coverage: f64,
    pub external_feature_coverage: f64,
    pub quality_grade: QualityGrade,
    pub data_quality_summary: DataQualitySummary,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssessmentMethodVersions {
    pub score_method_version: String,
    pub prob_model_version: String,
    pub calibration_version: String,
    pub actionability_model_version: Option<String>,
    pub actionability_calibration_version: Option<String>,
    pub feature_set_version: String,
    pub label_version: String,
    pub posture_policy_version: String,
    pub action_playbook_version: String,
    pub fusion_policy_version: Option<String>,
    pub actionability_enabled: bool,
    pub probability_mode: String,
    pub release_status: String,
    pub release_id: Option<String>,
    pub point_in_time_mode: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JpyCarrySnapshot {
    pub state: JpyCarryState,
    pub score: f64,
    pub usdjpy_level: Option<f64>,
    pub jp_call_rate: Option<f64>,
    pub us_short_rate: Option<f64>,
    pub us_jp_short_rate_diff: Option<f64>,
    pub change_5d: Option<f64>,
    pub change_20d: Option<f64>,
    pub realized_vol_20d: Option<f64>,
    pub funding_pressure_score: f64,
    pub vix_coupling_score: f64,
    pub credit_coupling_score: f64,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostureGuidance {
    pub posture: DecisionPosture,
    pub summary: String,
    pub reasons: Vec<String>,
    pub upgrade_condition: String,
    pub downgrade_condition: String,
    #[serde(default)]
    pub trigger_codes: Vec<String>,
    #[serde(default)]
    pub blocker_codes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionGuidanceGovernance {
    pub system_budget_only: bool,
    pub auto_execution_allowed: bool,
    pub manual_confirmation_required: bool,
    pub policy_change_requires_release_review: bool,
    pub policy_change_requires_go_no_go: bool,
    pub required_operator_checks: Vec<String>,
}

impl Default for PositionGuidanceGovernance {
    fn default() -> Self {
        Self {
            system_budget_only: true,
            auto_execution_allowed: false,
            manual_confirmation_required: true,
            policy_change_requires_release_review: true,
            policy_change_requires_go_no_go: true,
            required_operator_checks: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionGuidance {
    pub action_playbook_version: String,
    pub execution_urgency: String,
    pub confidence_gate: String,
    pub target_equity_exposure_pct: f64,
    pub target_cash_pct: f64,
    pub hedge_ratio_pct: f64,
    pub leverage_cap_pct: f64,
    pub option_overlay_pct: f64,
    pub action_summary: String,
    pub actions: Vec<String>,
    pub forbidden_actions: Vec<String>,
    pub reentry_conditions: Vec<String>,
    pub guardrails: Vec<String>,
    pub capital_preservation_overlay_enabled: bool,
    #[serde(default)]
    pub governance: PositionGuidanceGovernance,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeMetadata {
    pub data_mode: DataMode,
    pub generated_at: DateTime<Utc>,
    pub requested_as_of_date: NaiveDate,
    pub latest_observation_at: Option<NaiveDate>,
    pub latest_observation_lag_days: Option<i64>,
    pub demo_mode: bool,
    pub stale_warning: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyIndicatorStatus {
    pub indicator_id: String,
    pub display_name: String,
    pub entity_id: String,
    pub source_id: Option<String>,
    pub dataset_id: Option<String>,
    pub unit: String,
    pub latest_value: Option<f64>,
    pub latest_as_of_date: Option<NaiveDate>,
    pub lag_days: Option<i64>,
    pub stale_threshold_days: i64,
    pub status: FreshnessStatus,
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventSignalSummary {
    pub event_type: AlertType,
    pub level: RiskLevel,
    pub triggered_as_of_date: NaiveDate,
    pub trigger_reason: String,
    pub related_indicators: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventAssessment {
    pub state: EventConfirmationState,
    pub confirmation_score: f64,
    pub recent_event_count: u32,
    pub summary: String,
    pub confirmed_signals: Vec<String>,
    pub pending_gaps: Vec<String>,
    pub recent_events: Vec<EventSignalSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BacktestRollingAuditEpisode {
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub duration_days: u32,
    pub signal_count: u32,
    pub classification: String,
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BacktestRollingAudit {
    pub history_point_count: u32,
    pub actionable_signal_count: u32,
    pub pre_crisis_signal_count: u32,
    pub in_crisis_signal_count: u32,
    pub stress_window_signal_count: u32,
    pub false_positive_signal_count: u32,
    pub false_positive_episode_count: u32,
    pub longest_false_positive_episode_days: u32,
    pub actionable_precision: f64,
    pub classified_episodes: Vec<BacktestRollingAuditEpisode>,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BacktestPerformanceSummary {
    pub scenario_count: u32,
    pub real_scenario_count: u32,
    pub fallback_scenario_count: u32,
    pub structural_warning_rate: f64,
    pub timely_warning_rate: f64,
    pub missed_rate: f64,
    pub avg_structural_lead_time_days: Option<f64>,
    pub avg_lead_time_days: Option<f64>,
    pub median_lead_time_days: Option<f64>,
    pub total_false_positive_count: u32,
    pub history_start: Option<NaiveDate>,
    pub history_end: Option<NaiveDate>,
    pub rolling_audit: BacktestRollingAudit,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserRiskPreferences {
    pub profile: UserRiskProfile,
    pub cash_floor_pct: f64,
    pub max_equity_cap_pct: f64,
    pub max_leverage_pct: f64,
    pub option_overlay_preference_pct: f64,
    pub allow_aggressive_reentry: bool,
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssessmentSnapshot {
    pub as_of_date: NaiveDate,
    pub entity_id: String,
    pub market_scope: String,
    pub probabilities: ProbabilityBlock,
    pub actionability: ActionabilityBlock,
    #[serde(default)]
    pub probability_diagnostics: ProbabilityDiagnostics,
    pub time_to_risk_bucket: TimeToRiskBucket,
    pub posture: DecisionPosture,
    pub conviction_score: f64,
    pub scores: AssessmentScores,
    pub summary: String,
    pub posture_reason: String,
    pub top_risk_drivers: Vec<RiskContributor>,
    pub top_relief_drivers: Vec<RiskContributor>,
    pub historical_analogs: Vec<HistoricalAnalog>,
    pub data_trust: DataTrust,
    pub jpy_carry: JpyCarrySnapshot,
    pub position_guidance: PositionGuidance,
    pub runtime: RuntimeMetadata,
    pub key_indicators: Vec<KeyIndicatorStatus>,
    pub event_assessment: EventAssessment,
    pub backtest_summary: BacktestPerformanceSummary,
    pub user_preferences: UserRiskPreferences,
    pub method: AssessmentMethodVersions,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssessmentHistoryPoint {
    pub as_of_date: NaiveDate,
    pub overall_score: f64,
    pub p_5d: f64,
    pub p_20d: f64,
    pub p_60d: f64,
    #[serde(default)]
    pub raw_p_5d: Option<f64>,
    #[serde(default)]
    pub raw_p_20d: Option<f64>,
    #[serde(default)]
    pub raw_p_60d: Option<f64>,
    pub posture: DecisionPosture,
    pub time_to_risk_bucket: TimeToRiskBucket,
    pub external_shock_score: f64,
    #[serde(default)]
    pub posture_trigger_codes: Vec<String>,
    #[serde(default)]
    pub posture_blocker_codes: Vec<String>,
}
