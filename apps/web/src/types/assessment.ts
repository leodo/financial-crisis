import type {
  ActionabilityBlock,
  DataMode,
  DataQualitySummary,
  DecisionPosture,
  EventConfirmationState,
  FreshnessStatus,
  JpyCarryState,
  ProbabilityBlock,
  QualityGrade,
  RiskContributor,
  TimeToRiskBucket,
  UserRiskProfile,
} from "./common";
import type { BacktestPerformanceSummary } from "./backtest";

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

export interface PositionGuidanceGovernance {
  system_budget_only: boolean;
  auto_execution_allowed: boolean;
  manual_confirmation_required: boolean;
  policy_change_requires_release_review: boolean;
  policy_change_requires_go_no_go: boolean;
  required_operator_checks: string[];
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
  governance: PositionGuidanceGovernance;
}

export interface RuntimeMetadata {
  data_mode: DataMode;
  generated_at: string;
  requested_as_of_date: string;
  latest_observation_at: string | null;
  latest_observation_lag_days: number | null;
  latest_observation_lag_business_days: number | null;
  latest_key_indicator_at: string | null;
  latest_key_indicator_lag_days: number | null;
  latest_key_indicator_lag_business_days: number | null;
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
  lag_business_days: number | null;
  stale_threshold_days: number;
  status: FreshnessStatus;
  note: string;
  lineage?: KeyIndicatorLineage | null;
}

export type KeyIndicatorLineageEvidenceLevel =
  | "run_raw_observation"
  | "raw_observation"
  | "observation_only"
  | "missing";

export interface KeyIndicatorLineage {
  evidence_level: KeyIndicatorLineageEvidenceLevel;
  note: string;
  raw_payload_id: string | null;
  run_id: string | null;
  run_status: string | null;
  fetched_at: string | null;
  records_written: number | null;
  response_hash: string | null;
  raw_file_path: string | null;
}

export interface EventSignalSummary {
  event_type: string;
  level: import("./common").RiskLevel;
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

export interface UserRiskPreferences {
  profile: UserRiskProfile;
  cash_floor_pct: number;
  max_equity_cap_pct: number;
  max_leverage_pct: number;
  option_overlay_preference_pct: number;
  allow_aggressive_reentry: boolean;
  note: string;
}

export interface ActionEvidenceBreakdown {
  score: number;
  data_quality_component: number;
  breadth_component: number;
  agreement_component: number;
  data_quality_weight: number;
  breadth_weight: number;
  agreement_high_component: number;
  agreement_low_component: number;
  breadth_score: number;
  structural_trigger_agreement: boolean;
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
  action_evidence?: ActionEvidenceBreakdown;
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
  replay_run_id?: string | null;
  feature_snapshot_id?: string | null;
  history_source?: string | null;
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

export interface HistoryProvenanceSourceSummary {
  source_id: string;
  count: number;
  latest_as_of_date: string | null;
  note: string;
}

export interface HistoryProvenanceSummary {
  evidence_tier: string;
  dominant_source: string;
  total_points: number;
  feature_backed_points: number;
  reused_feature_snapshot_points: number;
  raw_observation_points: number;
  snapshot_bridge_points: number;
  runtime_only_points: number;
  latest_feature_backed_date: string | null;
  latest_reused_feature_snapshot_date: string | null;
  latest_raw_observation_date: string | null;
  latest_snapshot_bridge_date: string | null;
  latest_replay_run_id: string | null;
  note: string;
  sources: HistoryProvenanceSourceSummary[];
}

export interface ScenarioDataCoverageRecord {
  scenario_id: string;
  scenario_label: string;
  recommended_role: string;
  coverage_grade: string;
  point_in_time_mode: string;
  usable_for_main_training: boolean;
  usable_for_extension_training: boolean;
  usable_for_protected_stress: boolean;
  usable_for_historical_analog: boolean;
  free_sources: string[];
  current_status: string;
  blocking_gaps: string[];
}

export interface ScenarioDataCoverageCatalog {
  catalog_id: string;
  scenario_catalog_id: string;
  market_scope: string;
  note: string;
  source: string;
  warning: string | null;
  records: ScenarioDataCoverageRecord[];
}

export interface AssessmentMethodResponse {
  method: AssessmentMethodVersions;
  note: string;
  history_provenance: HistoryProvenanceSummary;
  protected_stress_window_catalog: ProtectedStressWindowCatalog;
  scenario_data_coverage_catalog: ScenarioDataCoverageCatalog;
  runtime_thresholds: RuntimeThresholdDiagnostics;
}
