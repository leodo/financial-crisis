use fc_domain::{
    AssessmentScores, DataTrust, EventAssessment, EventConfirmationState, JpyCarrySnapshot,
    JpyCarryState, MvpProbabilityInputStatus, MvpRiskState, MvpRiskStateCode,
    ProbabilityDiagnostics, QualityGrade,
};

const USDJPY_HIGH_TAIL_SUPPRESSOR_FEATURE: &str = "tail_pos__us_usdjpy_level__145";

pub(super) fn build_mvp_risk_state(
    scores: &AssessmentScores,
    data_trust: &DataTrust,
    jpy_carry: &JpyCarrySnapshot,
    event_assessment: &EventAssessment,
    probability_diagnostics: &ProbabilityDiagnostics,
) -> MvpRiskState {
    let probability_input_status = if has_probability_semantic_anomaly(probability_diagnostics) {
        MvpProbabilityInputStatus::AuditOnly
    } else {
        MvpProbabilityInputStatus::Usable
    };

    let data_blocked = data_trust.coverage_score < 0.75
        || matches!(data_trust.quality_grade, QualityGrade::D | QualityGrade::F);
    let code = if data_blocked {
        MvpRiskStateCode::Observe
    } else {
        classify_rule_state(scores, jpy_carry, event_assessment)
    };

    let mut primary_evidence = Vec::new();
    primary_evidence.push(format!(
        "风险强度 {:.1}，结构 {:.1}，触发 {:.1}，外部 {:.1}",
        scores.overall_score,
        scores.structural_score,
        scores.trigger_score,
        scores.external_shock_score
    ));
    primary_evidence.push(format!(
        "数据覆盖 {:.1}%，质量等级 {:?}",
        data_trust.coverage_score * 100.0,
        data_trust.quality_grade
    ));
    primary_evidence.push(format!(
        "事件确认 {:.1}，日元套息 {:?} {:.1}",
        event_assessment.confirmation_score, jpy_carry.state, jpy_carry.score
    ));

    let mut blockers = Vec::new();
    if matches!(
        probability_input_status,
        MvpProbabilityInputStatus::AuditOnly
    ) {
        blockers.push("正式 5d/20d/60d 概率命中模型语义异常，只能作为审计输入。".to_string());
    }
    if data_blocked {
        blockers.push("数据覆盖或质量不足，MVP 规则层只能保持观察。".to_string());
    }
    if scores.overall_score < 45.0 && scores.trigger_score < 45.0 {
        blockers.push("总风险和触发压力尚未进入 45 分以上的准备区。".to_string());
    }
    if event_assessment.confirmation_score < 45.0 {
        blockers.push("事件层尚未形成可升级仓位动作的确认。".to_string());
    }

    MvpRiskState {
        code,
        label: mvp_risk_state_label(code, probability_input_status).to_string(),
        probability_input_status,
        summary: mvp_risk_state_summary(code, probability_input_status),
        primary_evidence,
        blockers,
        next_actions: mvp_next_actions(code, probability_input_status),
    }
}

fn classify_rule_state(
    scores: &AssessmentScores,
    jpy_carry: &JpyCarrySnapshot,
    event_assessment: &EventAssessment,
) -> MvpRiskStateCode {
    if matches!(event_assessment.state, EventConfirmationState::Escalating)
        || (scores.overall_score >= 75.0
            && (scores.trigger_score >= 65.0 || scores.external_shock_score >= 65.0))
        || (matches!(jpy_carry.state, JpyCarryState::Unwind) && scores.trigger_score >= 55.0)
    {
        return MvpRiskStateCode::Defend;
    }

    if (matches!(
        event_assessment.state,
        EventConfirmationState::Confirmed | EventConfirmationState::Escalating
    ) && scores.overall_score >= 55.0)
        || (scores.trigger_score >= 60.0
            && (scores.overall_score >= 55.0
                || scores.structural_score >= 55.0
                || scores.external_shock_score >= 55.0))
        || (matches!(
            jpy_carry.state,
            JpyCarryState::Stress | JpyCarryState::Unwind
        ) && jpy_carry.score >= 60.0
            && scores.external_shock_score >= 50.0)
    {
        return MvpRiskStateCode::Hedge;
    }

    if scores.overall_score >= 45.0
        || scores.structural_score >= 45.0
        || scores.trigger_score >= 45.0
        || scores.external_shock_score >= 45.0
        || jpy_carry.score >= 45.0
        || event_assessment.confirmation_score >= 45.0
        || matches!(
            jpy_carry.state,
            JpyCarryState::Building | JpyCarryState::Stress
        )
    {
        return MvpRiskStateCode::Prepare;
    }

    MvpRiskStateCode::Observe
}

