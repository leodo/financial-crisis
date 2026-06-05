use std::collections::BTreeMap;

use fc_domain::{
    DataQualitySummary, DimensionScore, IndicatorRisk, QualityGrade, RiskContributor,
    RiskDimension, RiskLevel,
};

use crate::{explain_indicator, round1, TAIL_WEIGHT};

pub(crate) fn build_dimension_scores(indicator_risks: &mut [IndicatorRisk]) -> Vec<DimensionScore> {
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

pub(crate) fn aggregate_dimension_group(dimensions: &[DimensionScore], structural: bool) -> f64 {
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

pub(crate) fn summarize_quality(indicator_risks: &[IndicatorRisk]) -> DataQualitySummary {
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
