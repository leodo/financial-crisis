use fc_domain::{
    observation_history_for_indicator, observation_value_difference_from_tail,
    ActionEvidenceBreakdown, DataTrust, IndicatorRisk, JpyCarrySnapshot, JpyCarryState,
    Observation, RiskContributor, RiskDimension, RiskSnapshot,
};

use super::{round1, round3, round_option, scaled_pressure};

pub(super) fn build_data_trust(
    snapshot: &RiskSnapshot,
    indicator_risks: &[IndicatorRisk],
    has_jpy_data: bool,
) -> DataTrust {
    let (core_total, core_present) = coverage_by_group(indicator_risks, |risk| {
        !is_external_or_event(risk.indicator.dimension)
    });
    let (trigger_total, trigger_present) = coverage_by_group(indicator_risks, |risk| {
        matches!(
            risk.indicator.dimension,
            RiskDimension::MarketStress
                | RiskDimension::LiquidityFunding
                | RiskDimension::EventsSentiment
        )
    });
    let (external_total, external_present) = coverage_by_group(indicator_risks, |risk| {
        risk.indicator.dimension == RiskDimension::ExternalSector
            || risk.indicator.indicator_id.starts_with("us_external_")
    });

    let core_feature_coverage = ratio(core_present, core_total);
    let trigger_feature_coverage = ratio(trigger_present, trigger_total);
    let external_feature_coverage = if external_total == 0 {
        if has_jpy_data {
            1.0
        } else {
            0.0
        }
    } else {
        ratio(external_present, external_total)
    };
    let coverage_score = round3(
        (core_feature_coverage * 0.45
            + trigger_feature_coverage * 0.35
            + external_feature_coverage * 0.2)
            .clamp(0.0, 1.0),
    );

    let mut warnings = Vec::new();
    if snapshot.data_quality_summary.prototype_source_count > 0 {
        warnings.push("部分事件或新闻数据仍是原型源，不能单独触发强结论。".to_string());
    }
    if snapshot.data_quality_summary.stale_indicator_count > 0 {
        warnings.push("存在滞后数据，短期概率需要保守解释。".to_string());
    }
    if !has_jpy_data {
        warnings.push("JPY carry 模块缺少 USDJPY 历史数据，外部冲击识别能力受限。".to_string());
    }
    let (blocked_core_count, blocked_auxiliary_count) =
        blocked_indicator_counts_by_decision_role(indicator_risks);
    if blocked_core_count > 0 {
        warnings.push(format!(
            "存在 {blocked_core_count} 个核心指标缺少或被阻断，建议先补齐数据再做强动作。"
        ));
    }
    if blocked_auxiliary_count > 0 {
        warnings.push(format!(
            "存在 {blocked_auxiliary_count} 个辅助/原型指标缺少观测；它不会单独触发强结论，但会降低事件层覆盖。"
        ));
    }

    DataTrust {
        coverage_score,
        core_feature_coverage: round3(core_feature_coverage),
        trigger_feature_coverage: round3(trigger_feature_coverage),
        external_feature_coverage: round3(external_feature_coverage),
        quality_grade: snapshot.data_quality_summary.grade,
        data_quality_summary: snapshot.data_quality_summary.clone(),
        warnings,
    }
}

fn blocked_indicator_counts_by_decision_role(indicator_risks: &[IndicatorRisk]) -> (usize, usize) {
    indicator_risks
        .iter()
        .filter(|risk| matches!(risk.quality_grade, fc_domain::QualityGrade::F))
        .fold((0, 0), |(core, auxiliary), risk| {
            if is_auxiliary_or_prototype_indicator(risk) {
                (core, auxiliary + 1)
            } else {
                (core + 1, auxiliary)
            }
        })
}

fn is_auxiliary_or_prototype_indicator(risk: &IndicatorRisk) -> bool {
    let quality_tier = risk.indicator.quality_tier.as_str();
    quality_tier.eq_ignore_ascii_case("supplemental")
        || quality_tier.eq_ignore_ascii_case("best_effort")
        || matches!(
            risk.indicator.default_source_id.as_str(),
            "gdelt" | "yfinance"
        )
}