fn has_probability_semantic_anomaly(diagnostics: &ProbabilityDiagnostics) -> bool {
    diagnostics.horizon_overlays.iter().any(|horizon| {
        horizon.base_contributions.iter().any(|contribution| {
            contribution.name == USDJPY_HIGH_TAIL_SUPPRESSOR_FEATURE
                && contribution.raw_value > 0.0
                && contribution.contribution <= -1.0
        })
    })
}

fn mvp_risk_state_label(
    code: MvpRiskStateCode,
    probability_input_status: MvpProbabilityInputStatus,
) -> &'static str {
    match (code, probability_input_status) {
        (MvpRiskStateCode::Observe, MvpProbabilityInputStatus::AuditOnly) => {
            "观察为主（概率待审计）"
        }
        (MvpRiskStateCode::Observe, MvpProbabilityInputStatus::Usable) => "观察为主",
        (MvpRiskStateCode::Prepare, MvpProbabilityInputStatus::AuditOnly) => {
            "提前准备（概率待审计）"
        }
        (MvpRiskStateCode::Prepare, MvpProbabilityInputStatus::Usable) => "提前准备",
        (MvpRiskStateCode::Hedge, MvpProbabilityInputStatus::AuditOnly) => {
            "保护性对冲（概率待审计）"
        }
        (MvpRiskStateCode::Hedge, MvpProbabilityInputStatus::Usable) => "保护性对冲",
        (MvpRiskStateCode::Defend, MvpProbabilityInputStatus::AuditOnly) => {
            "防守优先（概率待审计）"
        }
        (MvpRiskStateCode::Defend, MvpProbabilityInputStatus::Usable) => "防守优先",
    }
}

fn mvp_risk_state_summary(
    code: MvpRiskStateCode,
    probability_input_status: MvpProbabilityInputStatus,
) -> String {
    let posture_copy = match code {
        MvpRiskStateCode::Observe => {
            "MVP 规则层未看到足够证据支持主动减仓或对冲，当前以观察和数据复核为主。"
        }
        MvpRiskStateCode::Prepare => "MVP 规则层看到风险积累，先准备现金、对冲工具和执行顺序。",
        MvpRiskStateCode::Hedge => {
            "MVP 规则层看到未来数周风险升温，应考虑保护性对冲和降低高波动暴露。"
        }
        MvpRiskStateCode::Defend => {
            "MVP 规则层看到近端防守条件，优先保流动性、降杠杆和压低尾部暴露。"
        }
    };
    if matches!(
        probability_input_status,
        MvpProbabilityInputStatus::AuditOnly
    ) {
        format!("{posture_copy} 正式概率当前只作为审计输入，不参与 MVP 主结论。")
    } else {
        posture_copy.to_string()
    }
}

fn mvp_next_actions(
    code: MvpRiskStateCode,
    probability_input_status: MvpProbabilityInputStatus,
) -> Vec<String> {
    let mut actions = match code {
        MvpRiskStateCode::Observe => vec![
            "保持常规监控，不把低 formal 概率解释成风险已经远离。".to_string(),
            "继续盯 VIX、信用利差、收益率曲线、USDJPY 和事件层是否共振。".to_string(),
        ],
        MvpRiskStateCode::Prepare => vec![
            "准备现金和保护工具，先不做一次性清仓动作。".to_string(),
            "确认流动性、税务和账户约束，预设分段减仓/对冲顺序。".to_string(),
        ],
        MvpRiskStateCode::Hedge => vec![
            "优先增加保护性对冲或降低高波动风险资产暴露。".to_string(),
            "避免新增杠杆，确认保护工具成交和流动性。".to_string(),
        ],
        MvpRiskStateCode::Defend => vec![
            "优先保流动性和资本，压低杠杆及尾部风险暴露。".to_string(),
            "只保留人工确认后的必要仓位，不把系统输出当自动交易指令。".to_string(),
        ],
    };

    if matches!(
        probability_input_status,
        MvpProbabilityInputStatus::AuditOnly
    ) {
        actions.push("等待 formal 概率模型通过 Go/No-Go 后，才恢复概率作为主结论。".to_string());
    }

    actions
}

