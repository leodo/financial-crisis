export interface LeadtimeMetricRow {
  metric: string;
  baseline: number | null;
  candidate: number | null;
  delta: number | null;
}

export interface LeadtimeRuntimeRow {
  horizon_days: number;
  baseline_diagnosis: string | null;
  candidate_diagnosis: string | null;
  baseline_threshold: number | null;
  candidate_threshold: number | null;
  baseline_early_warning_regime: string | null;
  candidate_early_warning_regime: string | null;
  baseline_early_warning_avg_probability: number | null;
  candidate_early_warning_avg_probability: number | null;
  baseline_normal_avg_probability: number | null;
  candidate_normal_avg_probability: number | null;
  baseline_early_warning_gap_vs_normal: number | null;
  candidate_early_warning_gap_vs_normal: number | null;
  baseline_floor_gap: number | null;
  candidate_floor_gap: number | null;
  baseline_threshold_hit_rate: number | null;
  candidate_threshold_hit_rate: number | null;
}

export interface LeadtimeGapRow {
  scenario_id: string;
  name: string;
  outcome: string | null;
  signal_source: string | null;
  baseline_lead_time_days: number | null;
  candidate_lead_time_days: number | null;
  baseline_actionable_lead_time_days: number | null;
  candidate_actionable_lead_time_days: number | null;
  actionable_delta_days: number | null;
}

export interface LeadtimeFocusRow {
  scenario_id: string;
  name: string;
  outcome: string | null;
  baseline_primary_failure_mode: string | null;
  candidate_primary_failure_mode: string | null;
  baseline_actionable_point_count: number | null;
  candidate_actionable_point_count: number | null;
  baseline_runtime_floor_hit_point_count: number | null;
  candidate_runtime_floor_hit_point_count: number | null;
  baseline_dominant_runtime_block: string | null;
  baseline_dominant_runtime_block_count: number | null;
  candidate_dominant_runtime_block: string | null;
  candidate_dominant_runtime_block_count: number | null;
  baseline_dominant_continuity_facet: string | null;
  baseline_dominant_continuity_facet_count: number | null;
  candidate_dominant_continuity_facet: string | null;
  candidate_dominant_continuity_facet_count: number | null;
  baseline_first_runtime_floor_hit_without_l3_reason: string | null;
  candidate_first_runtime_floor_hit_without_l3_reason: string | null;
  first_block_date: string | null;
  first_baseline_block_category: string | null;
  first_candidate_block_category: string | null;
  first_baseline_block_reason: string | null;
  first_candidate_block_reason: string | null;
}

export interface LeadtimeCountRow {
  scenario_id: string;
  name: string;
  category: string;
  baseline_count: number;
  candidate_count: number;
  delta: number;
}

export interface LeadtimeWorkstreamRow {
  workstream: string;
  scenario_count: number;
  protected_count: number;
  scenarios: string | null;
  scenario_families: string | null;
  training_roles: string | null;
  baseline_gate_gap_profiles: string | null;
  candidate_gate_gap_profiles: string | null;
  baseline_gate_gap_points: string | null;
  candidate_gate_gap_points: string | null;
  suggested_review: string | null;
}

export interface LeadtimeAttributionRow {
  workstream: string;
  attribution: string;
  scenario_count: number;
  protected_count: number;
  explanation: string | null;
}

export interface LeadtimeActionRow {
  workstream: string;
  attribution: string;
  action_type: string;
  scenario_count: number;
  protected_count: number;
  recommendation: string | null;
}

export interface LeadtimeAuditArtifactSummary {
  generated_at: string;
  source: string;
  release_review_artifact: string;
  baseline_release_id: string;
  candidate_release_id: string;
  market_scope: string;
  history_mode: string;
  reviewed_at: string | null;
  metric_rows: LeadtimeMetricRow[];
  runtime_rows: LeadtimeRuntimeRow[];
  leadtime_gap_rows: LeadtimeGapRow[];
  focus_rows: LeadtimeFocusRow[];
  block_mix_rows: LeadtimeCountRow[];
  continuity_facet_rows: LeadtimeCountRow[];
  workstream_rows: LeadtimeWorkstreamRow[];
  attribution_rows: LeadtimeAttributionRow[];
  action_rows: LeadtimeActionRow[];
  takeaways: string[];
}
