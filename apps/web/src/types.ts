export type RiskLevel = "normal" | "watch" | "stress" | "warning" | "crisis";
export type QualityGrade = "a" | "b" | "c" | "d" | "f";
export type DecisionPosture = "normal" | "prepare" | "hedge" | "defend";
export type TimeToRiskBucket = "normal" | "months" | "weeks" | "now";
export type JpyCarryState = "quiet" | "building" | "stress" | "unwind";
export type DataMode = "demo" | "sqlite" | "postgres";
export type FreshnessStatus = "fresh" | "delayed" | "stale" | "missing";
export type BacktestSignalSource = "real_history" | "fallback_template";
export type EventConfirmationState = "quiet" | "watching" | "confirmed" | "escalating";
export type UserRiskProfile = "conservative" | "neutral" | "aggressive";

export interface DataQualitySummary {
  overall_score: number;
  grade: QualityGrade;
  stale_indicator_count: number;
  low_quality_indicator_count: number;
  prototype_source_count: number;
  blocked_indicator_count: number;
}

export interface RiskContributor {
  indicator_id: string;
  display_name: string;
  dimension: string;
  score: number;
  contribution: number;
  explanation: string;
}

export interface DimensionScore {
  dimension: string;
  label: string;
  score: number;
  level: RiskLevel;
  change_30d: number | null;
  quality_score: number;
  top_contributors: RiskContributor[];
}

export interface RiskSnapshot {
  as_of_date: string;
  entity_id: string;
  market_scope: string;
  overall_score: number;
  overall_level: RiskLevel;
  structural_score: number;
  trigger_score: number;
  level_reason: string;
  dimensions: DimensionScore[];
  top_contributors: RiskContributor[];
  data_quality_summary: DataQualitySummary;
  generated_at: string;
  method_version: string;
}

export interface Indicator {
  indicator_id: string;
  display_name: string;
  dimension: string;
  description: string;
  unit: string;
  frequency: string;
  risk_direction: string;
  default_source_id: string;
  quality_tier: string;
}

export interface Observation {
  indicator_id: string;
  entity_id: string;
  as_of_date: string;
  value: number;
  unit: string;
  source_id: string;
  dataset_id: string;
  quality_score: number;
  quality_flags: string[];
}

export interface IndicatorRisk {
  indicator: Indicator;
  latest_observation: Observation | null;
  score: number;
  level: RiskLevel;
  percentile: number | null;
  change_30d: number | null;
  score_basis: string;
  score_input_value: number | null;
  score_input_unit: string | null;
  quality_grade: QualityGrade;
  contribution: number;
}

export interface DataSource {
  source_id: string;
  display_name: string;
  source_type: string;
  priority: string;
  access_method: string;
  documentation_url: string | null;
  production_allowed: boolean;
  license_note: string;
  health: {
    status: string;
    last_success_at: string | null;
    lag_seconds: number | null;
    consecutive_failures: number;
    quality_score: number;
    message: string;
  };
}

export interface AlertEvent {
  alert_id: string;
  event_type: string;
  scope: string;
  entity_id: string;
  dimension: string | null;
  level: RiskLevel;
  status: string;
  triggered_at: string;
  triggered_as_of_date: string;
  resolved_at: string | null;
  score: number;
  previous_score: number | null;
  trigger_reason: string;
  top_contributors: RiskContributor[];
  related_indicators: string[];
  method_version: string;
}

export interface BacktestScenarioSummary {
  scenario_id: string;
  name: string;
  region: string;
  signal_source: BacktestSignalSource;
  crisis_start: string;
  crisis_end: string;
  first_l2_date: string | null;
  first_l3_date: string | null;
  max_level: RiskLevel;
  max_score: number;
  lead_time_days: number | null;
  actionable_lead_time_days: number | null;
  false_positive_count: number;
  missed: boolean;
  history_start: string | null;
  history_end: string | null;
  history_point_count: number;
  note: string;
  top_contributors: RiskContributor[];
  method_version: string;
}

export interface BacktestWindowPoint {
  as_of_date: string;
  overall_score: number;
  p_5d: number;
  p_20d: number;
  p_60d: number;
  posture: DecisionPosture;
  crisis_window_open: boolean;
}

export interface ProbabilityBlock {
  p_5d: number;
  p_20d: number;
  p_60d: number;
}

