mod clauses;
mod counters;
mod preferences;

use fc_domain::{
    ActionabilityBlock, DataTrust, DecisionPosture, EventAssessment, HistoricalAnalog,
    JpyCarrySnapshot, PostureGuidance, ProbabilityBlock, QualityGrade, RiskSnapshot,
    TimeToRiskBucket, UserRiskPreferences,
};

use super::super::{format_probability_threshold, ProbabilityActionThresholds};
use clauses::{
    build_posture_clause_diagnostics, prepare_continuity_bridge_signal,
    prepare_probability_plateau_signal,
};
use counters::{
    prepare_context_confirmation_count_without_events,
    prepare_non_carry_confirmation_count_without_events,
    prepare_non_external_confirmation_count_without_events,
};
use preferences::{adjust_posture_for_preferences, preference_adjustment_code};

#[allow(clippy::too_many_arguments)]
pub(in super::super) fn build_time_to_risk_bucket(
    probabilities: &ProbabilityBlock,
    prepare_reference_p60d: Option<f64>,
    actionability_trigger: Option<&ActionabilityBlock>,
    actionability_support: Option<&ActionabilityBlock>,
    overall_score: f64,
    structural_score: f64,
    trigger_score: f64,
    external_shock_score: f64,
    breadth_score: f64,
    jpy_carry: &JpyCarrySnapshot,
    thresholds: ProbabilityActionThresholds,
) -> TimeToRiskBucket {
    let prepare_p60d = prepare_reference_p60d.unwrap_or(probabilities.p_60d);
    let severe_carry = jpy_carry.score >= 70.0 && jpy_carry.funding_pressure_score >= 55.0;
    let stressed_carry = jpy_carry.score >= 58.0 && jpy_carry.funding_pressure_score >= 48.0;
    let prepare_confirmation_count = prepare_context_confirmation_count_without_events(
        trigger_score,
        external_shock_score,
        breadth_score,
        jpy_carry.funding_pressure_score,
    );
    let prepare_non_external_confirmation_count =
        prepare_non_external_confirmation_count_without_events(
            trigger_score,
            breadth_score,
            jpy_carry.funding_pressure_score,
        );
    let prepare_non_carry_confirmation_count = prepare_non_carry_confirmation_count_without_events(
        trigger_score,
        external_shock_score,
        breadth_score,
    );
    let defend_head_now = actionability_trigger.is_some_and(|scores| {
        scores.defend >= 0.33
            && (trigger_score >= 55.0 || external_shock_score >= 55.0 || breadth_score >= 44.0)
    });
    let hedge_head_weeks = actionability_trigger.is_some_and(|scores| {
        scores.hedge >= 0.34
            && (trigger_score >= 48.0 || external_shock_score >= 48.0 || breadth_score >= 38.0)
    });
    let prepare_head_months = actionability_trigger.is_some_and(|scores| {
        scores.prepare >= 0.38
            && prepare_p60d >= thresholds.downgrade_prepare_p60d()
            && prepare_confirmation_count >= 2
            && (structural_score >= 56.0 || external_shock_score >= 55.0)
    });
    let prepare_continuity_bridge = prepare_continuity_bridge_signal(
        probabilities,
        prepare_reference_p60d,
        actionability_support,
        structural_score,
        trigger_score,
        external_shock_score,
        breadth_score,
    );
    let prepare_probability_plateau = prepare_probability_plateau_signal(
        probabilities,
        prepare_reference_p60d,
        overall_score,
        structural_score,
        trigger_score,
        external_shock_score,
        breadth_score,
        thresholds,
    );

    if (probabilities.p_5d >= thresholds.defend_p5d
        && trigger_score >= 62.0
        && breadth_score >= 48.0)
        || (probabilities.p_20d >= thresholds.severe_now_p20d()
            && trigger_score >= 68.0
            && external_shock_score >= 55.0
            && breadth_score >= 45.0)
        || (severe_carry && external_shock_score >= 55.0 && trigger_score >= 50.0)
        || defend_head_now
    {
        TimeToRiskBucket::Now
    } else if (probabilities.p_20d >= thresholds.hedge_p20d
        && (trigger_score >= 50.0 || external_shock_score >= 50.0)
        && breadth_score >= 38.0)
        || (probabilities.p_60d >= thresholds.elevated_weeks_p60d()
            && structural_score >= 55.0
            && trigger_score >= 55.0
            && breadth_score >= 40.0)
        || (stressed_carry && external_shock_score >= 50.0 && structural_score >= 50.0)
        || hedge_head_weeks
    {
        TimeToRiskBucket::Weeks
    } else if (prepare_p60d >= thresholds.prepare_p60d
        && structural_score >= 58.0
        && prepare_confirmation_count >= 2)
        || (structural_score >= 62.0
            && prepare_confirmation_count >= 2
            && prepare_p60d >= thresholds.downgrade_prepare_p60d())
        || (external_shock_score >= 58.0
            && structural_score >= 54.0
            && prepare_non_external_confirmation_count >= 1
            && probabilities.p_20d >= thresholds.external_prepare_p20d())
        || (stressed_carry
            && structural_score >= 56.0
            && prepare_non_carry_confirmation_count >= 1
            && prepare_p60d >= thresholds.carry_prepare_p60d())
        || prepare_head_months
        || prepare_continuity_bridge
        || prepare_probability_plateau
    {
        TimeToRiskBucket::Months
    } else {
        TimeToRiskBucket::Normal
    }
}

