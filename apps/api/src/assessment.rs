use chrono::NaiveDate;
use fc_domain::{
    AlertEvent, AssessmentMethodVersions, AssessmentScores, AssessmentSnapshot,
    BacktestRollingAudit, BacktestScenarioSummary, DataMode, IndicatorRisk, Observation,
    PostureGuidance, RiskDimension, RiskSnapshot, UserRiskPreferences,
};

mod common;
mod context;
mod market_context;
mod mvp_state;
mod posture;
mod probability;
mod runtime_policy;

use common::{
    clamp_probability, format_probability_threshold, posture_label, round6, round_option,
    scaled_pressure,
};
use common::{round1, round3};
pub(crate) use context::build_backtest_summary;
use context::{
    build_event_assessment, build_historical_analogs, build_key_indicator_statuses,
    build_runtime_metadata,
};
use market_context::{
    apply_key_indicator_freshness_guard, build_action_evidence_breakdown, build_data_trust,
    build_jpy_carry_snapshot, build_relief_drivers, high_risk_breadth,
};
use mvp_state::build_mvp_risk_state;
use posture::{
    build_position_guidance, build_posture_guidance, build_summary, build_time_to_risk_bucket,
};
pub(crate) use probability::ProbabilityComputationTrace;
use probability::{build_probabilities, build_probability_trace};
pub use runtime_policy::runtime_threshold_diagnostics;
pub(crate) use runtime_policy::{
    history_runtime_policy_version, probability_action_thresholds, ProbabilityActionThresholds,
};
pub use runtime_policy::{RuntimeThresholdDiagnostics, ServingModelContext};

const PROB_MODEL_VERSION: &str = "prob_v1_20260531";
const CALIBRATION_VERSION: &str = "calib_v1_20260531";
const FEATURE_SET_VERSION: &str = "feature_v2_20260531";
const LABEL_VERSION: &str = "label_v1_20260530";
const POSTURE_POLICY_VERSION: &str = "posture_v1_20260530";
const ACTION_PLAYBOOK_VERSION: &str = "action_playbook_v1_20260531";
const PROBABILITY_MODE: &str = "heuristic_mvp";
const RELEASE_STATUS: &str = "degraded";

pub(crate) fn prepare_reference_p60d(trace: &ProbabilityComputationTrace) -> Option<f64> {
    trace
        .probability_diagnostics
        .horizon_overlays
        .iter()
        .find(|horizon| horizon.horizon_days == 60)
        .map(|horizon| {
            horizon
                .runtime_final_probability
                .unwrap_or(horizon.final_probability)
        })
}