export interface ActionabilityBlock {
  prepare: number;
  hedge: number;
  defend: number;
}

export interface ProbabilityOverlayContribution {
  family_id: string;
  gate_feature: string;
  gate_value: number;
  gate: number;
  blend: number;
  overlay_probability: number;
  contribution: number;
}

export interface ProbabilityFamilyOverlayAudit {
  family_id: string;
  gate_feature: string;
  gate_active_threshold: number;
  scenario_count: number;
  train_row_count: number;
  calibration_row_count: number;
  evaluation_row_count: number;
  train_gate_active_row_count: number;
  calibration_gate_active_row_count: number;
  evaluation_gate_active_row_count: number;
  positive_label_count: number;
  early_warning_row_count: number;
  protected_action_window_count: number;
  avg_gate_value: number;
  max_gate_value: number;
  note: string;
}

export interface ProbabilityHorizonOverlayDiagnostics {
  horizon_days: number;
  raw_probability: number;
  calibrated_probability: number;
  final_probability: number;
  runtime_final_probability?: number;
  monotonic_lift?: number;
  configured_overlay_count: number;
  contributions: ProbabilityOverlayContribution[];
  overlay_audits: ProbabilityFamilyOverlayAudit[];
}

export interface ProbabilityDiagnostics {
  horizon_overlays: ProbabilityHorizonOverlayDiagnostics[];
}

export interface AssessmentScores {
  overall_score: number;
  structural_score: number;
  trigger_score: number;
  external_shock_score: number;
}

export interface HistoricalAnalog {
  scenario_id: string;
  name: string;
  similarity_score: number;
  reference_phase: string;
  note: string;
  peak_score: number;
  lead_time_days: number | null;
  actionable_lead_time_days: number | null;
}

export interface DataTrust {
  coverage_score: number;
  core_feature_coverage: number;
  trigger_feature_coverage: number;
  external_feature_coverage: number;
  quality_grade: QualityGrade;
  data_quality_summary: DataQualitySummary;
  warnings: string[];
}

export interface AssessmentMethodVersions {
  score_method_version: string;
  prob_model_version: string;
  calibration_version: string;
  actionability_model_version: string | null;
  actionability_calibration_version: string | null;
  feature_set_version: string;
  label_version: string;
  posture_policy_version: string;
  action_playbook_version: string;
  fusion_policy_version: string | null;
  actionability_enabled: boolean;
  probability_mode: string;
  release_status: string;
  release_id: string | null;
  point_in_time_mode: string;
}

export interface JpyCarrySnapshot {
  state: JpyCarryState;
  score: number;
  usdjpy_level: number | null;
  jp_call_rate: number | null;
  us_short_rate: number | null;
  us_jp_short_rate_diff: number | null;
  change_5d: number | null;
  change_20d: number | null;
  realized_vol_20d: number | null;
  funding_pressure_score: number;
  vix_coupling_score: number;
  credit_coupling_score: number;
  reason: string;
}

export interface PositionGuidance {
  action_playbook_version: string;
  execution_urgency: string;
  confidence_gate: string;
  target_equity_exposure_pct: number;
  target_cash_pct: number;
  hedge_ratio_pct: number;
  leverage_cap_pct: number;
  option_overlay_pct: number;
  action_summary: string;
  actions: string[];
  forbidden_actions: string[];
  reentry_conditions: string[];
  guardrails: string[];
  capital_preservation_overlay_enabled: boolean;
}

export interface RuntimeMetadata {
  data_mode: DataMode;
  generated_at: string;
  requested_as_of_date: string;
  latest_observation_at: string | null;
  latest_observation_lag_days: number | null;
  demo_mode: boolean;
  stale_warning: string | null;
}

export interface KeyIndicatorStatus {
  indicator_id: string;
  display_name: string;
  entity_id: string;
  source_id: string | null;
  dataset_id: string | null;
  unit: string;
  latest_value: number | null;
  latest_as_of_date: string | null;
  lag_days: number | null;
  stale_threshold_days: number;
  status: FreshnessStatus;
  note: string;
}

export interface EventSignalSummary {
  event_type: string;
  level: RiskLevel;
  triggered_as_of_date: string;
  trigger_reason: string;
  related_indicators: string[];
}

