import type { DecisionPosture } from "./common";
import type { BacktestScenarioSummary } from "./risk";

export interface BacktestWindowPoint {
  as_of_date: string;
  overall_score: number;
  p_5d: number;
  p_20d: number;
  p_60d: number;
  posture: DecisionPosture;
  crisis_window_open: boolean;
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

export interface BacktestPerformanceSummary {
  scenario_count: number;
  real_scenario_count: number;
  fallback_scenario_count: number;
  coverage_scope_note: string;
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

export type { BacktestScenarioSummary };