#[allow(clippy::too_many_arguments)]
pub(in super::super) fn build_posture_guidance(
    snapshot: &RiskSnapshot,
    probabilities: &ProbabilityBlock,
    prepare_reference_p60d: Option<f64>,
    actionability_trigger: Option<&ActionabilityBlock>,
    actionability_support: Option<&ActionabilityBlock>,
    conviction_score: f64,
    data_trust: &DataTrust,
    external_shock_score: f64,
    breadth_score: f64,
    analogs: &[HistoricalAnalog],
    jpy_carry: &JpyCarrySnapshot,
    event_assessment: &EventAssessment,
    user_preferences: &UserRiskPreferences,
    thresholds: ProbabilityActionThresholds,
) -> PostureGuidance {
    let severe_quality_block =
        matches!(data_trust.quality_grade, QualityGrade::D | QualityGrade::F);
    let clause_diagnostics = build_posture_clause_diagnostics(
        snapshot,
        probabilities,
        prepare_reference_p60d,
        actionability_trigger,
        actionability_support,
        conviction_score,
        data_trust,
        external_shock_score,
        breadth_score,
        jpy_carry,
        event_assessment,
        thresholds,
    );
    let defend_signal = clause_diagnostics.has_defend_signal();
    let hedge_signal = clause_diagnostics.has_hedge_signal();
    let prepare_signal = clause_diagnostics.has_prepare_signal();

    let base_posture = if defend_signal {
        DecisionPosture::Defend
    } else if !severe_quality_block && hedge_signal {
        DecisionPosture::Hedge
    } else if prepare_signal {
        DecisionPosture::Prepare
    } else {
        DecisionPosture::Normal
    };
    let posture = adjust_posture_for_preferences(base_posture, user_preferences, event_assessment);
    let mut trigger_codes = clause_diagnostics.selected_trigger_codes(base_posture);
    if posture != base_posture {
        trigger_codes.push(preference_adjustment_code(user_preferences).to_string());
    }

    let mut reasons = Vec::new();
    if snapshot.structural_score >= 60.0 {
        reasons.push("结构性脆弱性已抬升，说明风险不是单日噪声。".to_string());
    }
    if snapshot.trigger_score >= 60.0 {
        reasons.push("触发层指标进入高压区，风险窗口正在缩短。".to_string());
    }
    if external_shock_score >= 55.0 {
        reasons.push("外部放大器偏强，JPY carry 或外部冲击可能加速风险传导。".to_string());
    }
    if event_assessment.confirmation_score >= 60.0 {
        reasons.push("事件层已经开始确认压力，不再只是市场价格单侧波动。".to_string());
    }
    if jpy_carry.funding_pressure_score >= 45.0 {
        reasons.push("美日短端利差仍偏高，套息资金在风险释放阶段更容易形成拥挤平仓。".to_string());
    }
    if conviction_score < 0.55 {
        reasons.push("当前信号可信度一般，仓位动作应保留二次确认。".to_string());
    }
    if let Some(analog) = analogs.first() {
        reasons.push(format!(
            "历史上当前形态最接近 {}，但仍需看事件层是否继续确认。",
            analog.name
        ));
    }
    if reasons.is_empty() {
        reasons.push("当前多维度尚未形成高强度共振。".to_string());
    }

    let summary = match posture {
        DecisionPosture::Normal => {
            "系统未看到足够证据支持主动防守，重点是继续观察触发层变化。".to_string()
        }
        DecisionPosture::Prepare => {
            "系统认为中期脆弱性已升高，适合先做流动性检查与对冲准备。".to_string()
        }
        DecisionPosture::Hedge => {
            "系统认为未来数周风险已值得对冲，重点是先保护组合而不是等待事件完全落地。".to_string()
        }
        DecisionPosture::Defend => {
            "系统认为短期风险窗口已经打开，优先资本保全和流动性管理。".to_string()
        }
    };

    let upgrade_condition = match posture {
        DecisionPosture::Normal => {
            format!(
                "若 p_60d 升至 {} 以上且 structural score 抬升，或外部冲击与结构脆弱性同步恶化，则升级为 prepare。",
                format_probability_threshold(thresholds.prepare_p60d)
            )
        }
        DecisionPosture::Prepare => {
            format!(
                "若 p_20d 升至 {} 以上，且 trigger、external、breadth 至少一项同步恶化，则升级为 hedge。",
                format_probability_threshold(thresholds.hedge_p20d)
            )
        }
        DecisionPosture::Hedge => {
            format!(
                "若 p_5d 升至 {} 以上、数据质量不低于 B，且 trigger / external / event 至少两类确认，则升级为 defend。",
                format_probability_threshold(thresholds.defend_p5d)
            )
        }
        DecisionPosture::Defend => "除非 p_5d 明显回落且触发层缓解，否则保持 defend。".to_string(),
    };

    let downgrade_condition = match posture {
        DecisionPosture::Normal => "维持 normal，直到结构与触发层重新抬升。".to_string(),
        DecisionPosture::Prepare => {
            format!(
                "若 p_60d 回落到 {} 以下且 structural score 不再继续抬升，则降回 normal。",
                format_probability_threshold(thresholds.downgrade_prepare_p60d())
            )
        }
        DecisionPosture::Hedge => {
            format!(
                "若 p_20d 连续回落到 {} 以下、外部冲击降温且 trigger score 下降，则降回 prepare。",
                format_probability_threshold(thresholds.downgrade_hedge_p20d())
            )
        }
        DecisionPosture::Defend => {
            format!(
                "若 p_5d 连续回落到 {} 以下、触发层缓和且没有新的高等级事件确认，可先降回 hedge。",
                format_probability_threshold(thresholds.downgrade_defend_p5d())
            )
        }
    };

    PostureGuidance {
        posture,
        summary,
        reasons,
        upgrade_condition,
        downgrade_condition,
        trigger_codes,
        blocker_codes: clause_diagnostics.blocker_code_strings(),
    }
}