export interface EventAssessment {
  state: EventConfirmationState;
  confirmation_score: number;
  recent_event_count: number;
  summary: string;
  confirmed_signals: string[];
  pending_gaps: string[];
  recent_events: EventSignalSummary[];
}

export interface BacktestPerformanceSummary {
  scenario_count: number;
  real_scenario_count: number;
  fallback_scenario_count: number;
  structural_warning_rate: number;
  timely_warning_rate: number;
  missed_rate: number;
  avg_structural_lead_time_days: number | null;
  avg_lead_time_days: number | null;
  median_lead_time_days: number | null;
  total_false_positive_count: number;
  history_start: string | null;
  history_end: string | null;
  rolling_audit: BacktestRollingAudit;
  summary: string;
}

export type BacktestRollingAuditEpisodeClassification = "stress_window" | "false_positive";

export interface BacktestRollingAuditEpisode {
  start_date: string;
  end_date: string;
  duration_days: number;
  signal_count: number;
  classification: BacktestRollingAuditEpisodeClassification;
  note: string;
}

export interface BacktestRollingAudit {
  history_point_count: number;
  actionable_signal_count: number;
  pre_crisis_signal_count: number;
  in_crisis_signal_count: number;
  stress_window_signal_count: number;
  false_positive_signal_count: number;
  false_positive_episode_count: number;
  longest_false_positive_episode_days: number;
  actionable_precision: number;
  classified_episodes: BacktestRollingAuditEpisode[];
  summary: string;
}

export interface UserRiskPreferences {
  profile: UserRiskProfile;
  cash_floor_pct: number;
  max_equity_cap_pct: number;
  max_leverage_pct: number;
  option_overlay_preference_pct: number;
  allow_aggressive_reentry: boolean;
  note: string;
}

export interface AssessmentSnapshot {
  as_of_date: string;
  entity_id: string;
  market_scope: string;
  probabilities: ProbabilityBlock;
  actionability: ActionabilityBlock;
  probability_diagnostics: ProbabilityDiagnostics;
  time_to_risk_bucket: TimeToRiskBucket;
  posture: DecisionPosture;
  conviction_score: number;
  scores: AssessmentScores;
  summary: string;
  posture_reason: string;
  top_risk_drivers: RiskContributor[];
  top_relief_drivers: RiskContributor[];
  historical_analogs: HistoricalAnalog[];
  data_trust: DataTrust;
  jpy_carry: JpyCarrySnapshot;
  position_guidance: PositionGuidance;
  runtime: RuntimeMetadata;
  key_indicators: KeyIndicatorStatus[];
  event_assessment: EventAssessment;
  backtest_summary: BacktestPerformanceSummary;
  user_preferences: UserRiskPreferences;
  method: AssessmentMethodVersions;
}

export interface AssessmentHistoryPoint {
  as_of_date: string;
  overall_score: number;
  p_5d: number;
  p_20d: number;
  p_60d: number;
  raw_p_5d?: number;
  raw_p_20d?: number;
  raw_p_60d?: number;
  posture: DecisionPosture;
  time_to_risk_bucket: TimeToRiskBucket;
  external_shock_score: number;
  posture_trigger_codes: string[];
  posture_blocker_codes: string[];
}

export interface PostureGuidance {
  posture: DecisionPosture;
  summary: string;
  reasons: string[];
  upgrade_condition: string;
  downgrade_condition: string;
  trigger_codes: string[];
  blocker_codes: string[];
}

export interface ProtectedStressWindow {
  window_id: string;
  label: string;
  start_date: string;
  end_date: string;
  note: string;
}

export interface ProtectedStressWindowCatalog {
  catalog_id: string;
  market_scope: string;
  note: string;
  source: string;
  warning: string | null;
  windows: ProtectedStressWindow[];
}

export interface ModelReleaseManifest {
  release_id: string;
  market_scope: string;
  status: string;
  probability_mode: string;
  serving_status: string;
  bundle_uri: string;
  feature_set_version: string;
  label_version: string;
  prob_model_version: string;
  calibration_version: string;
  posture_policy_version: string;
  action_playbook_version: string;
  point_in_time_mode: string;
  training_range_start: string | null;
  training_range_end: string | null;
  calibration_range_start: string | null;
  calibration_range_end: string | null;
  evaluation_range_start: string | null;
  evaluation_range_end: string | null;
  brier_score: number | null;
  log_loss: number | null;
  ece: number | null;
  note: string;
}