#[cfg(test)]
mod tests {
    use super::*;
    use fc_domain::{
        DataQualitySummary, EventAssessment, LogisticProbabilityFeatureContribution,
        ProbabilityHorizonOverlayDiagnostics,
    };

    fn data_trust() -> DataTrust {
        DataTrust {
            coverage_score: 0.97,
            core_feature_coverage: 1.0,
            trigger_feature_coverage: 0.92,
            external_feature_coverage: 1.0,
            quality_grade: QualityGrade::A,
            data_quality_summary: DataQualitySummary {
                overall_score: 91.0,
                grade: QualityGrade::A,
                stale_indicator_count: 0,
                low_quality_indicator_count: 0,
                prototype_source_count: 0,
                blocked_indicator_count: 0,
            },
            warnings: Vec::new(),
        }
    }

    fn quiet_event() -> EventAssessment {
        EventAssessment {
            state: EventConfirmationState::Quiet,
            confirmation_score: 18.0,
            recent_event_count: 0,
            summary: "quiet".to_string(),
            confirmed_signals: Vec::new(),
            pending_gaps: Vec::new(),
            recent_events: Vec::new(),
        }
    }

    fn quiet_carry() -> JpyCarrySnapshot {
        JpyCarrySnapshot {
            state: JpyCarryState::Quiet,
            score: 17.5,
            usdjpy_level: Some(160.0),
            jp_call_rate: Some(0.7),
            us_short_rate: Some(3.6),
            us_jp_short_rate_diff: Some(2.9),
            change_5d: Some(0.7),
            change_20d: Some(3.14),
            realized_vol_20d: Some(0.001),
            funding_pressure_score: 34.7,
            vix_coupling_score: 46.5,
            credit_coupling_score: 20.0,
            reason: "quiet".to_string(),
        }
    }

    fn anomaly_diagnostics() -> ProbabilityDiagnostics {
        ProbabilityDiagnostics {
            horizon_overlays: vec![ProbabilityHorizonOverlayDiagnostics {
                horizon_days: 20,
                raw_probability: 0.000024,
                calibrated_probability: 0.000024,
                final_probability: 0.000067,
                runtime_final_probability: Some(0.000067),
                monotonic_lift: 0.0,
                configured_overlay_count: 5,
                base_contributions: vec![LogisticProbabilityFeatureContribution {
                    name: USDJPY_HIGH_TAIL_SUPPRESSOR_FEATURE.to_string(),
                    raw_value: 15.0,
                    normalized_value: 7.36,
                    weight: -1.22,
                    contribution: -9.02,
                }],
                contributions: Vec::new(),
                overlay_audits: Vec::new(),
            }],
        }
    }

    #[test]
    fn mvp_state_downgrades_formal_probability_to_audit_input() {
        let state = build_mvp_risk_state(
            &AssessmentScores {
                overall_score: 36.8,
                structural_score: 38.4,
                trigger_score: 35.0,
                external_shock_score: 29.3,
            },
            &data_trust(),
            &quiet_carry(),
            &quiet_event(),
            &anomaly_diagnostics(),
        );

        assert_eq!(state.code, MvpRiskStateCode::Observe);
        assert_eq!(
            state.probability_input_status,
            MvpProbabilityInputStatus::AuditOnly
        );
        assert_eq!(state.label, "观察为主（概率待审计）");
        assert!(state.summary.contains("正式概率当前只作为审计输入"));
        assert!(state
            .blockers
            .iter()
            .any(|blocker| blocker.contains("正式 5d/20d/60d 概率命中模型语义异常")));
    }

    #[test]
    fn mvp_state_promotes_prepare_from_validated_rule_layer_pressure() {
        let state = build_mvp_risk_state(
            &AssessmentScores {
                overall_score: 48.0,
                structural_score: 46.0,
                trigger_score: 42.0,
                external_shock_score: 30.0,
            },
            &data_trust(),
            &quiet_carry(),
            &quiet_event(),
            &ProbabilityDiagnostics::default(),
        );

        assert_eq!(state.code, MvpRiskStateCode::Prepare);
        assert_eq!(
            state.probability_input_status,
            MvpProbabilityInputStatus::Usable
        );
        assert_eq!(state.label, "提前准备");
    }
}
