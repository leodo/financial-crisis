use std::collections::BTreeMap;

use chrono::{NaiveDate, Utc};
use fc_domain::{
    DataQualitySummary, DimensionScore, Frequency, Indicator, IndicatorRisk, Observation,
    QualityGrade, RiskContributor, RiskDimension, RiskDirection, RiskLevel, RiskSnapshot,
};

const METHOD_VERSION: &str = "scoring_v2_20260531";
const TAIL_WEIGHT: f64 = 0.2;
const YOY_DAYS: i64 = 365;

#[derive(Debug, Clone)]
pub struct ScoringEngine {
    method_version: String,
}

impl Default for ScoringEngine {
    fn default() -> Self {
        Self {
            method_version: METHOD_VERSION.to_string(),
        }
    }
}

impl ScoringEngine {
    pub fn method_version(&self) -> &str {
        &self.method_version
    }

    pub fn score(
        &self,
        indicators: &[Indicator],
        observations: &[Observation],
        as_of_date: NaiveDate,
        entity_id: &str,
        market_scope: &str,
    ) -> ScoringOutput {
        let mut by_indicator: BTreeMap<&str, Vec<&Observation>> = BTreeMap::new();
        for observation in observations
            .iter()
            .filter(|observation| {
                observation.entity_id == entity_id
                    || external_proxy_entity(&observation.indicator_id)
                        == Some(observation.entity_id.as_str())
            })
            .filter(|observation| observation.as_of_date <= as_of_date)
        {
            by_indicator
                .entry(observation.indicator_id.as_str())
                .or_default()
                .push(observation);
        }

        for history in by_indicator.values_mut() {
            history.sort_by_key(|observation| observation.as_of_date);
        }

        let mut indicator_risks = Vec::with_capacity(indicators.len());
        for indicator in indicators {
            let history = by_indicator
                .get(indicator.indicator_id.as_str())
                .cloned()
                .unwrap_or_default();
            let latest = history.last().copied().cloned();
            let signal = compute_signal(indicator, &history, latest.as_ref());
            let quality_grade = latest
                .as_ref()
                .map(|observation| QualityGrade::from_score(observation.quality_score))
                .unwrap_or(QualityGrade::F);
            let change_30d = latest
                .as_ref()
                .and_then(|latest| change_since_days(&history, latest.as_of_date, 30));
            indicator_risks.push(IndicatorRisk {
                indicator: indicator.clone(),
                latest_observation: latest,
                score: signal.score,
                level: RiskLevel::from_score(signal.score),
                percentile: signal.percentile,
                change_30d,
                score_basis: signal.score_basis,
                score_input_value: signal.score_input_value,
                score_input_unit: signal.score_input_unit,
                quality_grade,
                contribution: 0.0,
            });
        }

        let dimensions = build_dimension_scores(&mut indicator_risks);
        let structural_score = aggregate_dimension_group(&dimensions, true);
        let trigger_score = aggregate_dimension_group(&dimensions, false);
        let interaction_boost =
            ((structural_score - 60.0).max(0.0) * (trigger_score - 60.0).max(0.0) / 100.0) * 0.25;
        let overall_score =
            (0.55 * structural_score + 0.45 * trigger_score + interaction_boost).clamp(0.0, 100.0);
        let overall_level = RiskLevel::from_score(overall_score);
        let mut top_contributors = indicator_risks
            .iter()
            .filter(|risk| risk.latest_observation.is_some())
            .map(|risk| RiskContributor {
                indicator_id: risk.indicator.indicator_id.clone(),
                display_name: risk.indicator.display_name.clone(),
                dimension: risk.indicator.dimension,
                score: round1(risk.score),
                contribution: round1(risk.contribution),
                explanation: explain_indicator(risk),
            })
            .collect::<Vec<_>>();
        top_contributors.sort_by(|a, b| b.contribution.total_cmp(&a.contribution));
        top_contributors.truncate(5);

        let data_quality_summary = summarize_quality(&indicator_risks);
        let level_reason = build_level_reason(overall_level, &top_contributors);
        let snapshot = RiskSnapshot {
            as_of_date,
            entity_id: entity_id.to_string(),
            market_scope: market_scope.to_string(),
            overall_score: round1(overall_score),
            overall_level,
            structural_score: round1(structural_score),
            trigger_score: round1(trigger_score),
            level_reason,
            dimensions,
            top_contributors: top_contributors.clone(),
            data_quality_summary,
            generated_at: Utc::now(),
            method_version: self.method_version.clone(),
        };

        ScoringOutput {
            snapshot,
            indicator_risks,
        }
    }
}