export interface ModelReleaseRecord {
  created_at: string;
  activated_at: string | null;
  retired_at: string | null;
  release_id: string;
  market_scope: string;
  status: string;
  probability_mode: string;
  serving_status: string;
  bundle_uri: string;
  feature_set_version: string;
  label_version: string;
  prob_model_version: string;
  calibration_version: string;
  posture_policy_version: string;
  action_playbook_version: string;
  point_in_time_mode: string;
  training_range_start: string | null;
  training_range_end: string | null;
  calibration_range_start: string | null;
  calibration_range_end: string | null;
  evaluation_range_start: string | null;
  evaluation_range_end: string | null;
  brier_score: number | null;
  log_loss: number | null;
  ece: number | null;
  note: string;
}

export interface HistoricalReplayRunRecord {
  replay_run_id: string;
  release_id: string | null;
  market_scope: string;
  from_date: string;
  to_date: string;
  history_cache_key: string;
  feature_set_version: string;
  label_version: string;
  point_in_time_mode: string;
  runtime_policy_version: string;
  action_playbook_version: string;
  protected_window_catalog_id: string;
  source_watermark: string;
  status: string;
  point_count: number;
  failure_reason: string | null;
  created_at: string;
}

export interface PredictionSnapshotRecord {
  as_of_date: string;
  entity_id: string;
  market_scope: string;
  release_id: string | null;
  probability_mode: string;
  release_status: string;
  point_in_time_mode: string;
  overall_score: number;
  external_shock_score: number;
  raw_p_5d: number;
  raw_p_20d: number;
  raw_p_60d: number;
  calibrated_p_5d: number;
  calibrated_p_20d: number;
  calibrated_p_60d: number;
  posture: string;
  time_to_risk_bucket: string;
  feature_set_version: string;
  label_version: string;
  coverage_score: number;
  freshness_status: string;
  method_version: string;
  posture_trigger_codes: string[];
  posture_blocker_codes: string[];
  recorded_at: string;
}

export interface RuntimeThresholdDiagnostics {
  prepare_p60d: number;
  hedge_p20d: number;
  defend_p5d: number;
  severe_now_p20d: number;
  elevated_weeks_p60d: number;
  external_prepare_p20d: number;
  carry_prepare_p60d: number;
  downgrade_prepare_p60d: number;
  downgrade_hedge_p20d: number;
  downgrade_defend_p5d: number;
  history_runtime_policy_version: string;
}

export interface AssessmentMethodResponse {
  method: AssessmentMethodVersions;
  note: string;
  protected_stress_window_catalog: ProtectedStressWindowCatalog;
  runtime_thresholds: RuntimeThresholdDiagnostics;
}

export interface ReleaseReviewAuditAttributionSummary {
  workstream: string;
  attribution: string;
  scenario_count: number;
  protected_count: number;
  baseline_count: number;
  candidate_count: number;
  baseline_scenarios: string[];
  candidate_scenarios: string[];
  explanation: string;
}

export interface ReleaseReviewAuditActionSummary {
  workstream: string;
  attribution: string;
  action_type: string;
  scenario_count: number;
  protected_count: number;
  recommendation: string;
}

export interface ReleaseReviewArtifactSummary {
  reviewed_at: string;
  market_scope: string;
  history_mode: string;
  original_active_release_id: string;
  restored_release_id: string;
  baseline_release_id: string;
  candidate_release_id: string;
  overall_guard_passed: boolean;
  recommendation: string;
  historical_audit_attribution: ReleaseReviewAuditAttributionSummary[];
  historical_audit_actions: ReleaseReviewAuditActionSummary[];
}

export interface ResearchAuditResponse {
  supported: boolean;
  storage_mode: string;
  market_scope: string;
  active_release_id: string | null;
  runtime_probability_mode: string;
  runtime_release_status: string;
  latest_snapshot_date: string | null;
  latest_replay_run_id: string | null;
  latest_release_review: ReleaseReviewArtifactSummary | null;
  note: string;
  releases: ModelReleaseRecord[];
  replay_runs: HistoricalReplayRunRecord[];
  snapshots: PredictionSnapshotRecord[];
}