pub(super) fn build_jpy_carry_snapshot(
    snapshot: &RiskSnapshot,
    indicator_risks: &[IndicatorRisk],
    observations: &[Observation],
) -> JpyCarrySnapshot {
    let usdjpy_history = observation_history_for_indicator(
        observations,
        "us_external_usdjpy_level",
        snapshot.as_of_date,
    );
    let usdjpy_level = usdjpy_history.last().map(|observation| observation.value);
    let jp_call_rate_history =
        observation_history_for_indicator(observations, "jp_rates_call_rate", snapshot.as_of_date);
    let jp_call_rate = jp_call_rate_history
        .last()
        .map(|observation| observation.value);
    let us_short_rate_history =
        observation_history_for_indicator(observations, "us_liquidity_effr", snapshot.as_of_date);
    let us_short_rate = us_short_rate_history
        .last()
        .map(|observation| observation.value);
    let us_jp_short_rate_diff = match (us_short_rate, jp_call_rate) {
        (Some(us), Some(jp)) => Some(us - jp),
        _ => None,
    };
    let change_5d = observation_value_difference_from_tail(&usdjpy_history, 5);
    let change_20d = observation_value_difference_from_tail(&usdjpy_history, 20);
    let realized_vol_20d = realized_volatility(&usdjpy_history, 20);
    let vix_score = find_indicator_score(indicator_risks, "us_market_vix_close");
    let credit_score = find_indicator_score(indicator_risks, "us_credit_high_yield_oas");
    let direction_reversal_score = change_5d
        .map(|change| (change.abs() * 4.0).clamp(0.0, 100.0))
        .unwrap_or(0.0);
    let vol_score = realized_vol_20d
        .map(|value| (value * 8.0).clamp(0.0, 100.0))
        .unwrap_or(0.0);
    let funding_pressure_score = round1(
        us_jp_short_rate_diff
            .map(|diff| (diff * 12.0).clamp(0.0, 100.0))
            .unwrap_or(18.0),
    );
    let vix_coupling_score =
        round1((direction_reversal_score * 0.35 + vix_score * 0.65).clamp(0.0, 100.0));
    let credit_coupling_score = round1((vol_score * 0.35 + credit_score * 0.65).clamp(0.0, 100.0));
    let score = round1(
        (direction_reversal_score * 0.25
            + vol_score * 0.22
            + funding_pressure_score * 0.18
            + vix_coupling_score * 0.2
            + credit_coupling_score * 0.15)
            .clamp(0.0, 100.0),
    );

    let state = if score >= 75.0 {
        JpyCarryState::Unwind
    } else if score >= 58.0 {
        JpyCarryState::Stress
    } else if score >= 35.0 {
        JpyCarryState::Building
    } else {
        JpyCarryState::Quiet
    };

    let reason = match state {
        JpyCarryState::Quiet => {
            if let Some(diff) = us_jp_short_rate_diff {
                format!("USDJPY 波动与美股/信用压力暂未形成明显共振，美日短端利差约 {diff:.2}%。")
            } else {
                "USDJPY 波动与美股/信用压力暂未形成明显共振。".to_string()
            }
        }
        JpyCarryState::Building => {
            if let Some(diff) = us_jp_short_rate_diff {
                format!("USDJPY 开始波动，美日短端利差约 {diff:.2}%，套息吸引力仍在，但还没有与信用和波动率形成全面同步。")
            } else {
                "USDJPY 开始波动，但还没有与信用和波动率形成全面同步。".to_string()
            }
        }
        JpyCarryState::Stress => {
            if let Some(diff) = us_jp_short_rate_diff {
                format!("USDJPY 波动已与 VIX 或信用利差形成联动，美日短端利差约 {diff:.2}%，外部放大器正在增强。")
            } else {
                "USDJPY 波动已与 VIX 或信用利差形成联动，外部放大器正在增强。".to_string()
            }
        }
        JpyCarryState::Unwind => {
            if let Some(diff) = us_jp_short_rate_diff {
                format!("JPY carry 平仓压力进入高位，美日短端利差约 {diff:.2}%，可能把数周风险压缩到数日。")
            } else {
                "JPY carry 平仓压力进入高位，可能把数周风险压缩到数日。".to_string()
            }
        }
    };

    JpyCarrySnapshot {
        state,
        score,
        usdjpy_level,
        jp_call_rate: round_option(jp_call_rate, 3),
        us_short_rate: round_option(us_short_rate, 3),
        us_jp_short_rate_diff: round_option(us_jp_short_rate_diff, 3),
        change_5d: round_option(change_5d, 3),
        change_20d: round_option(change_20d, 3),
        realized_vol_20d: round_option(realized_vol_20d, 3),
        funding_pressure_score,
        vix_coupling_score,
        credit_coupling_score,
        reason,
    }
}

