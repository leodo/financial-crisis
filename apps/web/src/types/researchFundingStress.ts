export interface FundingStressCountSummary {
  value: string;
  count: number;
}

export interface FundingStressMaxSummary {
  value: number | null;
  date: string | null;
}

export interface FundingStressHitSummary {
  hit_count: number;
  segment_count: number;
  max_streak: number;
  first_hit_date: string | null;
  last_hit_date: string | null;
  max_streak_start: string | null;
  max_streak_end: string | null;
}

export interface FundingStressNearThresholdSummary {
  count: number;
  first_date: string | null;
  last_date: string | null;
  max_value: number | null;
  min_gap_to_threshold: number | null;
}

export interface FundingStressThresholdSummary {
  baseline_20d: number | null;
  candidate_20d: number | null;
  baseline_60d: number | null;
  candidate_60d: number | null;
}

export interface FundingStressGroupSummary {
  label: string;
  row_count: number;
  avg_baseline_p20d: number | null;
  avg_candidate_p20d: number | null;
  avg_delta_p20d: number | null;
  avg_candidate_p60d: number | null;
  candidate_max_p20d: FundingStressMaxSummary;
  candidate_max_p60d: FundingStressMaxSummary;
  candidate_hit_20d: FundingStressHitSummary;
  candidate_hit_60d: FundingStressHitSummary;
  near_candidate_20d_5pp: FundingStressNearThresholdSummary;
  near_candidate_60d_5pp: FundingStressNearThresholdSummary;
  split_counts: FundingStressCountSummary[];
  phase_counts: FundingStressCountSummary[];
  action_level_counts: FundingStressCountSummary[];
}

export interface FundingStressDatasetEvidence {
  split_counts: FundingStressCountSummary[];
  regime_20d_counts: FundingStressCountSummary[];
  regime_60d_counts: FundingStressCountSummary[];
  action_phase_counts: FundingStressCountSummary[];
  action_level_counts: FundingStressCountSummary[];
  protected_row_count: number;
  label_20d_count: number;
  label_60d_count: number;
  prepare_episode_count: number;
  hedge_episode_count: number;
  avg_coverage_score: number | null;
  feature_name_count: number;
  raw_feature_name_count: number;
  resolved_feature_name_count: number;
  available_relevant_features: string[];
  missing_relevant_features: string[];
}

export interface FundingStressProbabilityEvidence {
  full_window: FundingStressGroupSummary;
  primary_phase: FundingStressGroupSummary;
  prepare_primary: FundingStressGroupSummary;
  hedge_primary: FundingStressGroupSummary;
  late_validation: FundingStressGroupSummary;
  positive_window_20d: FundingStressGroupSummary;
  pre_warning_buffer_20d: FundingStressGroupSummary;
  normal_20d: FundingStressGroupSummary;
}

export interface FundingStressFeatureGap {
  feature: string;
  left_group: string;
  right_group: string;
  left_mean: number | null;
  right_mean: number | null;
  mean_delta: number | null;
  standardized_gap: number | null;
}

export interface FundingStressBaseContribution {
  name: string;
  mean_raw_value: number | null;
  mean_normalized_value: number | null;
  mean_weight: number | null;
  mean_contribution: number | null;
  sum_contribution: number | null;
  count: number;
}

export interface FundingStressOverlayContribution {
  family_id: string;
  gate_feature: string;
  mean_gate_value: number | null;
  mean_gate: number | null;
  mean_blend: number | null;
  mean_overlay_probability: number | null;
  mean_contribution: number | null;
  sum_contribution: number | null;
  count: number;
}

export interface FundingStressContributionGroup {
  label: string;
  horizon_days: number;
  row_count: number;
  top_positive_base: FundingStressBaseContribution[];
  top_negative_base: FundingStressBaseContribution[];
  top_absolute_base: FundingStressBaseContribution[];
  overlay_contributions: FundingStressOverlayContribution[];
}

export interface FundingStressFeatureContext {
  separation: Record<string, FundingStressFeatureGap[]>;
  candidate_resolved_relevant_features: FundingStressBaseContribution[];
  candidate_absolute_contributions: Record<string, FundingStressContributionGroup>;
}

export interface FundingStressScenarioSummary {
  scenario_id: string;
  label: string;
  family: string;
  pre_warning_start: string;
  crisis_start: string;
  acute_start: string | null;
  crisis_end: string;
  training_role: string;
  protected_window: boolean;
  protected_action_levels: string[];
  default_horizon_roles: number[];
}

export interface FundingStressCoverageSummary {
  coverage_grade: string;
  recommended_role: string;
  point_in_time_mode: string;
  free_sources: string[];
  blocking_gaps: string[];
}

export interface FundingStressDiagnosis {
  primary_class: string;
  trainability_class: string;
  family_context_class: string;
  candidate_margin_class: string;
  reasons: string[];
  next_actions: string[];
}

export interface FundingStressAuditArtifactSummary {
  generated_at: string;
  source: string;
  compare_path: string;
  slice_path: string;
  candidate_scored_slice_path: string | null;
  baseline_release_id: string;
  candidate_release_id: string;
  market_scope: string;
  scenario: FundingStressScenarioSummary;
  coverage: FundingStressCoverageSummary;
  dataset_key: string;
  from_date: string;
  to_date: string;
  row_count: number;
  thresholds: FundingStressThresholdSummary;
  dataset_evidence: FundingStressDatasetEvidence;
  probability_evidence: FundingStressProbabilityEvidence;
  feature_context: FundingStressFeatureContext;
  diagnosis: FundingStressDiagnosis;
}
