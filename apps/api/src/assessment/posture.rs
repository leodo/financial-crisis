use fc_domain::{
    ActionabilityBlock, DataTrust, DecisionPosture, EventAssessment, EventConfirmationState,
    HistoricalAnalog, JpyCarrySnapshot, ModelReleaseRecord, PositionGuidance,
    PositionGuidanceGovernance, PostureGuidance, ProbabilityBlock, QualityGrade, RiskSnapshot,
    TimeToRiskBucket, UserRiskPreferences, UserRiskProfile,
};

use super::{
    format_probability_threshold, posture_label, round1, ProbabilityActionThresholds,
    ACTION_PLAYBOOK_VERSION,
};

#[derive(Debug, Clone, Default)]
struct PostureClauseDiagnostics {
    defend_trigger_codes: Vec<&'static str>,
    hedge_trigger_codes: Vec<&'static str>,
    prepare_trigger_codes: Vec<&'static str>,
    blocker_codes: Vec<&'static str>,
}

impl PostureClauseDiagnostics {
    fn selected_trigger_codes(&self, posture: DecisionPosture) -> Vec<String> {
        match posture {
            DecisionPosture::Defend => self
                .defend_trigger_codes
                .iter()
                .map(|code| (*code).to_string())
                .collect(),
            DecisionPosture::Hedge => self
                .hedge_trigger_codes
                .iter()
                .map(|code| (*code).to_string())
                .collect(),
            DecisionPosture::Prepare => self
                .prepare_trigger_codes
                .iter()
                .map(|code| (*code).to_string())
                .collect(),
            DecisionPosture::Normal => Vec::new(),
        }
    }

