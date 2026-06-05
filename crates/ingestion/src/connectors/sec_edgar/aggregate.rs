use std::collections::{BTreeMap, HashMap};

use chrono::{DateTime, NaiveDate, NaiveTime, TimeZone, Utc};
use fc_domain::{
    AlertEvent, AlertStatus, AlertType, Observation, RiskContributor, RiskDimension, RiskLevel,
};
use uuid::Uuid;

use super::rules::round1;
use super::types::{DailyAccumulator, DailyAggregate, SecFilingRecord};
use super::{SEC_EVENTS_DATASET_ID, SEC_METHOD_VERSION, SEC_SCOPE};

pub(super) fn build_daily_aggregates(
    start: NaiveDate,
    end: NaiveDate,
    filings: &[SecFilingRecord],
) -> Vec<DailyAggregate> {
    let mut raw_days: HashMap<NaiveDate, DailyAccumulator> = HashMap::new();
    for filing in filings {
        let day = raw_days.entry(filing.filing_date).or_default();
        day.filing_count += 1;
        if filing.form_type == "8-K" {
            day.bank_8k_count += 1;
        }
        day.rule_hit_count += (filing.keyword_hits.len() + filing.rule_hits.len()) as u32;
        if filing.severity >= 40.0
            || !filing.keyword_hits.is_empty()
            || !filing.rule_hits.is_empty()
        {
            day.stress_count += 1;
        }
        day.max_filing_severity = day.max_filing_severity.max(filing.severity);
        day.latest_acceptance_time = match (day.latest_acceptance_time, filing.acceptance_time) {
            (Some(current), Some(candidate)) => Some(current.max(candidate)),
            (None, Some(candidate)) => Some(candidate),
            (current, None) => current,
        };
        *day.entity_score_by_ticker
            .entry(filing.institution.ticker.to_string())
            .or_insert(0.0) += filing.severity;
    }

    let mut result = Vec::new();
    let mut cursor = start;
    while cursor <= end {
        let aggregate = raw_days.remove(&cursor).unwrap_or_default();
        let breadth_boost =
            (aggregate.entity_score_by_ticker.len().saturating_sub(1) as f64 * 4.0).min(12.0);
        let stress_boost = (aggregate.stress_count.saturating_sub(1) as f64 * 6.0).min(18.0);
        let rule_boost = (aggregate.rule_hit_count as f64 * 3.0).min(18.0);
        let severity_index = if aggregate.filing_count == 0 {
            0.0
        } else {
            (aggregate.max_filing_severity + breadth_boost + stress_boost + rule_boost)
                .clamp(0.0, 100.0)
        };

        let mut entity_scores = aggregate
            .entity_score_by_ticker
            .into_iter()
            .collect::<Vec<_>>();
        entity_scores.sort_by(|a, b| b.1.total_cmp(&a.1));
        result.push(DailyAggregate {
            as_of_date: cursor,
            bank_8k_count: aggregate.bank_8k_count,
            rule_hit_count: aggregate.rule_hit_count,
            stress_count: aggregate.stress_count,
            severity_index: round1(severity_index),
            filing_count: aggregate.filing_count,
            latest_acceptance_time: aggregate.latest_acceptance_time,
            entity_scores,
        });
        cursor += chrono::Duration::days(1);
    }

    result
}

pub(super) fn build_observations(
    daily: &[DailyAggregate],
    fetched_at: DateTime<Utc>,
) -> Vec<Observation> {
    let mut observations = Vec::with_capacity(daily.len() * 4);
    for aggregate in daily {
        let mut flags = vec![
            "official_sec_filing_metadata".to_string(),
            "sec_rule_aggregate".to_string(),
        ];
        if aggregate.filing_count == 0 {
            flags.push("synthetic_zero_fill".to_string());
        }
        observations.push(build_observation(
            "us_event_bank_8k_count",
            aggregate.as_of_date,
            aggregate.bank_8k_count as f64,
            "count",
            aggregate.latest_acceptance_time.unwrap_or(fetched_at),
            &flags,
        ));
        observations.push(build_observation(
            "us_event_risk_keyword_count",
            aggregate.as_of_date,
            aggregate.rule_hit_count as f64,
            "count",
            aggregate.latest_acceptance_time.unwrap_or(fetched_at),
            &flags,
        ));
        observations.push(build_observation(
            "us_banking_filing_stress_count",
            aggregate.as_of_date,
            aggregate.stress_count as f64,
            "count",
            aggregate.latest_acceptance_time.unwrap_or(fetched_at),
            &flags,
        ));
        observations.push(build_observation(
            "us_event_official_filing_severity",
            aggregate.as_of_date,
            aggregate.severity_index,
            "score",
            aggregate.latest_acceptance_time.unwrap_or(fetched_at),
            &flags,
        ));
    }
    observations
}

