import type {
  AssessmentMethodVersions,
  HistoryProvenanceSummary,
  ProtectedStressWindowCatalog,
  RuntimeThresholdDiagnostics,
} from "./assessment";
import type { FundingStressAuditArtifactSummary } from "./researchFundingStress";
import type { LeadtimeAuditArtifactSummary } from "./researchLeadtime";

export type {
  FundingStressAuditArtifactSummary,
  FundingStressBaseContribution,
  FundingStressFeatureGap,
  FundingStressOverlayContribution,
} from "./researchFundingStress";
export type { LeadtimeAuditArtifactSummary, LeadtimeFocusRow, LeadtimeRuntimeRow } from "./researchLeadtime";

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

export interface PredictionSnapshotAuditSummary {
  role: string;
  active_release_snapshot_count: number;
  other_release_snapshot_count: number;
  formal_probability_snapshot_count: number;
  heuristic_probability_snapshot_count: number;
  note: string;
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

export interface ReleaseReviewScenarioCoverageCatalogSummary {
  catalog_id: string;
  scenario_catalog_id: string;
  market_scope: string;
  source: string;
  warning: string | null;
  backtest_scenario_count: number;
  covered_backtest_scenario_count: number;
  focus_scenario_count: number;
  covered_focus_scenario_count: number;
  main_training_eligible_count: number;
  extension_training_eligible_count: number;
  protected_stress_eligible_count: number;
  historical_analog_eligible_count: number;
}

export interface ReleaseReviewScenarioCoverageSummary {
  scenario_id: string;
  scenario_name: string;
  scenario_family: string;
  training_role: string;
  protected_window: boolean;
  in_backtest_comparison: boolean;
  in_focus_review: boolean;
  recommended_role: string;
  coverage_grade: string;
  point_in_time_mode: string;
  current_status: string;
  blocking_gaps: string[];
  free_sources: string[];
  usable_for_main_training: boolean;
  usable_for_extension_training: boolean;
  usable_for_protected_stress: boolean;
  usable_for_historical_analog: boolean;
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
  scenario_coverage_catalog: ReleaseReviewScenarioCoverageCatalogSummary;
  scenario_coverages: ReleaseReviewScenarioCoverageSummary[];
}

export interface ScenarioPackAuditBlockerCountSummary {
  key: string;
  count: number;
  scenarios: string[];
}

export interface ScenarioPackAuditScenarioSummary {
  scenario_id: string;
  scenario_label: string;
  family: string;
  training_role: string;
  recommended_role: string;
  coverage_grade: string;
  point_in_time_mode: string;
  current_status: string;
  protected_window: boolean;
  free_sources: string[];
  blocking_gaps: string[];
  outcome: string | null;
  signal_source: string | null;
  baseline_lead_time_days: number | null;
  candidate_lead_time_days: number | null;
  baseline_actionable_lead_time_days: number | null;
  candidate_actionable_lead_time_days: number | null;
  primary_workstream: string | null;
  suggested_review: string | null;
  candidate_primary_failure_mode: string | null;
  compare_status: string;
  compare_dataset_key: string | null;
  attempted_datasets: string[];
  row_count: number;
  positive_window_retention_20d: number | null;
  overall_avg_delta_p_20d: number | null;
  blocker_class: string;
  takeaway: string;
}

export interface ScenarioPackAuditArtifactSummary {
  generated_at: string;
  source: string;
  baseline_release_id: string;
  candidate_release_id: string;
  history_mode: string;
  market_scope: string;
  compare_ok_count: number;
  compare_missing_count: number;
  blocker_counts: ScenarioPackAuditBlockerCountSummary[];
  scenario_summaries: ScenarioPackAuditScenarioSummary[];
}

export interface RateShockAuditThresholdSummary {
  baseline_20d: number | null;
  candidate_20d: number | null;
  baseline_60d: number | null;
  candidate_60d: number | null;
}

export interface RateShockAuditHitSummary {
  hit_count: number;
  segment_count: number;
  max_streak: number;
  first_hit_date: string | null;
  last_hit_date: string | null;
  max_streak_start: string | null;
  max_streak_end: string | null;
}

export interface RateShockAuditWindowSummary {
  row_count: number;
  avg_delta_p_20d: number | null;
  avg_abs_delta_p_20d: number | null;
  avg_delta_p_60d: number | null;
  avg_abs_delta_p_60d: number | null;
  baseline_hit_rate_20d: number | null;
  candidate_hit_rate_20d: number | null;
  baseline_hit_rate_60d: number | null;
  candidate_hit_rate_60d: number | null;
}

export interface RateShockAuditCompareSummary {
  baseline_hit_count_20d: number;
  candidate_hit_count_20d: number;
  baseline_hit_count_60d: number;
  candidate_hit_count_60d: number;
  baseline_max_p_20d: number | null;
  baseline_max_p_20d_date: string | null;
  candidate_max_p_20d: number | null;
  candidate_max_p_20d_date: string | null;
  baseline_max_p_60d: number | null;
  baseline_max_p_60d_date: string | null;
  candidate_max_p_60d: number | null;
  candidate_max_p_60d_date: string | null;
  overall_window: RateShockAuditWindowSummary;
  hedge_window: RateShockAuditWindowSummary;
  positive_window_20d: RateShockAuditWindowSummary;
}

export interface RateShockAuditSplitSummary {
  split_name: string;
  row_count: number;
}

export interface RateShockAuditGroupSummary {
  label: string;
  row_count: number;
  baseline_avg_p_20d: number | null;
  candidate_avg_p_20d: number | null;
  avg_delta_p_20d: number | null;
  baseline_avg_gap_to_threshold_20d: number | null;
  candidate_avg_gap_to_threshold_20d: number | null;
  baseline_avg_p_60d: number | null;
  candidate_avg_p_60d: number | null;
  avg_delta_p_60d: number | null;
  baseline_avg_gap_to_threshold_60d: number | null;
  candidate_avg_gap_to_threshold_60d: number | null;
  baseline_hit_rate_20d: number | null;
  candidate_hit_rate_20d: number | null;
  baseline_hit_rate_60d: number | null;
  candidate_hit_rate_60d: number | null;
  baseline_hit_20d: RateShockAuditHitSummary;
  candidate_hit_20d: RateShockAuditHitSummary;
  baseline_hit_60d: RateShockAuditHitSummary;
  candidate_hit_60d: RateShockAuditHitSummary;
  baseline_near_threshold_20d_within_5pp_count: number;
  candidate_near_threshold_20d_within_5pp_count: number;
  baseline_near_threshold_60d_within_5pp_count: number;
  candidate_near_threshold_60d_within_5pp_count: number;
  baseline_max_p_20d: number | null;
  baseline_max_p_20d_date: string | null;
  candidate_max_p_20d: number | null;
  candidate_max_p_20d_date: string | null;
  baseline_max_p_60d: number | null;
  baseline_max_p_60d_date: string | null;
  candidate_max_p_60d: number | null;
  candidate_max_p_60d_date: string | null;
}

export interface RateShockAuditContinuityFocus {
  prepare_primary: RateShockAuditGroupSummary;
  hedge_primary: RateShockAuditGroupSummary;
  primary_phase: RateShockAuditGroupSummary;
  late_validation: RateShockAuditGroupSummary;
}

export interface RateShockAuditArtifactSummary {
  generated_at: string;
  source: string;
  compare_path: string;
  slice_path: string;
  baseline_release_id: string;
  candidate_release_id: string;
  dataset_key: string;
  scenario_id: string;
  from_date: string;
  to_date: string;
  thresholds: RateShockAuditThresholdSummary;
  compare_summary: RateShockAuditCompareSummary;
  split_counts: RateShockAuditSplitSummary[];
  phase_summaries: RateShockAuditGroupSummary[];
  action_level_summaries: RateShockAuditGroupSummary[];
  continuity_focus: RateShockAuditContinuityFocus;
}

export interface DatasetSummaryArtifactDatasetRecord {
  dataset_id: string;
  dataset_version: string;
  market_scope: string;
  feature_set_version: string;
  label_version: string;
  scenario_set_version: string;
  point_in_time_mode: string;
  from_date: string;
  to_date: string;
  train_end_date: string;
  calibration_end_date: string;
  evaluation_start_date: string;
  row_count: number;
  note: string | null;
  created_at: string;
}

export interface DatasetSummaryArtifactSplitSummary {
  split_name: string;
  row_count: number;
  positive_5d_count: number;
  positive_20d_count: number;
  positive_60d_count: number;
  prepare_primary_count: number;
  hedge_primary_count: number;
  defend_primary_count: number;
  late_validation_row_count: number;
  protected_row_count: number;
  avg_coverage_score: number;
  scenario_count: number;
}

export interface DatasetSummaryArtifactScenarioSummary {
  scenario_id: string;
  label: string | null;
  row_count: number;
  split_count: number;
  first_as_of_date: string;
  last_as_of_date: string;
  family: string | null;
  training_role: string | null;
  protected_window: boolean | null;
  episode_template_id: string | null;
  default_horizon_roles: number[];
  coverage_recommended_role: string | null;
  coverage_grade: string | null;
  coverage_point_in_time_mode: string | null;
  coverage_current_status: string | null;
  coverage_blocking_gaps: string[];
  coverage_free_sources: string[];
  usable_for_main_training: boolean | null;
  usable_for_extension_training: boolean | null;
  usable_for_protected_stress: boolean | null;
  usable_for_historical_analog: boolean | null;
}

export interface DatasetSummaryArtifactCoverageCatalog {
  catalog_id: string;
  scenario_catalog_id: string;
  market_scope: string;
  source: string;
  warning: string | null;
  dataset_intent: string;
  aligned_scenario_count: number;
  total_scenario_count: number;
  main_training_eligible_count: number;
  extension_training_eligible_count: number;
  protected_stress_eligible_count: number;
  historical_analog_eligible_count: number;
}

export interface DatasetSummaryArtifactSummary {
  generated_at: string;
  source: string;
  dataset_key: string;
  dataset: DatasetSummaryArtifactDatasetRecord;
  split_summaries: DatasetSummaryArtifactSplitSummary[];
  scenario_summaries: DatasetSummaryArtifactScenarioSummary[];
  coverage_catalog: DatasetSummaryArtifactCoverageCatalog;
  recommendation: string;
}

export interface WorkstreamAuditSummary {
  workstream: string;
  scenario_count: number;
  scenarios: string[];
  covered_scenario_count: number;
  missing_scenario_count: number;
  missing_scenarios: string[];
  training_roles: string[];
  scenario_families: string[];
  total_rows: number;
  total_positive_label_5d_count: number;
  total_positive_label_20d_count: number;
  total_positive_label_60d_count: number;
  total_prepare_primary_count: number;
  total_hedge_primary_count: number;
  total_defend_primary_count: number;
  total_protected_row_count: number;
  avg_coverage_score: number | null;
  avg_core_feature_coverage: number | null;
  avg_trigger_feature_coverage: number | null;
  avg_external_feature_coverage: number | null;
}

export interface WorkstreamAuditScenarioSummary {
  scenario_id: string;
  scenario_name: string;
  workstream: string;
  scenario_family: string;
  training_role: string;
  protected_window: boolean;
  suggested_review: string | null;
  window_start: string;
  window_end: string;
  crisis_start: string | null;
  crisis_end: string | null;
  slice_status: string;
  slice_selector_reason: string;
  attempted_datasets: string[];
  dataset_key: string;
  feature_set_version: string;
  label_version: string;
  row_count: number;
  split_counts: string[];
  quality_counts: string[];
  regime20_counts: string[];
  regime60_counts: string[];
  action_phase_counts: string[];
  primary_action_level_counts: string[];
  avg_coverage_score: number | null;
  avg_core_feature_coverage: number | null;
  avg_trigger_feature_coverage: number | null;
  avg_external_feature_coverage: number | null;
  positive_label_5d_count: number;
  positive_label_20d_count: number;
  positive_label_60d_count: number;
  prepare_primary_count: number;
  hedge_primary_count: number;
  defend_primary_count: number;
  protected_row_count: number;
  feature_name_count: number;
  feature_names: string[];
  slice_json_path: string;
}

export interface WorkstreamAuditArtifactSummary {
  generated_at: string;
  source: string;
  review_report_path: string;
  baseline_release_id: string;
  candidate_release_id: string;
  history_mode: string;
  market_scope: string;
  dataset_key: string;
  dataset_id: string;
  dataset_version: string;
  requested_workstreams: string[];
  workstream_summaries: WorkstreamAuditSummary[];
  scenario_summaries: WorkstreamAuditScenarioSummary[];
}

export interface CooldownAuditNoGoReason {
  code: string;
  summary: string;
  evidence: Record<string, unknown>;
}

export interface CooldownAuditRuntimeRow {
  horizon_days: number;
  baseline_diagnosis: string | null;
  candidate_diagnosis: string | null;
  candidate_cooldown_minus_positive: number | null;
  candidate_cooldown_minus_normal: number | null;
  comparison: Record<string, unknown>;
}

export interface CooldownAuditFalsePositiveEpisode {
  start_date: string;
  end_date: string;
  duration_days: number;
  signal_count: number;
  classification: string;
  note: string;
}

export interface CooldownAuditEpisodeRegression {
  kind: string;
  episode: CooldownAuditFalsePositiveEpisode;
  overlapping_baseline_episodes: CooldownAuditFalsePositiveEpisode[];
}

export interface CooldownAuditFalsePositiveEpisodes {
  baseline_top: CooldownAuditFalsePositiveEpisode[];
  candidate_top: CooldownAuditFalsePositiveEpisode[];
  candidate_regressions: CooldownAuditEpisodeRegression[];
}

export interface CooldownAuditScenarioFalsePositiveDelta {
  scenario_id: string;
  name: string;
  baseline_false_positive_count: number;
  candidate_false_positive_count: number;
  delta: number;
  outcome: string | null;
}

export interface CooldownAuditArtifactSummary {
  generated_at: string;
  source: string;
  baseline_release_id: string;
  candidate_release_id: string;
  history_mode: string;
  release_review_artifact: string;
  reviewed_at: string | null;
  recommendation: string;
  comparison_metrics: Record<string, unknown>;
  runtime_cooldown_rows: CooldownAuditRuntimeRow[];
  false_positive_episodes: CooldownAuditFalsePositiveEpisodes;
  scenario_false_positive_deltas: CooldownAuditScenarioFalsePositiveDelta[];
  no_go_reasons: CooldownAuditNoGoReason[];
}

export interface PrewarningGapHitSummary {
  hit_count: number;
  segment_count: number;
  max_streak: number;
  first_hit_date: string | null;
  last_hit_date: string | null;
  max_streak_start: string | null;
  max_streak_end: string | null;
}

export interface PrewarningGapDatasetEvidence {
  dataset_key: string | null;
  row_count: number;
  split_counts: string[];
  regime20_counts: string[];
  regime60_counts: string[];
  action_phase_counts: string[];
  primary_action_level_counts: string[];
  label_20d_count: number;
  label_60d_count: number;
  prepare_primary_count: number;
  hedge_primary_count: number;
  protected_row_count: number;
  avg_coverage_score: number | null;
  feature_name_count: number;
}

export interface PrewarningGapProbabilityEvidence {
  compare_status: string | null;
  compare_row_count: number;
  baseline_hit_20d: PrewarningGapHitSummary;
  candidate_hit_20d: PrewarningGapHitSummary;
  baseline_hit_60d: PrewarningGapHitSummary;
  candidate_hit_60d: PrewarningGapHitSummary;
  candidate_near_threshold_20d_5pp_count: number;
  candidate_near_threshold_60d_5pp_count: number;
  baseline_max_p_20d: number | null;
  candidate_max_p_20d: number | null;
  baseline_max_p_60d: number | null;
  candidate_max_p_60d: number | null;
  candidate_avg_p_20d: number | null;
  candidate_avg_p_60d: number | null;
  baseline_avg_p_20d: number | null;
  baseline_avg_p_60d: number | null;
  avg_delta_p_20d: number | null;
  avg_delta_p_60d: number | null;
  positive_window_candidate_hit_rate_20d: number | null;
  hedge_window_candidate_hit_rate_20d: number | null;
}

export interface PrewarningGapDiagnosis {
  gap_class: string;
  reasons: string[];
  next_action: string | null;
}

export interface PrewarningGapScenarioSummary {
  scenario_id: string;
  scenario_label: string;
  family: string;
  training_role: string;
  protected_window: boolean;
  pre_warning_start: string;
  crisis_start: string;
  crisis_end: string;
  coverage_grade: string;
  coverage_role: string;
  coverage_pit_mode: string;
  free_sources: string[];
  blocking_gaps: string[];
  dataset_evidence: PrewarningGapDatasetEvidence;
  probability_evidence: PrewarningGapProbabilityEvidence;
  diagnosis: PrewarningGapDiagnosis;
}

export interface PrewarningGapAuditArtifactSummary {
  generated_at: string;
  source: string;
  baseline_release_id: string;
  candidate_release_id: string;
  market_scope: string;
  scenario_count: number;
  gap_counts: string[];
  scenario_summaries: PrewarningGapScenarioSummary[];
}

export interface ResearchAuditResponse {
  supported: boolean;
  storage_mode: string;
  market_scope: string;
  active_release_id: string | null;
  runtime_probability_mode: string;
  runtime_release_status: string;
  history_provenance: HistoryProvenanceSummary;
  prediction_snapshot_audit: PredictionSnapshotAuditSummary;
  latest_snapshot_date: string | null;
  latest_replay_run_id: string | null;
  latest_release_review: ReleaseReviewArtifactSummary | null;
  latest_scenario_pack_audit: ScenarioPackAuditArtifactSummary | null;
  latest_workstream_audit: WorkstreamAuditArtifactSummary | null;
  latest_rate_shock_audit: RateShockAuditArtifactSummary | null;
  latest_prewarning_gap_audit: PrewarningGapAuditArtifactSummary | null;
  latest_funding_stress_audit: FundingStressAuditArtifactSummary | null;
  latest_leadtime_audit: LeadtimeAuditArtifactSummary | null;
  latest_cooldown_audit: CooldownAuditArtifactSummary | null;
  latest_dataset_summaries: DatasetSummaryArtifactSummary[];
  note: string;
  releases: ModelReleaseRecord[];
  replay_runs: HistoricalReplayRunRecord[];
  snapshots: PredictionSnapshotRecord[];
}

export type {
  AssessmentMethodVersions,
  ProtectedStressWindowCatalog,
  RuntimeThresholdDiagnostics,
};
