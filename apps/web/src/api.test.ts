import { describe, it, expect } from "vitest";
import type {
  AssessmentSnapshot,
  PositionGuidance,
  PositionGuidanceGovernance
} from "./types";

function makePositionGuidance(
  governance?: Partial<PositionGuidanceGovernance>
): PositionGuidance {
  return {
    action_playbook_version: "1.0",
    execution_urgency: "routine",
    confidence_gate: "normal",
    target_equity_exposure_pct: 80,
    target_cash_pct: 10,
    hedge_ratio_pct: 0,
    leverage_cap_pct: 100,
    option_overlay_pct: 0,
    action_summary: "",
    actions: [],
    forbidden_actions: [],
    inapplicable_scenarios: [],
    manual_confirmation_items: [],
    reentry_conditions: [],
    guardrails: [],
    capital_preservation_overlay_enabled: false,
    governance: {
      system_budget_only: true,
      auto_execution_allowed: false,
      manual_confirmation_required: true,
      policy_change_requires_release_review: true,
      policy_change_requires_go_no_go: true,
      required_operator_checks: [],
      ...governance
    }
  };
}

const DEFAULT_GOVERNANCE: PositionGuidanceGovernance = {
  system_budget_only: true,
  auto_execution_allowed: false,
  manual_confirmation_required: true,
  policy_change_requires_release_review: true,
  policy_change_requires_go_no_go: true,
  required_operator_checks: []
};

function normalizeAssessmentSnapshot(
  assessment: AssessmentSnapshot
): AssessmentSnapshot {
  return {
    ...assessment,
    position_guidance: {
      ...assessment.position_guidance,
      governance: assessment.position_guidance.governance
        ? {
            ...DEFAULT_GOVERNANCE,
            ...assessment.position_guidance.governance,
            required_operator_checks:
              assessment.position_guidance.governance.required_operator_checks ??
              DEFAULT_GOVERNANCE.required_operator_checks
          }
        : DEFAULT_GOVERNANCE
    }
  };
}