fn build_observation(
    indicator_id: &str,
    as_of_date: NaiveDate,
    value: f64,
    unit: &str,
    fetched_at: DateTime<Utc>,
    flags: &[String],
) -> Observation {
    Observation {
        indicator_id: indicator_id.to_string(),
        entity_id: "us".to_string(),
        as_of_date,
        period_start: Some(as_of_date),
        period_end: Some(as_of_date),
        frequency: fc_domain::Frequency::Daily,
        value,
        unit: unit.to_string(),
        source_id: "sec_edgar".to_string(),
        dataset_id: SEC_EVENTS_DATASET_ID.to_string(),
        revision_time: None,
        publication_time: Some(fetched_at),
        quality_score: 88.0,
        quality_flags: flags.to_vec(),
    }
}

pub(super) fn build_alerts(daily: &[DailyAggregate], end: NaiveDate) -> Vec<AlertEvent> {
    let mut alerts = Vec::new();
    let severity_by_date = daily
        .iter()
        .map(|aggregate| (aggregate.as_of_date, aggregate.severity_index))
        .collect::<BTreeMap<_, _>>();
    for aggregate in daily {
        if aggregate.severity_index < 30.0
            && aggregate.stress_count < 2
            && aggregate.rule_hit_count < 3
        {
            continue;
        }
        let level = RiskLevel::from_score(aggregate.severity_index);
        let event_type = match level {
            RiskLevel::Crisis => AlertType::RiskCrisis,
            RiskLevel::Warning => AlertType::RiskWarning,
            RiskLevel::Stress => AlertType::RiskStress,
            RiskLevel::Watch | RiskLevel::Normal => AlertType::RiskWatch,
        };
        let previous_score = aggregate
            .as_of_date
            .checked_sub_signed(chrono::Duration::days(1))
            .and_then(|date| severity_by_date.get(&date).copied())
            .filter(|score| *score > 0.0);
        let entity_total = aggregate
            .entity_scores
            .iter()
            .map(|(_, score)| *score)
            .sum::<f64>()
            .max(1.0);
        let top_contributors = aggregate
            .entity_scores
            .iter()
            .take(3)
            .map(|(ticker, score)| RiskContributor {
                indicator_id: "us_event_official_filing_severity".to_string(),
                display_name: format!("{ticker} SEC filings"),
                dimension: RiskDimension::EventsSentiment,
                score: round1(*score),
                contribution: round1((*score / entity_total) * aggregate.severity_index),
                explanation: format!(
                    "{ticker} filings contributed {:.1} to the SEC event cluster.",
                    *score
                ),
            })
            .collect::<Vec<_>>();
        let major_tickers = aggregate
            .entity_scores
            .iter()
            .take(3)
            .map(|(ticker, _)| ticker.as_str())
            .collect::<Vec<_>>();
        let trigger_reason = format!(
            "SEC 白名单机构公告出现聚集：{} 个银行 8-K、{} 个压力 filing、{} 个风险规则命中。主要机构：{}。",
            aggregate.bank_8k_count,
            aggregate.stress_count,
            aggregate.rule_hit_count,
            if major_tickers.is_empty() {
                "无".to_string()
            } else {
                major_tickers.join("、")
            }
        );

        alerts.push(AlertEvent {
            alert_id: Uuid::new_v5(
                &Uuid::NAMESPACE_URL,
                format!("{SEC_SCOPE}:{}", aggregate.as_of_date).as_bytes(),
            ),
            event_type,
            scope: SEC_SCOPE.to_string(),
            entity_id: "us".to_string(),
            dimension: Some(RiskDimension::EventsSentiment),
            level,
            status: if (end - aggregate.as_of_date).num_days() <= 7 {
                AlertStatus::Open
            } else {
                AlertStatus::Monitoring
            },
            triggered_at: aggregate.latest_acceptance_time.unwrap_or_else(|| {
                Utc.from_utc_datetime(
                    &aggregate
                        .as_of_date
                        .and_time(NaiveTime::from_hms_opt(23, 59, 59).expect("valid time")),
                )
            }),
            triggered_as_of_date: aggregate.as_of_date,
            resolved_at: None,
            score: aggregate.severity_index,
            previous_score,
            trigger_reason,
            top_contributors,
            related_indicators: vec![
                "us_event_bank_8k_count".to_string(),
                "us_event_risk_keyword_count".to_string(),
                "us_banking_filing_stress_count".to_string(),
                "us_event_official_filing_severity".to_string(),
            ],
            method_version: SEC_METHOD_VERSION.to_string(),
        });
    }

    alerts.sort_by(|a, b| {
        b.triggered_as_of_date
            .cmp(&a.triggered_as_of_date)
            .then_with(|| b.score.total_cmp(&a.score))
    });
    alerts
}