pub(super) fn build_relief_drivers(indicator_risks: &[IndicatorRisk]) -> Vec<RiskContributor> {
    let mut rows = indicator_risks
        .iter()
        .filter(|risk| risk.latest_observation.is_some())
        .map(|risk| RiskContributor {
            indicator_id: risk.indicator.indicator_id.clone(),
            display_name: risk.indicator.display_name.clone(),
            dimension: risk.indicator.dimension,
            score: round1(risk.score),
            contribution: round1((100.0 - risk.score) * 0.2),
            explanation: format!("{} 当前处于相对低压区。", risk.indicator.display_name),
        })
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| left.score.total_cmp(&right.score));
    rows.truncate(3);
    rows
}

pub(super) fn build_action_evidence_breakdown(
    snapshot: &RiskSnapshot,
    data_trust: &DataTrust,
    breadth_score: f64,
) -> ActionEvidenceBreakdown {
    let breadth_component = scaled_pressure(breadth_score, 32.0, 35.0);
    let data_quality_component = data_trust.coverage_score * 0.48;
    let weighted_breadth_component = breadth_component * 0.34;
    let structural_trigger_agreement =
        snapshot.structural_score >= 55.0 && snapshot.trigger_score >= 55.0;
    let agreement_component = if structural_trigger_agreement {
        0.18
    } else {
        0.05
    };
    let score = round3(
        (data_quality_component + weighted_breadth_component + agreement_component)
            .clamp(0.12, 0.95),
    );
    ActionEvidenceBreakdown {
        score,
        data_quality_component: round3(data_quality_component),
        breadth_component: round3(weighted_breadth_component),
        agreement_component: round3(agreement_component),
        data_quality_weight: 0.48,
        breadth_weight: 0.34,
        agreement_high_component: 0.18,
        agreement_low_component: 0.05,
        breadth_score: round1(breadth_score),
        structural_trigger_agreement,
    }
}

pub(super) fn high_risk_breadth(snapshot: &RiskSnapshot) -> f64 {
    let total = snapshot.dimensions.len();
    if total == 0 {
        return 0.0;
    }
    let elevated = snapshot
        .dimensions
        .iter()
        .filter(|dimension| dimension.score >= 60.0)
        .count();
    elevated as f64 / total as f64 * 100.0
}

fn realized_volatility(observations: &[&Observation], window: usize) -> Option<f64> {
    let start = observations.len().saturating_sub(window + 1);
    let slice = observations.get(start..)?;
    if slice.len() < 3 {
        return None;
    }
    let changes = slice
        .windows(2)
        .filter_map(|pair| {
            let previous = pair.first()?.value;
            let current = pair.get(1)?.value;
            (previous.abs() > f64::EPSILON).then_some((current - previous) / previous.abs())
        })
        .collect::<Vec<_>>();
    if changes.len() < 2 {
        return None;
    }
    let mean = changes.iter().sum::<f64>() / changes.len() as f64;
    let variance = changes
        .iter()
        .map(|change| (change - mean).powi(2))
        .sum::<f64>()
        / changes.len() as f64;
    Some(variance.sqrt())
}