fn external_proxy_entity(indicator_id: &str) -> Option<&'static str> {
    if indicator_id.starts_with("jp_") {
        Some("jp")
    } else {
        None
    }
}

#[derive(Debug, Clone)]
pub struct ScoringOutput {
    pub snapshot: RiskSnapshot,
    pub indicator_risks: Vec<IndicatorRisk>,
}

#[derive(Debug, Clone)]
struct SignalComputation {
    score: f64,
    percentile: Option<f64>,
    score_basis: String,
    score_input_value: Option<f64>,
    score_input_unit: Option<String>,
}

#[derive(Debug, Clone, Copy)]
enum SignalTransform {
    Level,
    ChangeDays { days: i64 },
    PctChangeDays { days: i64, absolute: bool },
}

#[derive(Debug, Clone, Copy)]
struct SignalSpec {
    transform: SignalTransform,
    scoring_direction: RiskDirection,
    activation: SignalActivation,
    basis_label: &'static str,
    unit_override: Option<&'static str>,
}

#[derive(Debug, Clone, Copy)]
enum SignalActivation {
    Any,
    PositiveOnly,
    NegativeOnly,
}

fn compute_signal(
    indicator: &Indicator,
    history: &[&Observation],
    latest: Option<&Observation>,
) -> SignalComputation {
    let Some(latest) = latest else {
        return SignalComputation {
            score: 0.0,
            percentile: None,
            score_basis: "缺少观测".to_string(),
            score_input_value: None,
            score_input_unit: None,
        };
    };

    if matches!(indicator.risk_direction, RiskDirection::ManualRule) {
        return SignalComputation {
            score: score_manual_rule(&indicator.indicator_id, latest.value),
            percentile: None,
            score_basis: "人工规则".to_string(),
            score_input_value: Some(latest.value),
            score_input_unit: Some(indicator.unit.clone()),
        };
    }

    let spec = signal_spec(indicator);
    let signal_series = build_signal_series(history, spec.transform);
    let signal_values = signal_series
        .iter()
        .filter_map(|(_, value)| value.is_finite().then_some(*value))
        .collect::<Vec<_>>();
    let current_signal_value = signal_series
        .iter()
        .rev()
        .find(|(date, _)| *date == latest.as_of_date)
        .map(|(_, value)| *value);
    let percentile = current_signal_value.map(|value| percentile_rank(&signal_values, value));
    let score = current_signal_value
        .map(|value| {
            if !signal_is_active(value, spec.activation) {
                0.0
            } else {
                score_value(&signal_values, value, spec.scoring_direction).0
            }
        })
        .unwrap_or(0.0);

    SignalComputation {
        score,
        percentile,
        score_basis: spec.basis_label.to_string(),
        score_input_value: current_signal_value,
        score_input_unit: Some(
            spec.unit_override
                .unwrap_or(indicator.unit.as_str())
                .to_string(),
        ),
    }
}

