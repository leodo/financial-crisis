use fc_domain::{
    AssessmentMethodVersions, DataTrust, DecisionReliability, EventAssessment, HistoricalAnalog,
    MvpProbabilityInputStatus, MvpRiskState, RuntimeMetadata,
};

use super::common::round3;

pub(super) fn build_decision_reliability(
    data_trust: &DataTrust,
    method: &AssessmentMethodVersions,
    runtime: &RuntimeMetadata,
    event_assessment: &EventAssessment,
    historical_analogs: &[HistoricalAnalog],
    mvp_risk_state: &MvpRiskState,
) -> DecisionReliability {
    let data_coverage_component = data_trust.coverage_score.clamp(0.0, 1.0);
    let model_component = model_reliability_component(method, runtime, mvp_risk_state);
    let event_component = (event_assessment.confirmation_score / 100.0).clamp(0.0, 1.0);
    let historical_analog_component = historical_analog_component(historical_analogs);
    let freshness_component = freshness_reliability_component(runtime);
    let raw_score = data_coverage_component * 0.35
        + model_component * 0.25
        + event_component * 0.20
        + historical_analog_component * 0.10
        + freshness_component * 0.10;
    let (score, cap_reason) = apply_reliability_cap(raw_score, method, runtime, mvp_risk_state);
    let score = round3(score);
    let raw_score = round3(raw_score);
    let label = reliability_label(score, method, runtime, mvp_risk_state);
    let explanation = reliability_explanation(cap_reason.as_deref());

    DecisionReliability {
        score,
        raw_score,
        data_coverage_component: round3(data_coverage_component),
        model_component: round3(model_component),
        event_component: round3(event_component),
        historical_analog_component: round3(historical_analog_component),
        freshness_component: round3(freshness_component),
        label,
        cap_reason,
        explanation,
    }
}

fn model_reliability_component(
    method: &AssessmentMethodVersions,
    runtime: &RuntimeMetadata,
    mvp_risk_state: &MvpRiskState,
) -> f64 {
    if runtime.demo_mode {
        return 0.10;
    }
    if method.release_status == "degraded" {
        return 0.25;
    }
    if matches!(
        mvp_risk_state.probability_input_status,
        MvpProbabilityInputStatus::ReferenceOnly
    ) {
        return 0.35;
    }
    if method.release_status == "healthy" {
        return 0.90;
    }
    0.65
}

fn freshness_reliability_component(runtime: &RuntimeMetadata) -> f64 {
    if runtime.stale_warning.is_some() {
        return 0.35;
    }
    let business_lag = runtime
        .latest_key_indicator_lag_business_days
        .or(runtime.latest_observation_lag_business_days);
    match business_lag {
        None => 0.55,
        Some(lag) if lag <= 2 => 1.0,
        Some(lag) if lag <= 5 => 0.75,
        Some(lag) if lag <= 10 => 0.45,
        Some(_) => 0.25,
    }
}

fn historical_analog_component(historical_analogs: &[HistoricalAnalog]) -> f64 {
    historical_analogs
        .iter()
        .map(|analog| analog.similarity_score)
        .fold(0.0, f64::max)
        .clamp(0.0, 100.0)
        / 100.0
}

fn apply_reliability_cap(
    raw_score: f64,
    method: &AssessmentMethodVersions,
    runtime: &RuntimeMetadata,
    mvp_risk_state: &MvpRiskState,
) -> (f64, Option<String>) {
    if runtime.demo_mode {
        return (raw_score.min(0.40), Some("demo_mode_cap_40".to_string()));
    }
    if method.release_status == "degraded" {
        return (
            raw_score.min(0.50),
            Some("release_degraded_cap_50".to_string()),
        );
    }
    if matches!(
        mvp_risk_state.probability_input_status,
        MvpProbabilityInputStatus::ReferenceOnly
    ) {
        return (
            raw_score.min(0.45),
            Some("reference_only_cap_45".to_string()),
        );
    }
    if runtime.stale_warning.is_some() {
        return (
            raw_score.min(0.65),
            Some("runtime_stale_cap_65".to_string()),
        );
    }
    (raw_score, None)
}

fn reliability_label(
    score: f64,
    method: &AssessmentMethodVersions,
    runtime: &RuntimeMetadata,
    mvp_risk_state: &MvpRiskState,
) -> String {
    let percent = format!("{:.0}%", score * 100.0);
    if runtime.demo_mode {
        return format!("演示 {percent}");
    }
    if method.release_status == "degraded" {
        return format!("降级 {percent}");
    }
    if matches!(
        mvp_risk_state.probability_input_status,
        MvpProbabilityInputStatus::ReferenceOnly
    ) {
        return format!("参考上限 {percent}");
    }
    if score >= 0.80 {
        return format!("高可信 {percent}");
    }
    if score >= 0.65 {
        return format!("可用 {percent}");
    }
    if score >= 0.45 {
        return format!("需复核 {percent}");
    }
    format!("低可信 {percent}")
}

fn reliability_explanation(cap_reason: Option<&str>) -> String {
    let cap_copy = match cap_reason {
        Some("demo_mode_cap_40") => "当前是 demo 模式，可靠性分数封顶 40%。",
        Some("release_degraded_cap_50") => "当前 release 状态降级，可靠性分数封顶 50%。",
        Some("reference_only_cap_45") => "正式概率当前只作参考输入，可靠性分数封顶 45%。",
        Some("runtime_stale_cap_65") => "当前存在数据时效性提醒，可靠性分数封顶 65%。",
        _ => "当前没有触发可靠性封顶。",
    };
    format!(
        "结论可靠性不是危机发生概率，也不是动作升级证据。它按关键指标覆盖 35%、模型状态 25%、事件确认 20%、历史相似度 10%、关键数据新鲜度 10% 汇总。{cap_copy}"
    )
}
