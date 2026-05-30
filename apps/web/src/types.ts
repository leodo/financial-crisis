export type RiskLevel = "normal" | "watch" | "stress" | "warning" | "crisis";
export type QualityGrade = "a" | "b" | "c" | "d" | "f";

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
  quality_grade: QualityGrade;
  contribution: number;
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

export interface BacktestScenarioSummary {
  scenario_id: string;
  name: string;
  region: string;
  crisis_start: string;
  crisis_end: string;
  first_l2_date: string | null;
  first_l3_date: string | null;
  max_level: RiskLevel;
  max_score: number;
  lead_time_days: number | null;
  false_positive_count: number;
  missed: boolean;
  top_contributors: RiskContributor[];
  method_version: string;
}