fn signal_spec(indicator: &Indicator) -> SignalSpec {
    match indicator.indicator_id.as_str() {
        "us_real_estate_home_price" => SignalSpec {
            transform: SignalTransform::PctChangeDays {
                days: YOY_DAYS,
                absolute: false,
            },
            scoring_direction: RiskDirection::TwoSided,
            activation: SignalActivation::Any,
            basis_label: "12m同比",
            unit_override: Some("%"),
        },
        "us_real_estate_housing_starts" => SignalSpec {
            transform: SignalTransform::PctChangeDays {
                days: YOY_DAYS,
                absolute: false,
            },
            scoring_direction: RiskDirection::LowerIsRiskier,
            activation: SignalActivation::NegativeOnly,
            basis_label: "12m同比",
            unit_override: Some("%"),
        },
        "us_liquidity_money_supply_m2" => SignalSpec {
            transform: SignalTransform::PctChangeDays {
                days: YOY_DAYS,
                absolute: false,
            },
            scoring_direction: RiskDirection::LowerIsRiskier,
            activation: SignalActivation::NegativeOnly,
            basis_label: "12m同比",
            unit_override: Some("%"),
        },
        "us_external_usdjpy_level" => SignalSpec {
            transform: SignalTransform::PctChangeDays {
                days: 20,
                absolute: true,
            },
            scoring_direction: RiskDirection::HigherIsRiskier,
            activation: SignalActivation::Any,
            basis_label: "20d振幅",
            unit_override: Some("%"),
        },
        _ => match indicator.risk_direction {
            RiskDirection::RisingFastIsRiskier => SignalSpec {
                transform: SignalTransform::ChangeDays {
                    days: default_change_window_days(indicator.frequency),
                },
                scoring_direction: RiskDirection::HigherIsRiskier,
                activation: SignalActivation::PositiveOnly,
                basis_label: "变化幅度",
                unit_override: None,
            },
            RiskDirection::FallingFastIsRiskier => SignalSpec {
                transform: SignalTransform::ChangeDays {
                    days: default_change_window_days(indicator.frequency),
                },
                scoring_direction: RiskDirection::LowerIsRiskier,
                activation: SignalActivation::NegativeOnly,
                basis_label: "变化幅度",
                unit_override: None,
            },
            other => SignalSpec {
                transform: SignalTransform::Level,
                scoring_direction: other,
                activation: SignalActivation::Any,
                basis_label: "原始水平",
                unit_override: None,
            },
        },
    }
}

fn signal_is_active(value: f64, activation: SignalActivation) -> bool {
    match activation {
        SignalActivation::Any => true,
        SignalActivation::PositiveOnly => value > 0.0,
        SignalActivation::NegativeOnly => value < 0.0,
    }
}

fn default_change_window_days(frequency: Frequency) -> i64 {
    match frequency {
        Frequency::Daily => 30,
        Frequency::Weekly => 84,
        Frequency::Monthly => YOY_DAYS,
        Frequency::Quarterly => YOY_DAYS * 2,
        Frequency::Annual => YOY_DAYS * 3,
        Frequency::Event => 30,
    }
}

fn build_signal_series(
    history: &[&Observation],
    transform: SignalTransform,
) -> Vec<(NaiveDate, f64)> {
    match transform {
        SignalTransform::Level => history
            .iter()
            .filter_map(|observation| {
                observation
                    .value
                    .is_finite()
                    .then_some((observation.as_of_date, observation.value))
            })
            .collect(),
        SignalTransform::ChangeDays { days } => difference_series(history, days, false, false),
        SignalTransform::PctChangeDays { days, absolute } => {
            difference_series(history, days, true, absolute)
        }
    }
}

fn difference_series(
    history: &[&Observation],
    lookback_days: i64,
    percent: bool,
    absolute: bool,
) -> Vec<(NaiveDate, f64)> {
    let mut derived = Vec::new();
    if history.is_empty() {
        return derived;
    }

    let mut previous_index = 0_usize;
    for current_index in 0..history.len() {
        let current = history[current_index];
        if !current.value.is_finite() {
            continue;
        }
        let cutoff = current.as_of_date - chrono::Duration::days(lookback_days);
        while previous_index + 1 < current_index && history[previous_index + 1].as_of_date <= cutoff
        {
            previous_index += 1;
        }
        let previous = history[previous_index];
        if previous_index >= current_index
            || previous.as_of_date > cutoff
            || !previous.value.is_finite()
        {
            continue;
        }

        let mut value = if percent {
            if previous.value.abs() <= f64::EPSILON {
                continue;
            }
            (current.value - previous.value) / previous.value.abs() * 100.0
        } else {
            current.value - previous.value
        };
        if absolute {
            value = value.abs();
        }
        if value.is_finite() {
            derived.push((current.as_of_date, value));
        }
    }

    derived
}

