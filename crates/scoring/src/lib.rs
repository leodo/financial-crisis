use std::collections::BTreeMap;

use chrono::{NaiveDate, Utc};
use fc_domain::{
    DataQualitySummary, DimensionScore, Indicator, IndicatorRisk, Observation, QualityGrade,
    RiskContributor, RiskDimension, RiskDirection, RiskLevel, RiskSnapshot,
};

const METHOD_VERSION: &str = "scoring_v1_20260530";
const TAIL_WEIGHT: f64 = 0.2;

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
            .filter(|observation| observation.entity_id == entity_id)
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
            let values = history
                .iter()
                .filter_map(|observation| {
                    observation.value.is_finite().then_some(observation.value)
                })
                .collect::<Vec<_>>();
            let (score, percentile) = latest
                .as_ref()
                .map(|observation| {
                    score_value(&values, observation.value, indicator.risk_direction)
                })
                .unwrap_or((0.0, None));
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
                score,
                level: RiskLevel::from_score(score),
                percentile,
                change_30d,
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

#[derive(Debug, Clone)]
pub struct ScoringOutput {
    pub snapshot: RiskSnapshot,
    pub indicator_risks: Vec<IndicatorRisk>,
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
    format!(
        "{} 当前风险分为 {:.1}，方向规则为 {:?}。",
        risk.indicator.display_name, risk.score, risk.indicator.risk_direction
    )
}

fn round1(value: f64) -> f64 {
    (value * 10.0).round() / 10.0
}

#[cfg(test)]
mod tests {
    use fc_domain::RiskDirection;

    use super::score_value;

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
}
