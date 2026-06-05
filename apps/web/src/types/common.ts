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