fn coverage_by_group<F>(indicator_risks: &[IndicatorRisk], predicate: F) -> (usize, usize)
where
    F: Fn(&IndicatorRisk) -> bool,
{
    indicator_risks.iter().filter(|risk| predicate(risk)).fold(
        (0_usize, 0_usize),
        |(total, present), risk| {
            (
                total + 1,
                present + usize::from(risk.latest_observation.is_some()),
            )
        },
    )
}

fn is_external_or_event(dimension: RiskDimension) -> bool {
    matches!(
        dimension,
        RiskDimension::ExternalSector | RiskDimension::EventsSentiment
    )
}

fn find_indicator_score(indicator_risks: &[IndicatorRisk], indicator_id: &str) -> f64 {
    indicator_risks
        .iter()
        .find(|risk| risk.indicator.indicator_id == indicator_id)
        .map(|risk| risk.score)
        .unwrap_or(0.0)
}

fn ratio(present: usize, total: usize) -> f64 {
    if total == 0 {
        return 0.0;
    }
    present as f64 / total as f64
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{NaiveDate, Utc};
    use fc_domain::{
        DataQualitySummary, Frequency, Indicator, QualityGrade, RiskDirection, RiskLevel,
    };

    fn risk_with_quality(
        indicator_id: &str,
        display_name: &str,
        dimension: RiskDimension,
        source_id: &str,
        quality_tier: &str,
        quality_grade: QualityGrade,
    ) -> IndicatorRisk {
        IndicatorRisk {
            indicator: Indicator {
                indicator_id: indicator_id.to_string(),
                display_name: display_name.to_string(),
                dimension,
                description: "test".to_string(),
                unit: "count".to_string(),
                frequency: Frequency::Daily,
                risk_direction: RiskDirection::HigherIsRiskier,
                default_source_id: source_id.to_string(),
                quality_tier: quality_tier.to_string(),
            },
            latest_observation: None,
            score: 0.0,
            level: RiskLevel::Normal,
            percentile: None,
            change_30d: None,
            score_basis: "缺少观测".to_string(),
            score_input_value: None,
            score_input_unit: None,
            quality_grade,
            contribution: 0.0,
        }
    }

    fn snapshot_with_blocked_count(blocked_indicator_count: usize) -> RiskSnapshot {
        RiskSnapshot {
            as_of_date: NaiveDate::from_ymd_opt(2026, 6, 9).unwrap(),
            entity_id: "us".to_string(),
            market_scope: "financial_system".to_string(),
            overall_score: 0.0,
            overall_level: RiskLevel::Normal,
            structural_score: 0.0,
            trigger_score: 0.0,
            level_reason: "test".to_string(),
            dimensions: Vec::new(),
            top_contributors: Vec::new(),
            data_quality_summary: DataQualitySummary {
                overall_score: 88.0,
                grade: QualityGrade::B,
                stale_indicator_count: 0,
                low_quality_indicator_count: 0,
                prototype_source_count: 0,
                blocked_indicator_count,
            },
            generated_at: Utc::now(),
            method_version: "test".to_string(),
        }
    }

    #[test]
    fn data_trust_warns_auxiliary_when_gdelt_news_is_missing() {
        let risks = vec![risk_with_quality(
            "global_news_financial_stress_count",
            "金融压力新闻数量",
            RiskDimension::EventsSentiment,
            "gdelt",
            "supplemental",
            QualityGrade::F,
        )];

        let trust = build_data_trust(&snapshot_with_blocked_count(1), &risks, true);

        assert!(trust
            .warnings
            .iter()
            .any(|warning| warning.contains("辅助/原型指标缺少观测")));
        assert!(!trust
            .warnings
            .iter()
            .any(|warning| warning.contains("核心指标")));
    }

    #[test]
    fn data_trust_preserves_core_blocker_warning_for_core_missing_indicator() {
        let risks = vec![risk_with_quality(
            "us_market_vix_close",
            "VIX 收盘价",
            RiskDimension::MarketStress,
            "fred",
            "core",
            QualityGrade::F,
        )];

        let trust = build_data_trust(&snapshot_with_blocked_count(1), &risks, true);

        assert!(trust
            .warnings
            .iter()
            .any(|warning| warning.contains("核心指标缺少或被阻断")));
    }
}
