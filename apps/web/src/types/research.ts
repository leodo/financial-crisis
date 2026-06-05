import type {
  AssessmentMethodVersions,
  ProtectedStressWindowCatalog,
  RuntimeThresholdDiagnostics,
} from "./assessment";

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

export type {
  AssessmentMethodVersions,
  ProtectedStressWindowCatalog,
  RuntimeThresholdDiagnostics,
};