    fn blocker_code_strings(&self) -> Vec<String> {
        self.blocker_codes
            .iter()
            .map(|code| (*code).to_string())
            .collect()
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn build_time_to_risk_bucket(
    probabilities: &ProbabilityBlock,
    prepare_reference_p60d: Option<f64>,
    actionability: Option<&ActionabilityBlock>,
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
    let defend_head_now = actionability.is_some_and(|scores| {
        scores.defend >= 0.33
            && (trigger_score >= 55.0 || external_shock_score >= 55.0 || breadth_score >= 44.0)
    });
    let hedge_head_weeks = actionability.is_some_and(|scores| {
        scores.hedge >= 0.34
            && (trigger_score >= 48.0 || external_shock_score >= 48.0 || breadth_score >= 38.0)
    });
    let prepare_head_months = actionability.is_some_and(|scores| {
        scores.prepare >= 0.38
            && prepare_p60d >= thresholds.downgrade_prepare_p60d()
            && prepare_confirmation_count >= 2
            && (structural_score >= 56.0 || external_shock_score >= 55.0)
    });

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
    {
        TimeToRiskBucket::Months
    } else {
        TimeToRiskBucket::Normal
    }
}

#[allow(clippy::too_many_arguments)]
fn build_posture_clause_diagnostics(
    snapshot: &RiskSnapshot,
    probabilities: &ProbabilityBlock,
    prepare_reference_p60d: Option<f64>,
    actionability: Option<&ActionabilityBlock>,
    conviction_score: f64,
    data_trust: &DataTrust,
    external_shock_score: f64,
    breadth_score: f64,
    jpy_carry: &JpyCarrySnapshot,
    event_assessment: &EventAssessment,
    thresholds: ProbabilityActionThresholds,
) -> PostureClauseDiagnostics {
    let severe_quality_block =
        matches!(data_trust.quality_grade, QualityGrade::D | QualityGrade::F);
    let prepare_p60d = prepare_reference_p60d.unwrap_or(probabilities.p_60d);
    let defend_quality_gate = matches!(data_trust.quality_grade, QualityGrade::A | QualityGrade::B);
    let confirmation_count = posture_confirmation_count(
        snapshot.trigger_score,
        external_shock_score,
        event_assessment.confirmation_score,
    );
    let prepare_confirmation_count = prepare_context_confirmation_count(
        snapshot.trigger_score,
        external_shock_score,
        breadth_score,
        event_assessment.confirmation_score,
        jpy_carry.funding_pressure_score,
    );
    let prepare_non_external_confirmation_count = prepare_non_external_confirmation_count(
        snapshot.trigger_score,
        breadth_score,
        event_assessment.confirmation_score,
        jpy_carry.funding_pressure_score,
    );
    let prepare_non_carry_confirmation_count = prepare_non_carry_confirmation_count(
        snapshot.trigger_score,
        external_shock_score,
        breadth_score,
        event_assessment.confirmation_score,
    );
    let severe_carry = jpy_carry.score >= 70.0 && jpy_carry.funding_pressure_score >= 55.0;
    let stressed_carry = jpy_carry.score >= 58.0 && jpy_carry.funding_pressure_score >= 48.0;

    let mut defend_trigger_codes = Vec::new();
    if defend_quality_gate
        && confirmation_count >= 2
        && conviction_score >= 0.62
        && breadth_score >= 48.0
    {
        if probabilities.p_5d >= thresholds.defend_p5d && snapshot.trigger_score >= 60.0 {
            defend_trigger_codes.push("defend_p5d_trigger");
        }
        if severe_carry && snapshot.trigger_score >= 55.0 && external_shock_score >= 55.0 {
            defend_trigger_codes.push("defend_carry_trigger");
        }
        if actionability.is_some_and(|scores| {
            scores.defend >= 0.36
                && (snapshot.trigger_score >= 55.0 || external_shock_score >= 55.0)
        }) {
            defend_trigger_codes.push("defend_actionability");
        }
    }

    let mut hedge_trigger_codes = Vec::new();
    let hedge_context_support_count = [
        snapshot.trigger_score >= 50.0,
        external_shock_score >= 50.0,
        breadth_score >= 40.0,
        event_assessment.confirmation_score >= 40.0,
    ]
    .into_iter()
    .filter(|supported| *supported)
    .count();
    let hedge_medium_horizon_support = snapshot.structural_score >= 48.0
        || probabilities.p_60d >= thresholds.downgrade_prepare_p60d()
        || stressed_carry;
    let hedge_context_ready = snapshot.overall_score >= 58.0
        || external_shock_score >= 50.0
        || event_assessment.confirmation_score >= 45.0
        || stressed_carry;
    if probabilities.p_20d >= thresholds.hedge_p20d
        && hedge_context_support_count >= 2
        && hedge_medium_horizon_support
        && hedge_context_ready
    {
        hedge_trigger_codes.push("hedge_p20d_context");
    }
    if probabilities.p_60d >= thresholds.elevated_weeks_p60d()
        && snapshot.structural_score >= 55.0
        && snapshot.trigger_score >= 54.0
        && external_shock_score >= 48.0
    {
        hedge_trigger_codes.push("hedge_p60d_elevated");
    }
    if stressed_carry
        && external_shock_score >= 50.0
        && snapshot.structural_score >= 50.0
        && snapshot.trigger_score >= 45.0
    {
        hedge_trigger_codes.push("hedge_carry_structural");
    }
    if actionability.is_some_and(|scores| {
        scores.hedge >= 0.36
            && (snapshot.trigger_score >= 46.0
                || external_shock_score >= 48.0
                || event_assessment.confirmation_score >= 35.0)
    }) {
        hedge_trigger_codes.push("hedge_actionability");
    }

    let mut prepare_trigger_codes = Vec::new();
    if conviction_score >= 0.54 {
        if prepare_p60d >= thresholds.prepare_p60d
            && snapshot.structural_score >= 58.0
            && prepare_confirmation_count >= 2
        {
            prepare_trigger_codes.push("prepare_p60d_structural");
        }
        if snapshot.structural_score >= 64.0
            && prepare_p60d >= thresholds.downgrade_prepare_p60d()
            && prepare_confirmation_count >= 2
        {
            prepare_trigger_codes.push("prepare_structural_downgrade");
        }
        if external_shock_score >= 58.0
            && snapshot.structural_score >= 54.0
            && probabilities.p_20d >= thresholds.external_prepare_p20d()
            && prepare_non_external_confirmation_count >= 1
        {
            prepare_trigger_codes.push("prepare_external_structural");
        }
        if stressed_carry
            && snapshot.structural_score >= 56.0
            && prepare_p60d >= thresholds.carry_prepare_p60d()
            && prepare_non_carry_confirmation_count >= 1
        {
            prepare_trigger_codes.push("prepare_carry_structural");
        }
        if actionability.is_some_and(|scores| {
            scores.prepare >= 0.40
                && prepare_p60d >= thresholds.downgrade_prepare_p60d()
                && prepare_confirmation_count >= 2
                && (snapshot.structural_score >= 56.0 || external_shock_score >= 55.0)
        }) {
            prepare_trigger_codes.push("prepare_actionability");
        }
    }

    let mut blocker_codes = Vec::new();
    if severe_quality_block && !hedge_trigger_codes.is_empty() {
        blocker_codes.push("quality_blocked_hedge");
    }

    PostureClauseDiagnostics {
        defend_trigger_codes,
        hedge_trigger_codes,
        prepare_trigger_codes,
        blocker_codes,
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn build_posture_guidance(
    snapshot: &RiskSnapshot,
    probabilities: &ProbabilityBlock,
    prepare_reference_p60d: Option<f64>,
    actionability: Option<&ActionabilityBlock>,
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
        actionability,
        conviction_score,
        data_trust,
        external_shock_score,
        breadth_score,
        jpy_carry,
        event_assessment,
        thresholds,
    );
    let defend_signal = !clause_diagnostics.defend_trigger_codes.is_empty();
    let hedge_signal = !clause_diagnostics.hedge_trigger_codes.is_empty();
    let prepare_signal = !clause_diagnostics.prepare_trigger_codes.is_empty();

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
        let adjustment_code = match user_preferences.profile {
            UserRiskProfile::Conservative => "preference_conservative_escalation",
            UserRiskProfile::Aggressive => "preference_aggressive_deescalation",
            UserRiskProfile::Neutral => "preference_neutral_no_adjustment",
        };
        trigger_codes.push(adjustment_code.to_string());
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

fn posture_confirmation_count(
    trigger_score: f64,
    external_shock_score: f64,
    event_confirmation_score: f64,
) -> u8 {
    [
        trigger_score >= 60.0,
        external_shock_score >= 55.0,
        event_confirmation_score >= 55.0,
    ]
    .into_iter()
    .filter(|flag| *flag)
    .count() as u8
}

fn prepare_context_confirmation_count(
    trigger_score: f64,
    external_shock_score: f64,
    breadth_score: f64,
    event_confirmation_score: f64,
    carry_funding_pressure_score: f64,
) -> u8 {
    [
        trigger_score >= 45.0,
        external_shock_score >= 50.0,
        breadth_score >= 36.0,
        event_confirmation_score >= 38.0,
        carry_funding_pressure_score >= 48.0,
    ]
    .into_iter()
    .filter(|flag| *flag)
    .count() as u8
}

fn prepare_context_confirmation_count_without_events(
    trigger_score: f64,
    external_shock_score: f64,
    breadth_score: f64,
    carry_funding_pressure_score: f64,
) -> u8 {
    [
        trigger_score >= 45.0,
        external_shock_score >= 50.0,
        breadth_score >= 36.0,
        carry_funding_pressure_score >= 48.0,
    ]
    .into_iter()
    .filter(|flag| *flag)
    .count() as u8
}

fn prepare_non_external_confirmation_count(
    trigger_score: f64,
    breadth_score: f64,
    event_confirmation_score: f64,
    carry_funding_pressure_score: f64,
) -> u8 {
    [
        trigger_score >= 45.0,
        breadth_score >= 36.0,
        event_confirmation_score >= 38.0,
        carry_funding_pressure_score >= 48.0,
    ]
    .into_iter()
    .filter(|flag| *flag)
    .count() as u8
}

fn prepare_non_external_confirmation_count_without_events(
    trigger_score: f64,
    breadth_score: f64,
    carry_funding_pressure_score: f64,
) -> u8 {
    [
        trigger_score >= 45.0,
        breadth_score >= 36.0,
        carry_funding_pressure_score >= 48.0,
    ]
    .into_iter()
    .filter(|flag| *flag)
    .count() as u8
}

fn prepare_non_carry_confirmation_count(
    trigger_score: f64,
    external_shock_score: f64,
    breadth_score: f64,
    event_confirmation_score: f64,
) -> u8 {
    [
        trigger_score >= 45.0,
        external_shock_score >= 50.0,
        breadth_score >= 36.0,
        event_confirmation_score >= 38.0,
    ]
    .into_iter()
    .filter(|flag| *flag)
    .count() as u8
}

fn prepare_non_carry_confirmation_count_without_events(
    trigger_score: f64,
    external_shock_score: f64,
    breadth_score: f64,
) -> u8 {
    [
        trigger_score >= 45.0,
        external_shock_score >= 50.0,
        breadth_score >= 36.0,
    ]
    .into_iter()
    .filter(|flag| *flag)
    .count() as u8
}

#[allow(clippy::too_many_arguments)]
pub(super) fn build_position_guidance(
    posture: &PostureGuidance,
    probabilities: &ProbabilityBlock,
    time_to_risk_bucket: TimeToRiskBucket,
    data_trust: &DataTrust,
    jpy_carry: &JpyCarrySnapshot,
    event_assessment: &EventAssessment,
    active_release: Option<&ModelReleaseRecord>,
    user_preferences: &UserRiskPreferences,
    thresholds: ProbabilityActionThresholds,
) -> PositionGuidance {
    let (
        mut target_equity_exposure_pct,
        mut target_cash_pct,
        mut hedge_ratio_pct,
        mut leverage_cap_pct,
        mut option_overlay_pct,
    ): (f64, f64, f64, f64, f64) = match posture.posture {
        DecisionPosture::Normal => (70.0_f64, 10.0_f64, 0.0_f64, 100.0_f64, 0.0_f64),
        DecisionPosture::Prepare => (55.0_f64, 20.0_f64, 10.0_f64, 75.0_f64, 5.0_f64),
        DecisionPosture::Hedge => (40.0_f64, 30.0_f64, 25.0_f64, 45.0_f64, 10.0_f64),
        DecisionPosture::Defend => (25.0_f64, 45.0_f64, 40.0_f64, 20.0_f64, 15.0_f64),
    };

    target_equity_exposure_pct =
        target_equity_exposure_pct.min(user_preferences.max_equity_cap_pct);
    target_cash_pct = target_cash_pct.max(user_preferences.cash_floor_pct);
    leverage_cap_pct = leverage_cap_pct.min(user_preferences.max_leverage_pct);
    option_overlay_pct = option_overlay_pct.max(user_preferences.option_overlay_preference_pct);

    if matches!(user_preferences.profile, UserRiskProfile::Conservative) {
        hedge_ratio_pct = (hedge_ratio_pct + 5.0).clamp(0.0, 100.0);
        target_equity_exposure_pct = (target_equity_exposure_pct - 5.0).max(0.0);
    } else if matches!(user_preferences.profile, UserRiskProfile::Aggressive)
        && user_preferences.allow_aggressive_reentry
        && matches!(
            posture.posture,
            DecisionPosture::Normal | DecisionPosture::Prepare
        )
    {
        target_equity_exposure_pct = (target_equity_exposure_pct + 5.0).min(100.0);
    }

    let mut actions = Vec::new();
    match posture.posture {
        DecisionPosture::Normal => {
            actions.push("维持核心仓位，不主动放大高 beta 暴露。".to_string());
            actions.push("继续监控信用利差、波动率和 JPY carry 是否转入共振。".to_string());
        }
        DecisionPosture::Prepare => {
            actions.push("降低高 beta、低流动性和高杠杆资产比重。".to_string());
            actions.push("预留更多现金或短久期工具，准备必要时快速降仓。".to_string());
            actions.push("评估保护性认沽或波动率保护的成本窗口。".to_string());
        }
        DecisionPosture::Hedge => {
            actions.push("主动收缩高风险敞口，把组合回撤控制放到收益追逐之前。".to_string());
            actions.push("提高现金和短久期资产比例，并建立保护性认沽或指数对冲。".to_string());
            actions.push("避免新增尾部流动性差的仓位。".to_string());
        }
        DecisionPosture::Defend => {
            actions.push("优先降低总风险暴露，把组合切到资本保全模式。".to_string());
            actions.push("保留高流动性头寸，优先兑现低流动性和高弹性风险资产。".to_string());
            actions.push("用指数认沽、波动率或其他保护工具覆盖核心风险敞口。".to_string());
        }
    }

    if jpy_carry.funding_pressure_score >= 50.0 {
        actions.push("外部融资压力偏高，注意全球风险资产同步回撤时的流动性冲击。".to_string());
    }
    actions.push(format!(
        "当前用户配置为 {}，系统已按该风险偏好约束仓位预算。",
        user_profile_label(user_preferences.profile)
    ));

    let mut forbidden_actions = vec![
        "不要把单个概率值当成一键清仓指令。".to_string(),
        "不要只因单日反弹就撤掉全部保护。".to_string(),
    ];
    match posture.posture {
        DecisionPosture::Normal => {
            forbidden_actions.push("不要因为短期平静就盲目放大杠杆或追逐高 beta。".to_string());
        }
        DecisionPosture::Prepare => {
            forbidden_actions
                .push("不要等到流动性明显恶化后才开始腾挪现金和保护工具。".to_string());
        }
        DecisionPosture::Hedge => {
            forbidden_actions.push("不要在尚未满足再入场条件前逆势放大组合净敞口。".to_string());
        }
        DecisionPosture::Defend => {
            forbidden_actions
                .push("不要在短期风险窗口已打开时新增复杂、流动性差的保护结构。".to_string());
        }
    }

    let mut reentry_conditions = match posture.posture {
        DecisionPosture::Normal => vec![
            "当前无需系统性再入场动作，维持常规监控即可。".to_string(),
            "若后续进入 prepare，再按 3-10 个交易日节奏分段调整。".to_string(),
        ],
        DecisionPosture::Prepare => vec![
            format!(
                "当 p_60d 回落到 {} 以下，且 structural score 不再继续抬升时，再考虑恢复常态仓位。",
                format_probability_threshold(thresholds.downgrade_prepare_p60d())
            ),
            "恢复仓位时仍应先回补高流动性核心仓位，再看高 beta 资产。".to_string(),
        ],
        DecisionPosture::Hedge => vec![
            format!(
                "当 p_20d 连续 5 个交易日回落到 {} 以下，并且外部冲击与信用压力同步缓和时，再逐步恢复仓位。",
                format_probability_threshold(thresholds.downgrade_hedge_p20d())
            ),
            "默认按 1/3、1/3、1/3 的分批节奏恢复，不做一次性满仓回补。".to_string(),
        ],
        DecisionPosture::Defend => vec![
            format!(
                "只有当 p_5d 连续 3 个交易日回落到 {} 以下，且没有新的高等级事件确认时，才允许从 defend 降回 hedge。",
                format_probability_threshold(thresholds.downgrade_defend_p5d())
            ),
            "从 defend 撤回防守时先恢复核心流动性仓位，最后再恢复高弹性风险资产。".to_string(),
        ],
    };
    if matches!(
        event_assessment.state,
        EventConfirmationState::Confirmed | EventConfirmationState::Escalating
    ) {
        reentry_conditions
            .push("事件层仍有确认或升级信号时，不应仅凭价格反弹提前撤掉保护。".to_string());
    }

    let mut guardrails = vec![
        "系统 posture 不是自动交易指令，不能替代你自己的风险预算。".to_string(),
        "不要仅凭单个概率值做全清仓动作，必须结合流动性、税务和执行条件。".to_string(),
    ];
    if !matches!(data_trust.quality_grade, QualityGrade::A) {
        guardrails
            .push("当前数据可信度尚可，但事件层仍有原型源，建议保留人工二次确认。".to_string());
    }
    if probabilities.p_5d >= thresholds.defend_p5d {
        guardrails.push("短期窗口已打开，更应优先考虑可快速执行的保护动作。".to_string());
    }

    let execution_urgency = match time_to_risk_bucket {
        TimeToRiskBucket::Normal => "观察为主；当前不需要系统性快速去风险。".to_string(),
        TimeToRiskBucket::Months => {
            "分阶段执行；建议在 3-10 个交易日内先降脆弱仓位、补现金和准备保护工具。".to_string()
        }
        TimeToRiskBucket::Weeks => {
            "尽快执行；建议在 1-5 个交易日内完成主要减仓和第一层组合保护。".to_string()
        }
        TimeToRiskBucket::Now => {
            "立即执行；当日到 2 个交易日内优先去杠杆、补现金并建立核心保护覆盖。".to_string()
        }
    };
    let confidence_gate = match data_trust.quality_grade {
        QualityGrade::A | QualityGrade::B if event_assessment.confirmation_score >= 55.0 => {
            "当前数据可信度和事件确认度足以支持执行主要防守动作。".to_string()
        }
        QualityGrade::D | QualityGrade::F => {
            "数据可信度偏低，先把系统输出当成减震和排查信号，不应直接做极端仓位动作。".to_string()
        }
        _ => "当前更适合先降低组合脆弱性，再结合事件确认和市场流动性决定是否加大保护。".to_string(),
    };
    let capital_preservation_overlay_enabled = matches!(posture.posture, DecisionPosture::Defend)
        && matches!(time_to_risk_bucket, TimeToRiskBucket::Now)
        && probabilities.p_5d >= thresholds.capital_preservation_p5d()
        && matches!(data_trust.quality_grade, QualityGrade::A | QualityGrade::B)
        && matches!(
            event_assessment.state,
            EventConfirmationState::Confirmed | EventConfirmationState::Escalating
        );

    let action_summary = match posture.posture {
        DecisionPosture::Normal => "以观察为主，维持核心仓位，不建议主动大幅防守。".to_string(),
        DecisionPosture::Prepare => "先做减震，不急于极端防守，但要为快速切换做准备。".to_string(),
        DecisionPosture::Hedge => "进入保护性对冲区间，优先减少组合脆弱性。".to_string(),
        DecisionPosture::Defend => "进入资本保全区间，优先流动性、现金和保护覆盖。".to_string(),
    };
    let governance = build_position_guidance_governance(
        data_trust,
        event_assessment,
        active_release,
        capital_preservation_overlay_enabled,
    );

    PositionGuidance {
        action_playbook_version: active_release
            .map(|release| release.manifest.action_playbook_version.clone())
            .unwrap_or_else(|| ACTION_PLAYBOOK_VERSION.to_string()),
        execution_urgency,
        confidence_gate,
        target_equity_exposure_pct: round1(target_equity_exposure_pct),
        target_cash_pct: round1(target_cash_pct),
        hedge_ratio_pct: round1(hedge_ratio_pct),
        leverage_cap_pct: round1(leverage_cap_pct),
        option_overlay_pct: round1(option_overlay_pct),
        action_summary,
        actions,
        forbidden_actions,
        reentry_conditions,
        guardrails,
        capital_preservation_overlay_enabled,
        governance,
    }
}

fn build_position_guidance_governance(
    data_trust: &DataTrust,
    event_assessment: &EventAssessment,
    active_release: Option<&ModelReleaseRecord>,
    capital_preservation_overlay_enabled: bool,
) -> PositionGuidanceGovernance {
    let mut required_operator_checks = vec![
        "先确认当前动作框架版本与 active release 一致，再解释仓位预算。".to_string(),
        "先检查数据模式、关键指标日期和 stale warning，避免把演示值或陈旧值当成当前市场。"
            .to_string(),
        "执行前必须结合你自己的流动性、税务、账户约束和持仓结构做人为确认。".to_string(),
        "任何会改变仓位规则的变更都要先经过 release review，不能直接跳过动作手册边界。".to_string(),
        "正式主模型的动作规则升级仍需满足 Go/No-Go，不允许只凭页面观感直接放行。".to_string(),
    ];

    if !matches!(data_trust.quality_grade, QualityGrade::A | QualityGrade::B) {
        required_operator_checks.push(
            "当前数据可信度不足，先把输出当成减震与排查信号，不要直接执行极端仓位动作。"
                .to_string(),
        );
    }

    if matches!(
        event_assessment.state,
        EventConfirmationState::Confirmed | EventConfirmationState::Escalating
    ) {
        required_operator_checks.push(
            "事件层已确认或升级，执行时要优先核对保护工具、对手方和流动性是否可用。".to_string(),
        );
    }

    if capital_preservation_overlay_enabled {
        required_operator_checks.push(
            "资本保全叠加已打开；若要进一步收缩风险暴露，先确认该动作仍符合当前 playbook 与人工风控边界。"
                .to_string(),
        );
    }

    if active_release.is_none() {
        required_operator_checks.push(
            "当前没有绑定 active release，说明动作框架仍处于默认/降级路径，更需要人工复核。"
                .to_string(),
        );
    }

    PositionGuidanceGovernance {
        system_budget_only: true,
        auto_execution_allowed: false,
        manual_confirmation_required: true,
        policy_change_requires_release_review: true,
        policy_change_requires_go_no_go: true,
        required_operator_checks,
    }
}

fn adjust_posture_for_preferences(
    base_posture: DecisionPosture,
    user_preferences: &UserRiskPreferences,
    event_assessment: &EventAssessment,
) -> DecisionPosture {
    match user_preferences.profile {
        UserRiskProfile::Conservative => escalate_posture(base_posture),
        UserRiskProfile::Aggressive => {
            if matches!(
                event_assessment.state,
                EventConfirmationState::Quiet | EventConfirmationState::Watching
            ) {
                deescalate_posture(base_posture)
            } else {
                base_posture
            }
        }
        UserRiskProfile::Neutral => base_posture,
    }
}

fn escalate_posture(posture: DecisionPosture) -> DecisionPosture {
    match posture {
        DecisionPosture::Normal => DecisionPosture::Prepare,
        DecisionPosture::Prepare => DecisionPosture::Hedge,
        DecisionPosture::Hedge | DecisionPosture::Defend => DecisionPosture::Defend,
    }
}

fn deescalate_posture(posture: DecisionPosture) -> DecisionPosture {
    match posture {
        DecisionPosture::Defend => DecisionPosture::Hedge,
        DecisionPosture::Hedge => DecisionPosture::Prepare,
        DecisionPosture::Prepare | DecisionPosture::Normal => DecisionPosture::Normal,
    }
}

fn user_profile_label(profile: UserRiskProfile) -> &'static str {
    match profile {
        UserRiskProfile::Conservative => "保守",
        UserRiskProfile::Neutral => "中性",
        UserRiskProfile::Aggressive => "进取",
    }
}

pub(super) fn build_summary(
    _snapshot: &RiskSnapshot,
    probabilities: &ProbabilityBlock,
    time_to_risk_bucket: TimeToRiskBucket,
    posture: &PostureGuidance,
) -> String {
    let horizon_text = match time_to_risk_bucket {
        TimeToRiskBucket::Normal => "当前仍偏常态区间",
        TimeToRiskBucket::Months => "未来数月进入高风险阶段的概率已抬升",
        TimeToRiskBucket::Weeks => "未来数周风险窗口已经值得重视",
        TimeToRiskBucket::Now => "短期风险窗口已经打开",
    };
    format!(
        "{}。5d / 20d / 60d 概率分别为 {:.0}% / {:.0}% / {:.0}%，它们回答的是危机窗口离现在多近；prepare / hedge / defend 动作层与当前 posture 回答的是现在该不该开始准备、保护或防守。当前 posture 为 {}。",
        horizon_text,
        probabilities.p_5d * 100.0,
        probabilities.p_20d * 100.0,
        probabilities.p_60d * 100.0,
        posture_label(posture.posture)
    )
}