#[allow(clippy::too_many_arguments)]
pub fn build_assessment_snapshot(
    data_mode: DataMode,
    snapshot: &RiskSnapshot,
    indicator_risks: &[IndicatorRisk],
    observations: &[Observation],
    alerts: &[AlertEvent],
    backtests: &[BacktestScenarioSummary],
    rolling_audit: Option<&BacktestRollingAudit>,
    serving_model: Option<&ServingModelContext>,
    user_preferences: &UserRiskPreferences,
) -> (
    AssessmentSnapshot,
    PostureGuidance,
    ProbabilityComputationTrace,
) {
    let jpy_carry = build_jpy_carry_snapshot(snapshot, indicator_risks, observations);
    let external_dimension_score = snapshot
        .dimensions
        .iter()
        .find(|dimension| dimension.dimension == RiskDimension::ExternalSector)
        .map(|dimension| dimension.score)
        .unwrap_or(0.0);
    let event_dimension_score = snapshot
        .dimensions
        .iter()
        .find(|dimension| dimension.dimension == RiskDimension::EventsSentiment)
        .map(|dimension| dimension.score)
        .unwrap_or(0.0);
    let external_shock_score = round1(
        (external_dimension_score * 0.45 + jpy_carry.score * 0.4 + event_dimension_score * 0.15)
            .clamp(0.0, 100.0),
    );
    let key_indicators = build_key_indicator_statuses(observations, snapshot.as_of_date, data_mode);
    let data_trust = apply_key_indicator_freshness_guard(
        build_data_trust(snapshot, indicator_risks, jpy_carry.usdjpy_level.is_some()),
        &key_indicators,
    );
    let breadth_score = high_risk_breadth(snapshot);
    let action_evidence = build_action_evidence_breakdown(snapshot, &data_trust, breadth_score);
    let conviction_score = action_evidence.score;
    let heuristic_probabilities = build_probabilities(
        snapshot,
        external_shock_score,
        conviction_score,
        breadth_score,
        &data_trust,
        &jpy_carry,
    );
    let runtime = build_runtime_metadata(data_mode, snapshot, observations, &key_indicators);
    let probability_trace = build_probability_trace(
        snapshot,
        observations,
        external_shock_score,
        &data_trust,
        &jpy_carry,
        &heuristic_probabilities,
        &key_indicators,
        serving_model,
    );
    let probabilities = probability_trace.calibrated_probabilities.clone();
    let actionability = probability_trace.actionability.clone();
    let scores = AssessmentScores {
        overall_score: snapshot.overall_score,
        structural_score: snapshot.structural_score,
        trigger_score: snapshot.trigger_score,
        external_shock_score,
    };
    let actionability_trigger = probability_trace
        .actionability_enabled
        .then_some(&actionability);
    let actionability_support = Some(&actionability);
    let prepare_reference_p60d = prepare_reference_p60d(&probability_trace);
    let active_release = serving_model.map(|context| &context.release);
    let action_thresholds = probability_action_thresholds(serving_model);
    let time_to_risk_bucket = build_time_to_risk_bucket(
        &probabilities,
        prepare_reference_p60d,
        actionability_trigger,
        actionability_support,
        snapshot.overall_score,
        snapshot.structural_score,
        snapshot.trigger_score,
        external_shock_score,
        breadth_score,
        &jpy_carry,
        action_thresholds,
    );
    let top_risk_drivers = build_top_risk_drivers(
        &snapshot.top_contributors,
        indicator_risks,
        snapshot.as_of_date,
    );
    let top_relief_drivers = build_relief_drivers(indicator_risks);
    let historical_analogs = build_historical_analogs(
        snapshot,
        &probabilities,
        external_shock_score,
        backtests,
        action_thresholds,
    );
    let event_assessment = build_event_assessment(snapshot, alerts);
    let mvp_risk_state = build_mvp_risk_state(
        &scores,
        &data_trust,
        &jpy_carry,
        &event_assessment,
        &probability_trace.probability_diagnostics,
        &key_indicators,
    );
    let backtest_summary = build_backtest_summary(backtests, rolling_audit);
    let posture_guidance = build_posture_guidance(
        snapshot,
        &probabilities,
        prepare_reference_p60d,
        actionability_trigger,
        actionability_support,
        conviction_score,
        &data_trust,
        external_shock_score,
        breadth_score,
        &historical_analogs,
        &jpy_carry,
        &event_assessment,
        user_preferences,
        action_thresholds,
    );
    let position_guidance = build_position_guidance(
        &posture_guidance,
        &probabilities,
        time_to_risk_bucket,
        &data_trust,
        &jpy_carry,
        &event_assessment,
        active_release,
        user_preferences,
        action_thresholds,
    );
    let method = AssessmentMethodVersions {
        score_method_version: snapshot.method_version.clone(),
        prob_model_version: active_release
            .map(|release| release.manifest.prob_model_version.clone())
            .unwrap_or_else(|| PROB_MODEL_VERSION.to_string()),
        calibration_version: active_release
            .map(|release| release.manifest.calibration_version.clone())
            .unwrap_or_else(|| CALIBRATION_VERSION.to_string()),
        actionability_model_version: probability_trace.actionability_model_version.clone(),
        actionability_calibration_version: probability_trace
            .actionability_calibration_version
            .clone(),
        feature_set_version: active_release
            .map(|release| release.manifest.feature_set_version.clone())
            .unwrap_or_else(|| FEATURE_SET_VERSION.to_string()),
        label_version: active_release
            .map(|release| release.manifest.label_version.clone())
            .unwrap_or_else(|| LABEL_VERSION.to_string()),
        posture_policy_version: active_release
            .map(|release| release.manifest.posture_policy_version.clone())
            .unwrap_or_else(|| POSTURE_POLICY_VERSION.to_string()),
        action_playbook_version: active_release
            .map(|release| release.manifest.action_playbook_version.clone())
            .unwrap_or_else(|| ACTION_PLAYBOOK_VERSION.to_string()),
        fusion_policy_version: probability_trace.fusion_policy_version.clone(),
        actionability_enabled: probability_trace.actionability_enabled,
        probability_mode: serving_model
            .map(|context| context.runtime_probability_mode.clone())
            .unwrap_or_else(|| PROBABILITY_MODE.to_string()),
        release_status: serving_model
            .map(|context| context.runtime_release_status.clone())
            .unwrap_or_else(|| RELEASE_STATUS.to_string()),
        release_id: active_release.map(|release| release.manifest.release_id.clone()),
        point_in_time_mode: active_release
            .map(|release| release.manifest.point_in_time_mode.clone())
            .unwrap_or_else(|| "best_effort".to_string()),
    };
    let summary = build_summary(
        snapshot,
        &probabilities,
        time_to_risk_bucket,
        &posture_guidance,
        &mvp_risk_state,
    );

    (
        AssessmentSnapshot {
            as_of_date: snapshot.as_of_date,
            entity_id: snapshot.entity_id.clone(),
            market_scope: snapshot.market_scope.clone(),
            probabilities,
            actionability,
            probability_diagnostics: probability_trace.probability_diagnostics.clone(),
            time_to_risk_bucket,
            posture: posture_guidance.posture,
            mvp_risk_state,
            conviction_score,
            action_evidence,
            scores,
            summary,
            posture_reason: posture_guidance.summary.clone(),
            top_risk_drivers,
            top_relief_drivers,
            historical_analogs,
            data_trust,
            jpy_carry,
            position_guidance,
            runtime,
            key_indicators,
            event_assessment,
            backtest_summary,
            user_preferences: user_preferences.clone(),
            method,
        },
        posture_guidance,
        probability_trace,
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum DriverTimingBucket {
    NearTerm,
    Recent,
    Structural,
    Stale,
    Missing,
}

fn build_top_risk_drivers(
    base_drivers: &[fc_domain::RiskContributor],
    indicator_risks: &[IndicatorRisk],
    as_of_date: NaiveDate,
) -> Vec<fc_domain::RiskContributor> {
    let indicator_by_id = indicator_risks
        .iter()
        .map(|risk| (risk.indicator.indicator_id.as_str(), risk))
        .collect::<std::collections::BTreeMap<_, _>>();
    let base_explanation_by_id = base_drivers
        .iter()
        .map(|driver| (driver.indicator_id.as_str(), driver.explanation.as_str()))
        .collect::<std::collections::BTreeMap<_, _>>();

    let base_enriched = base_drivers.iter().filter_map(|driver| {
        enrich_top_risk_driver(
            driver,
            indicator_by_id.get(driver.indicator_id.as_str()).copied(),
            as_of_date,
        )
    });

    let near_term_candidates = indicator_risks
        .iter()
        .filter(|risk| risk.latest_observation.is_some() && risk.score > 0.0)
        .filter_map(|risk| {
            let synthetic = fc_domain::RiskContributor {
                indicator_id: risk.indicator.indicator_id.clone(),
                display_name: risk.indicator.display_name.clone(),
                dimension: risk.indicator.dimension,
                score: risk.score,
                contribution: risk.contribution,
                explanation: base_explanation_by_id
                    .get(risk.indicator.indicator_id.as_str())
                    .copied()
                    .unwrap_or("")
                    .to_string(),
            };
            enrich_top_risk_driver(&synthetic, Some(risk), as_of_date)
        })
        .filter(|driver| {
            matches!(
                driver.0,
                DriverTimingBucket::NearTerm | DriverTimingBucket::Recent
            )
        })
        .collect::<Vec<_>>();

    let mut selected = std::collections::BTreeMap::<
        String,
        (DriverTimingBucket, fc_domain::RiskContributor),
    >::new();
    let mut sorted_near_term = near_term_candidates;
    sorted_near_term.sort_by(|left, right| {
        left.0
            .cmp(&right.0)
            .then_with(|| right.1.contribution.total_cmp(&left.1.contribution))
    });
    for driver in sorted_near_term {
        selected.insert(driver.1.indicator_id.clone(), driver);
        if selected.len() >= 3 {
            break;
        }
    }

    let mut sorted_base = base_enriched.collect::<Vec<_>>();
    sorted_base.sort_by(|left, right| {
        left.0
            .cmp(&right.0)
            .then_with(|| right.1.contribution.total_cmp(&left.1.contribution))
    });
    for driver in sorted_base {
        if selected.len() >= 5 {
            break;
        }
        selected
            .entry(driver.1.indicator_id.clone())
            .or_insert(driver);
    }

    let mut rows = selected.into_values().collect::<Vec<_>>();
    rows.sort_by(|left, right| {
        left.0
            .cmp(&right.0)
            .then_with(|| right.1.contribution.total_cmp(&left.1.contribution))
    });
    rows.into_iter().map(|(_, driver)| driver).collect()
}

fn enrich_top_risk_driver(
    driver: &fc_domain::RiskContributor,
    risk: Option<&IndicatorRisk>,
    as_of_date: NaiveDate,
) -> Option<(DriverTimingBucket, fc_domain::RiskContributor)> {
    let Some(risk) = risk else {
        let mut enriched = driver.clone();
        enriched.explanation = format!(
            "{} 当前缺少可核对的最新观测日期。",
            driver.explanation.trim()
        );
        return Some((DriverTimingBucket::Missing, enriched));
    };
    let Some(latest_observation) = risk.latest_observation.as_ref() else {
        let mut enriched = driver.clone();
        enriched.explanation = format!(
            "{} 当前缺少可核对的最新观测日期。",
            driver.explanation.trim()
        );
        return Some((DriverTimingBucket::Missing, enriched));
    };

    let lag_days = (as_of_date - latest_observation.as_of_date)
        .num_days()
        .max(0);
    let timing_bucket = classify_driver_timing(risk.indicator.frequency, lag_days);
    let timing_note = build_driver_timing_note(
        latest_observation.as_of_date,
        risk.indicator.frequency,
        timing_bucket,
    );
    let mut enriched = driver.clone();
    let trimmed_explanation = enriched.explanation.trim();
    let base_explanation = if trimmed_explanation.is_empty() {
        build_synthetic_driver_explanation(risk)
    } else if let Some(rewritten) =
        rewrite_derived_driver_current_signal_copy(risk, trimmed_explanation)
    {
        rewritten
    } else {
        trimmed_explanation.to_string()
    };
    enriched.explanation = format!("{base_explanation} {timing_note}");
    Some((timing_bucket, enriched))
}

fn classify_driver_timing(frequency: fc_domain::Frequency, lag_days: i64) -> DriverTimingBucket {
    match frequency {
        fc_domain::Frequency::Daily => {
            if lag_days <= 7 {
                DriverTimingBucket::NearTerm
            } else if lag_days <= 14 {
                DriverTimingBucket::Recent
            } else {
                DriverTimingBucket::Stale
            }
        }
        fc_domain::Frequency::Weekly => {
            if lag_days <= 14 {
                DriverTimingBucket::NearTerm
            } else if lag_days <= 28 {
                DriverTimingBucket::Recent
            } else {
                DriverTimingBucket::Stale
            }
        }
        fc_domain::Frequency::Monthly => {
            if lag_days <= 45 {
                DriverTimingBucket::Recent
            } else if lag_days <= 120 {
                DriverTimingBucket::Structural
            } else {
                DriverTimingBucket::Stale
            }
        }
        fc_domain::Frequency::Quarterly => {
            if lag_days <= 180 {
                DriverTimingBucket::Structural
            } else {
                DriverTimingBucket::Stale
            }
        }
        fc_domain::Frequency::Annual => {
            if lag_days <= 420 {
                DriverTimingBucket::Structural
            } else {
                DriverTimingBucket::Stale
            }
        }
        fc_domain::Frequency::Event => DriverTimingBucket::NearTerm,
    }
}

fn build_driver_timing_note(
    latest_observation_date: NaiveDate,
    frequency: fc_domain::Frequency,
    timing_bucket: DriverTimingBucket,
) -> String {
    let frequency_label = match frequency {
        fc_domain::Frequency::Daily => "日频",
        fc_domain::Frequency::Weekly => "周频",
        fc_domain::Frequency::Monthly => "月频",
        fc_domain::Frequency::Quarterly => "季频",
        fc_domain::Frequency::Annual => "年频",
        fc_domain::Frequency::Event => "事件频",
    };
    match timing_bucket {
        DriverTimingBucket::NearTerm => {
            format!("最近观测 {latest_observation_date}（{frequency_label}，属于近端驱动）。")
        }
        DriverTimingBucket::Recent => format!(
            "最近观测 {latest_observation_date}（{frequency_label}，属于近月背景，解读要结合近端市场信号）。"
        ),
        DriverTimingBucket::Structural => format!(
            "最近观测 {latest_observation_date}（{frequency_label}慢变量，更偏结构背景，不表示今天市场刚出现这个变化）。"
        ),
        DriverTimingBucket::Stale => format!(
            "最近观测 {latest_observation_date}（{frequency_label}，相对当前请求日已偏旧，只能作为背景参照）。"
        ),
        DriverTimingBucket::Missing => "当前缺少可核对的最新观测日期。".to_string(),
    }
}

fn build_synthetic_driver_explanation(risk: &IndicatorRisk) -> String {
    let basis = risk.score_basis.as_str();
    let value = build_driver_score_input_copy(risk, basis);
    let percentile = risk
        .percentile
        .map(|value| {
            let normalized = if value > 1.0 { value } else { value * 100.0 };
            format!("，历史分位 {:.1}%", normalized)
        })
        .unwrap_or_default();
    format!(
        "{} 按{}评分，{}{}，风险分 {:.1}。",
        risk.indicator.display_name, basis, value, percentile, risk.score
    )
}

fn rewrite_derived_driver_current_signal_copy(
    risk: &IndicatorRisk,
    explanation: &str,
) -> Option<String> {
    let basis = risk.score_basis.as_str();
    if !is_derived_score_basis(basis)
        || explanation.contains("评分输入")
        || (!explanation.contains("当前信号") && !explanation.contains("当前读数"))
    {
        return None;
    }

    let replacement = build_driver_score_input_copy(risk, basis);
    replace_current_signal_clause(explanation, &replacement)
}

fn replace_current_signal_clause(explanation: &str, replacement: &str) -> Option<String> {
    for marker in ["当前信号", "当前读数"] {
        let Some(start) = explanation.find(marker) else {
            continue;
        };
        let tail = &explanation[start..];
        let Some(end) = ["，历史分位", "，风险分"]
            .iter()
            .filter_map(|delimiter| tail.find(delimiter))
            .min()
        else {
            continue;
        };

        return Some(format!(
            "{}{}{}",
            &explanation[..start],
            replacement,
            &tail[end..]
        ));
    }

    None
}

fn build_driver_score_input_copy(risk: &IndicatorRisk, basis: &str) -> String {
    let Some(value) = risk.score_input_value else {
        return if is_derived_score_basis(basis) {
            "评分输入缺失".to_string()
        } else {
            "当前信号缺失".to_string()
        };
    };

    let unit = display_driver_unit(risk.score_input_unit.as_deref());
    if is_derived_score_basis(basis) {
        let input = format_driver_value(value, unit, true);
        if let Some(latest) = risk.latest_observation.as_ref() {
            let latest_unit = display_driver_unit(Some(risk.indicator.unit.as_str()));
            let latest_value = format_driver_value(latest.value, latest_unit, false);
            return format!(
                "评分输入 {input}（{basis}，不是 {} 当前水平；最新水平 {latest_value}）",
                risk.indicator.display_name
            );
        }
        return format!("评分输入 {input}（{basis}，不是当前水平）");
    }

    format!("当前信号 {}", format_driver_value(value, unit, false))
}

fn is_derived_score_basis(basis: &str) -> bool {
    let lower = basis.to_ascii_lowercase();
    basis.contains("变化")
        || basis.contains("同比")
        || basis.contains("振幅")
        || lower.contains("change")
        || lower.contains("delta")
        || lower.contains("yoy")
}

fn format_driver_value(value: f64, unit: &str, signed: bool) -> String {
    let number = if signed {
        format!("{value:+.2}")
    } else {
        format!("{value:.2}")
    };
    if unit.is_empty() {
        number
    } else {
        format!("{number} {unit}")
    }
}

fn display_driver_unit(unit: Option<&str>) -> &str {
    match unit.unwrap_or("") {
        "percent" => "%",
        "index" => "指数",
        "jpy_per_usd" => "JPY/USD",
        "count" => "次",
        "score" => "分",
        "billions" => "十亿",
        "thousands" => "千",
        other => other,
    }
}

#[cfg(test)]
mod tests;
