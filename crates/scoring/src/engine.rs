use std::collections::BTreeMap;

use chrono::{NaiveDate, Utc};
use fc_domain::{
    Indicator, IndicatorRisk, Observation, QualityGrade, RiskContributor, RiskLevel, RiskSnapshot,
};

use crate::{
    aggregate_dimension_group, build_dimension_scores, build_level_reason, change_since_days,
    compute_signal, explain_indicator, round1, METHOD_VERSION,
};

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
        self.score_with_observation_filter(
            indicators,
            observations,
            as_of_date,
            entity_id,
            market_scope,
            |_| true,
        )
    }

    pub fn score_with_observation_filter<F>(
        &self,
        indicators: &[Indicator],
        observations: &[Observation],
        as_of_date: NaiveDate,
        entity_id: &str,
        market_scope: &str,
        observation_filter: F,
    ) -> ScoringOutput
    where
        F: Fn(&Observation) -> bool,
    {
        let mut by_indicator: BTreeMap<&str, Vec<&Observation>> = BTreeMap::new();
        for observation in observations
            .iter()
            .filter(|observation| {
                observation.entity_id == entity_id
                    || external_proxy_entity(&observation.indicator_id)
                        == Some(observation.entity_id.as_str())
            })
            .filter(|observation| observation.as_of_date <= as_of_date)
            .filter(|observation| observation_filter(observation))
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

        let data_quality_summary = crate::summarize_quality(&indicator_risks);
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