pub fn score_value(history: &[f64], value: f64, direction: RiskDirection) -> (f64, Option<f64>) {
    if history.is_empty() || !value.is_finite() {
        return (0.0, None);
    }
    let percentile = percentile_rank(history, value);
    let score = match direction {
        RiskDirection::HigherIsRiskier | RiskDirection::RisingFastIsRiskier => percentile,
        RiskDirection::LowerIsRiskier | RiskDirection::FallingFastIsRiskier => 100.0 - percentile,
        RiskDirection::TwoSided => percentile.max(100.0 - percentile),
        RiskDirection::ManualRule => percentile,
    };
    (score.clamp(0.0, 100.0), Some(percentile))
}

fn score_manual_rule(indicator_id: &str, value: f64) -> f64 {
    if !value.is_finite() || value <= 0.0 {
        return 0.0;
    }

    match indicator_id {
        "us_event_bank_8k_count" => {
            if value >= 5.0 {
                92.0
            } else if value >= 4.0 {
                82.0
            } else if value >= 3.0 {
                68.0
            } else if value >= 2.0 {
                48.0
            } else {
                24.0
            }
        }
        "us_event_risk_keyword_count" => {
            if value >= 6.0 {
                94.0
            } else if value >= 4.0 {
                82.0
            } else if value >= 3.0 {
                66.0
            } else if value >= 2.0 {
                48.0
            } else {
                28.0
            }
        }
        "us_banking_filing_stress_count" => {
            if value >= 4.0 {
                92.0
            } else if value >= 3.0 {
                78.0
            } else if value >= 2.0 {
                60.0
            } else {
                34.0
            }
        }
        "us_event_official_filing_severity" => value.clamp(0.0, 100.0),
        _ => value.clamp(0.0, 100.0),
    }
}

fn percentile_rank(history: &[f64], value: f64) -> f64 {
    let valid_values = history
        .iter()
        .copied()
        .filter(|candidate| candidate.is_finite())
        .collect::<Vec<_>>();
    if valid_values.is_empty() {
        return 0.0;
    }
    let below_or_equal = valid_values
        .iter()
        .filter(|candidate| **candidate <= value)
        .count();
    below_or_equal as f64 / valid_values.len() as f64 * 100.0
}

fn change_since_days(history: &[&Observation], latest_date: NaiveDate, days: i64) -> Option<f64> {
    let cutoff = latest_date - chrono::Duration::days(days);
    history
        .iter()
        .rev()
        .find(|observation| observation.as_of_date <= cutoff)
        .map(|previous| history.last().expect("history has latest").value - previous.value)
}

fn build_dimension_scores(indicator_risks: &mut [IndicatorRisk]) -> Vec<DimensionScore> {
    let mut by_dimension: BTreeMap<RiskDimension, Vec<usize>> = BTreeMap::new();
    for (index, risk) in indicator_risks.iter().enumerate() {
        by_dimension
            .entry(risk.indicator.dimension)
            .or_default()
            .push(index);
    }

    let mut dimensions = Vec::with_capacity(by_dimension.len());
    for (dimension, indexes) in by_dimension {
        let mut weighted_sum = 0.0;
        let mut weight_sum = 0.0;
        let mut max_score = 0.0_f64;
        let mut quality_sum = 0.0;
        let mut quality_count = 0.0;
        for index in indexes.iter().copied() {
            let risk = &indicator_risks[index];
            let weight = risk.quality_grade.scoring_weight();
            weighted_sum += risk.score * weight;
            weight_sum += weight;
            max_score = max_score.max(risk.score);
            if let Some(observation) = &risk.latest_observation {
                quality_sum += observation.quality_score;
                quality_count += 1.0;
            }
        }

        let base_score = if weight_sum > 0.0 {
            weighted_sum / weight_sum
        } else {
            0.0
        };
        let dimension_score =
            (base_score * (1.0 - TAIL_WEIGHT) + max_score * TAIL_WEIGHT).clamp(0.0, 100.0);
        let per_indicator_weight = if indexes.is_empty() {
            0.0
        } else {
            1.0 / indexes.len() as f64
        };
        for index in indexes.iter().copied() {
            indicator_risks[index].contribution =
                indicator_risks[index].score * per_indicator_weight;
        }

        let mut top_contributors = indexes
            .iter()
            .map(|index| {
                let risk = &indicator_risks[*index];
                RiskContributor {
                    indicator_id: risk.indicator.indicator_id.clone(),
                    display_name: risk.indicator.display_name.clone(),
                    dimension,
                    score: round1(risk.score),
                    contribution: round1(risk.contribution),
                    explanation: explain_indicator(risk),
                }
            })
            .collect::<Vec<_>>();
        top_contributors.sort_by(|a, b| b.contribution.total_cmp(&a.contribution));
        top_contributors.truncate(3);

        let quality_score = if quality_count > 0.0 {
            quality_sum / quality_count
        } else {
            0.0
        };
        dimensions.push(DimensionScore {
            dimension,
            label: dimension.label().to_string(),
            score: round1(dimension_score),
            level: RiskLevel::from_score(dimension_score),
            change_30d: None,
            quality_score: round1(quality_score),
            top_contributors,
        });
    }

    dimensions.sort_by(|a, b| b.score.total_cmp(&a.score));
    dimensions
}