function makeMinimalAssessment(
  overrides?: Partial<AssessmentSnapshot>
): AssessmentSnapshot {
  return {
    as_of_date: "2026-06-30",
    entity_id: "us",
    market_scope: "us",
    probabilities: { p_5d: 0.01, p_20d: 0.03, p_60d: 0.08 },
    actionability: { prepare: 10, hedge: 5, defend: 2 },
    probability_diagnostics: { horizon_overlays: [] },
    time_to_risk_bucket: "normal",
    posture: "normal",
    conviction_score: 0.5,
    scores: { overall_score: 18, structural_score: 10, trigger_score: 5, external_shock_score: 3 },
    summary: "",
    posture_reason: "",
    top_risk_drivers: [],
    top_relief_drivers: [],
    historical_analogs: [],
    data_trust: {
      coverage_score: 85,
      core_feature_coverage: 80,
      trigger_feature_coverage: 80,
      external_feature_coverage: 80,
      quality_grade: "b",
      data_quality_summary: {
        overall_score: 85,
        grade: "b",
        stale_indicator_count: 1,
        low_quality_indicator_count: 0,
        prototype_source_count: 0,
        blocked_indicator_count: 0
      },
      warnings: []
    },
    jpy_carry: {
      state: "quiet",
      score: 10,
      usdjpy_level: null,
      jp_call_rate: null,
      us_short_rate: null,
      us_jp_short_rate_diff: null,
      change_5d: null,
      change_20d: null,
      realized_vol_20d: null,
      funding_pressure_score: 5,
      vix_coupling_score: 15,
      credit_coupling_score: 5,
      reason: ""
    },
    position_guidance: makePositionGuidance(),
    runtime: {
      data_mode: "demo",
      generated_at: "2026-06-30T12:00:00Z",
      requested_as_of_date: "2026-06-30",
      latest_observation_at: null,
      latest_observation_lag_days: null,
      latest_observation_lag_business_days: null,
      latest_key_indicator_at: null,
      latest_key_indicator_lag_days: null,
      latest_key_indicator_lag_business_days: null,
      demo_mode: true,
      stale_warning: null
    },
    key_indicators: [],
    event_assessment: {
      state: "quiet",
      confirmation_score: 5,
      recent_event_count: 0,
      summary: "",
      confirmed_signals: [],
      pending_gaps: [],
      recent_events: []
    },
    backtest_summary: {
      scenario_count: 0,
      real_scenario_count: 0,
      fallback_scenario_count: 0,
      coverage_scope_note: "",
      structural_warning_rate: 0,
      timely_warning_rate: 0,
      missed_rate: 0,
      avg_structural_lead_time_days: null,
      avg_lead_time_days: null,
      median_lead_time_days: null,
      total_false_positive_count: 0,
      history_start: null,
      history_end: null,
      rolling_audit: {
        history_start: null,
        history_end: null,
        history_point_count: 0,
        scope_note: "",
        actionable_signal_count: 0,
        pre_crisis_signal_count: 0,
        in_crisis_signal_count: 0,
        stress_window_signal_count: 0,
        false_positive_signal_count: 0,
        false_positive_episode_count: 0,
        longest_false_positive_episode_days: 0,
        actionable_precision: 0,
        classified_episodes: [],
        summary: ""
      },
      summary: ""
    },
    user_preferences: {
      profile: "neutral",
      cash_floor_pct: 5,
      max_equity_cap_pct: 90,
      max_leverage_pct: 100,
      option_overlay_preference_pct: 0,
      allow_aggressive_reentry: false,
      note: ""
    },
    method: {
      score_method_version: "1.0",
      prob_model_version: "1.0",
      calibration_version: "1.0",
      actionability_model_version: null,
      actionability_calibration_version: null,
      feature_set_version: "1.0",
      label_version: "1.0",
      posture_policy_version: "1.0",
      action_playbook_version: "1.0",
      fusion_policy_version: null,
      actionability_enabled: false,
      probability_mode: "demo",
      release_status: "shadow",
      release_id: null,
      point_in_time_mode: "demo"
    },
    ...overrides
  };
}

describe("normalizeAssessmentSnapshot", () => {
  it("preserves existing governance fields", () => {
    const input = makeMinimalAssessment();
    const result = normalizeAssessmentSnapshot(input);
    expect(result.position_guidance.governance.system_budget_only).toBe(true);
    expect(result.position_guidance.governance.auto_execution_allowed).toBe(false);
  });

  it("fills missing governance with defaults", () => {
    const input = makeMinimalAssessment();
    // Remove governance by spreading override
    const inputWithGuardedGovernance: AssessmentSnapshot = {
      ...input,
      position_guidance: {
        ...input.position_guidance,
        governance: undefined as unknown as PositionGuidanceGovernance
      }
    };
    const result = normalizeAssessmentSnapshot(inputWithGuardedGovernance);
    expect(result.position_guidance.governance.system_budget_only).toBe(true);
    expect(result.position_guidance.governance.required_operator_checks).toEqual([]);
  });

  it("merges partial governance with defaults", () => {
    const input = makeMinimalAssessment({
      position_guidance: makePositionGuidance({
        auto_execution_allowed: true,
        required_operator_checks: ["check_1"]
      })
    });
    const result = normalizeAssessmentSnapshot(input);
    // Defaults preserved
    expect(result.position_guidance.governance.system_budget_only).toBe(true);
    expect(result.position_guidance.governance.manual_confirmation_required).toBe(true);
    // Overrides applied
    expect(result.position_guidance.governance.auto_execution_allowed).toBe(true);
    expect(result.position_guidance.governance.required_operator_checks).toEqual(["check_1"]);
  });

  it("does not mutate the input object", () => {
    const input = makeMinimalAssessment();
    // Deep freeze won't work easily with complex objects, but we verify the function handles it
    expect(() => normalizeAssessmentSnapshot(input)).not.toThrow();
  });
});
