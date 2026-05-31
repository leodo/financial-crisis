use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

use crate::{AlertType, DataQualitySummary, QualityGrade, RiskContributor, RiskLevel};

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
    pub feature_set_version: String,
    pub label_version: String,
    pub posture_policy_version: String,
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionGuidance {
    pub target_equity_exposure_pct: f64,
    pub target_cash_pct: f64,
    pub hedge_ratio_pct: f64,
    pub leverage_cap_pct: f64,
    pub option_overlay_pct: f64,
    pub action_summary: String,
    pub actions: Vec<String>,
    pub guardrails: Vec<String>,
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
pub struct BacktestPerformanceSummary {
    pub scenario_count: u32,
    pub real_scenario_count: u32,
    pub fallback_scenario_count: u32,
    pub timely_warning_rate: f64,
    pub missed_rate: f64,
    pub avg_lead_time_days: Option<f64>,
    pub median_lead_time_days: Option<f64>,
    pub total_false_positive_count: u32,
    pub history_start: Option<NaiveDate>,
    pub history_end: Option<NaiveDate>,
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
    pub posture: DecisionPosture,
    pub time_to_risk_bucket: TimeToRiskBucket,
    pub external_shock_score: f64,
}