fn aggregate_dimension_group(dimensions: &[DimensionScore], structural: bool) -> f64 {
    let selected = dimensions
        .iter()
        .filter(|dimension| dimension.dimension.is_structural() == structural)
        .collect::<Vec<_>>();
    if selected.is_empty() {
        return 0.0;
    }
    selected
        .iter()
        .map(|dimension| dimension.score)
        .sum::<f64>()
        / selected.len() as f64
}

fn summarize_quality(indicator_risks: &[IndicatorRisk]) -> DataQualitySummary {
    let mut total = 0.0;
    let mut count = 0.0;
    let mut stale = 0;
    let mut low_quality = 0;
    let mut prototype = 0;
    let mut blocked = 0;

    for risk in indicator_risks {
        if let Some(observation) = &risk.latest_observation {
            total += observation.quality_score;
            count += 1.0;
            if observation.quality_flags.iter().any(|flag| flag == "stale") {
                stale += 1;
            }
            if observation
                .quality_flags
                .iter()
                .any(|flag| flag == "prototype_source")
            {
                prototype += 1;
            }
        }
        if matches!(risk.quality_grade, QualityGrade::C | QualityGrade::D) {
            low_quality += 1;
        }
        if matches!(risk.quality_grade, QualityGrade::F) {
            blocked += 1;
        }
    }

    let overall_score = if count > 0.0 { total / count } else { 0.0 };
    DataQualitySummary {
        overall_score: round1(overall_score),
        grade: QualityGrade::from_score(overall_score),
        stale_indicator_count: stale,
        low_quality_indicator_count: low_quality,
        prototype_source_count: prototype,
        blocked_indicator_count: blocked,
    }
}

fn build_level_reason(level: RiskLevel, contributors: &[RiskContributor]) -> String {
    let headline = format!("{} {}", level.code(), level.label());
    match contributors.first() {
        Some(top) => format!(
            "{headline} 由 {}、{} 等指标驱动，主要集中在 {}。",
            top.display_name,
            contributors
                .get(1)
                .map(|contributor| contributor.display_name.as_str())
                .unwrap_or("其他风险信号"),
            top.dimension.label()
        ),
        None => format!("{headline}，暂无足够指标形成明确解释。"),
    }
}

fn explain_indicator(risk: &IndicatorRisk) -> String {
    match (
        risk.score_input_value,
        risk.score_input_unit.as_deref(),
        risk.percentile,
    ) {
        (Some(value), Some(unit), Some(percentile)) => format!(
            "{} 按{}评分，当前信号 {}，历史分位 {:.1}，风险分 {:.1}。",
            risk.indicator.display_name,
            risk.score_basis,
            format_signal_value(value, unit),
            percentile,
            risk.score
        ),
        (Some(value), Some(unit), None) => format!(
            "{} 按{}评分，当前信号 {}，风险分 {:.1}。",
            risk.indicator.display_name,
            risk.score_basis,
            format_signal_value(value, unit),
            risk.score
        ),
        _ => format!(
            "{} 当前风险分为 {:.1}，评分口径为 {}。",
            risk.indicator.display_name, risk.score, risk.score_basis
        ),
    }
}

fn format_signal_value(value: f64, unit: &str) -> String {
    match unit {
        "%" | "percent" => format!("{value:.2}%"),
        "index" | "jpy_per_usd" => format!("{value:.2}"),
        "count" => format!("{value:.0}"),
        "score" => format!("{value:.1}"),
        "billions" | "thousands" => format!("{value:.1} {unit}"),
        _ => format!("{value:.2} {unit}"),
    }
}

fn round1(value: f64) -> f64 {
    (value * 10.0).round() / 10.0
}

#[cfg(test)]
mod tests {
    use fc_domain::RiskDirection;

    use chrono::{NaiveDate, Utc};
    use fc_domain::{Frequency, Indicator, Observation, RiskDimension};

    use super::{compute_signal, score_value};

    #[test]
    fn higher_is_riskier_uses_percentile() {
        let (score, percentile) =
            score_value(&[1.0, 2.0, 3.0, 4.0], 4.0, RiskDirection::HigherIsRiskier);
        assert_eq!(score, 100.0);
        assert_eq!(percentile, Some(100.0));
    }

    #[test]
    fn lower_is_riskier_inverts_percentile() {
        let (score, percentile) =
            score_value(&[1.0, 2.0, 3.0, 4.0], 1.0, RiskDirection::LowerIsRiskier);
        assert_eq!(score, 75.0);
        assert_eq!(percentile, Some(25.0));
    }

    #[test]
    fn home_price_uses_yoy_signal_not_raw_level() {
        let indicator = Indicator {
            indicator_id: "us_real_estate_home_price".to_string(),
            display_name: "Case-Shiller 房价指数".to_string(),
            dimension: RiskDimension::RealEstate,
            description: String::new(),
            unit: "index".to_string(),
            frequency: Frequency::Monthly,
            risk_direction: RiskDirection::TwoSided,
            default_source_id: "fred".to_string(),
            quality_tier: "core".to_string(),
        };
        let history = vec![
            observation("us_real_estate_home_price", 2024, 1, 1, 200.0),
            observation("us_real_estate_home_price", 2025, 1, 1, 210.0),
            observation("us_real_estate_home_price", 2026, 1, 1, 220.5),
        ];
        let refs = history.iter().collect::<Vec<_>>();
        let signal = compute_signal(&indicator, &refs, history.last());
        assert_eq!(signal.score_basis, "12m同比");
        assert_eq!(signal.score_input_unit.as_deref(), Some("%"));
        assert!(signal.score_input_value.is_some());
        assert!(signal.score_input_value.unwrap() < 10.0);
    }

    #[test]
    fn rising_fast_series_scores_off_change_not_level() {
        let indicator = Indicator {
            indicator_id: "us_liquidity_effr".to_string(),
            display_name: "有效联邦基金利率".to_string(),
            dimension: RiskDimension::LiquidityFunding,
            description: String::new(),
            unit: "percent".to_string(),
            frequency: Frequency::Daily,
            risk_direction: RiskDirection::RisingFastIsRiskier,
            default_source_id: "fred".to_string(),
            quality_tier: "core".to_string(),
        };
        let history = vec![
            observation("us_liquidity_effr", 2026, 1, 1, 3.0),
            observation("us_liquidity_effr", 2026, 1, 31, 3.1),
            observation("us_liquidity_effr", 2026, 3, 2, 3.7),
        ];
        let refs = history.iter().collect::<Vec<_>>();
        let signal = compute_signal(&indicator, &refs, history.last());
        assert_eq!(signal.score_basis, "变化幅度");
        assert_eq!(signal.score_input_unit.as_deref(), Some("percent"));
        assert!(signal.score_input_value.unwrap() > 0.0);
    }

    fn observation(indicator_id: &str, year: i32, month: u32, day: u32, value: f64) -> Observation {
        Observation {
            indicator_id: indicator_id.to_string(),
            entity_id: "us".to_string(),
            as_of_date: NaiveDate::from_ymd_opt(year, month, day).unwrap(),
            period_start: None,
            period_end: None,
            frequency: Frequency::Daily,
            value,
            unit: "source_unit".to_string(),
            source_id: "fred".to_string(),
            dataset_id: "fred_series_observations".to_string(),
            revision_time: None,
            publication_time: Some(Utc::now()),
            quality_score: 92.0,
            quality_flags: Vec::new(),
        }
    }
}
