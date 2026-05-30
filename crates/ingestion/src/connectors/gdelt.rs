use std::time::Duration;

use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};
use fc_domain::{
    AlertEvent, AlertStatus, AlertType, Frequency, Observation, RiskDimension, RiskLevel,
};
use serde::Deserialize;
use url::Url;
use uuid::Uuid;

use crate::ConnectorError;

const GDELT_SCOPE: &str = "gdelt_daily";
const GDELT_METHOD_VERSION: &str = "gdelt_doc_rules_v1_20260531";
const GDELT_QUERY: &str = "(\"regional bank\" OR \"bank liquidity\" OR \"funding stress\" OR \"deposit outflow\" OR \"credit stress\" OR \"bank run\" OR FDIC)";

#[derive(Debug, Clone)]
pub struct GdeltBackfill {
    pub payload_url: String,
    pub payload_body: String,
    pub observations: Vec<Observation>,
    pub alerts: Vec<AlertEvent>,
    pub latest_date: Option<NaiveDate>,
}

#[derive(Debug, Clone)]
pub struct GdeltConnector {
    client: reqwest::Client,
    base_url: Url,
}

impl GdeltConnector {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .user_agent("financial-crisis-research/0.1")
                .http1_only()
                .no_proxy()
                .timeout(Duration::from_secs(30))
                .build()
                .expect("valid GDELT reqwest client"),
            base_url: Url::parse("https://api.gdeltproject.org/api/v2/doc/doc")
                .expect("valid GDELT DOC API URL"),
        }
    }

    pub async fn backfill_range(
        &self,
        start: NaiveDate,
        end: NaiveDate,
    ) -> Result<GdeltBackfill, ConnectorError> {
        if start > end {
            return Err(ConnectorError::InvalidRequest(
                "GDELT backfill start must be on or before end".to_string(),
            ));
        }
        let days = (end - start).num_days() + 1;
        if days > 90 {
            return Err(ConnectorError::InvalidRequest(
                "GDELT DOC API backfill is limited to 90 days per request".to_string(),
            ));
        }

        let url = self.build_url(start, end)?;
        let response = self
            .client
            .get(url.clone())
            .send()
            .await
            .map_err(|error| ConnectorError::TemporaryNetwork(format!("{error:?}")))?;
        let status = response.status();
        if status.as_u16() == 429 {
            return Err(ConnectorError::RateLimited);
        }
        if status.is_server_error() {
            return Err(ConnectorError::SourceUnavailable(status.to_string()));
        }
        if !status.is_success() {
            return Err(ConnectorError::InvalidRequest(status.to_string()));
        }

        let body = response
            .text()
            .await
            .map_err(|error| ConnectorError::TemporaryNetwork(format!("{error:?}")))?;
        if body.contains("Please limit requests to one every 5 seconds") {
            return Err(ConnectorError::RateLimited);
        }

        let parsed: GdeltTimelineResponse = serde_json::from_str(&body)
            .map_err(|error| ConnectorError::Parse(error.to_string()))?;
        let observations = build_observations(&parsed.timeline)?;
        let alerts = build_alerts(&parsed.timeline)?;
        let latest_date = observations
            .iter()
            .map(|observation| observation.as_of_date)
            .max();

        Ok(GdeltBackfill {
            payload_url: url.to_string(),
            payload_body: body,
            observations,
            alerts,
            latest_date,
        })
    }

    fn build_url(&self, start: NaiveDate, end: NaiveDate) -> Result<Url, ConnectorError> {
        let mut url = self.base_url.clone();
        {
            let mut query = url.query_pairs_mut();
            query.append_pair("query", GDELT_QUERY);
            query.append_pair("mode", "timelinevolraw");
            query.append_pair("format", "json");
            query.append_pair("maxrecords", "250");
            query.append_pair(
                "startdatetime",
                &format!("{}000000", start.format("%Y%m%d")),
            );
            query.append_pair("enddatetime", &format!("{}235959", end.format("%Y%m%d")));
        }
        Ok(url)
    }
}

impl Default for GdeltConnector {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Deserialize)]
struct GdeltTimelineResponse {
    timeline: Vec<GdeltTimelineSeries>,
}

#[derive(Debug, Deserialize)]
struct GdeltTimelineSeries {
    series: String,
    data: Vec<GdeltTimelinePoint>,
}

#[derive(Debug, Deserialize, Clone)]
struct GdeltTimelinePoint {
    date: String,
    value: f64,
    norm: Option<f64>,
}

fn build_observations(series: &[GdeltTimelineSeries]) -> Result<Vec<Observation>, ConnectorError> {
    let points = article_count_points(series)?;
    points
        .iter()
        .map(|point| {
            let as_of_date = parse_gdelt_date(&point.date)?;
            Ok(Observation {
                indicator_id: "global_news_financial_stress_count".to_string(),
                entity_id: "us".to_string(),
                as_of_date,
                period_start: Some(as_of_date),
                period_end: Some(as_of_date),
                frequency: Frequency::Daily,
                value: point.value,
                unit: "count".to_string(),
                source_id: "gdelt".to_string(),
                dataset_id: "gdelt_doc_timeline".to_string(),
                revision_time: None,
                publication_time: Some(Utc::now()),
                quality_score: 68.0,
                quality_flags: vec![
                    "gdelt_doc_api".to_string(),
                    "prototype_source".to_string(),
                    "news_aggregate_only".to_string(),
                ],
            })
        })
        .collect::<Result<Vec<_>, ConnectorError>>()
}

fn build_alerts(series: &[GdeltTimelineSeries]) -> Result<Vec<AlertEvent>, ConnectorError> {
    let points = article_count_points(series)?;
    let mut alerts = Vec::new();
    for point in points {
        let as_of_date = parse_gdelt_date(&point.date)?;
        let density = relative_density(point);
        let count = point.value;
        let Some((level, score)) = gdelt_alert_level(count, density) else {
            continue;
        };
        let event_type = match level {
            RiskLevel::Crisis => AlertType::RiskCrisis,
            RiskLevel::Warning => AlertType::RiskWarning,
            RiskLevel::Stress => AlertType::RiskStress,
            RiskLevel::Watch | RiskLevel::Normal => AlertType::RiskWatch,
        };
        alerts.push(AlertEvent {
            alert_id: Uuid::new_v5(
                &Uuid::NAMESPACE_URL,
                format!("{GDELT_SCOPE}:{as_of_date}").as_bytes(),
            ),
            event_type,
            scope: GDELT_SCOPE.to_string(),
            entity_id: "us".to_string(),
            dimension: Some(RiskDimension::EventsSentiment),
            level,
            status: AlertStatus::Monitoring,
            triggered_at: NaiveDateTime::parse_from_str(&point.date, "%Y%m%dT%H%M%SZ")
                .map(|datetime| DateTime::<Utc>::from_naive_utc_and_offset(datetime, Utc))
                .unwrap_or_else(|_| Utc::now()),
            triggered_as_of_date: as_of_date,
            resolved_at: None,
            score,
            previous_score: None,
            trigger_reason: format!(
                "GDELT 金融压力新闻热度抬升：当日 {} 篇，标准化密度 {:.1}。该源仅作为低置信辅助信号。",
                count.round() as i64,
                density
            ),
            top_contributors: Vec::new(),
            related_indicators: vec!["global_news_financial_stress_count".to_string()],
            method_version: GDELT_METHOD_VERSION.to_string(),
        });
    }
    alerts.sort_by(|a, b| b.triggered_as_of_date.cmp(&a.triggered_as_of_date));
    Ok(alerts)
}

fn article_count_points(
    series: &[GdeltTimelineSeries],
) -> Result<&[GdeltTimelinePoint], ConnectorError> {
    series
        .iter()
        .find(|entry| entry.series == "Article Count")
        .or_else(|| series.first())
        .map(|entry| entry.data.as_slice())
        .ok_or_else(|| ConnectorError::SchemaChanged("missing GDELT timeline series".to_string()))
}

fn relative_density(point: &GdeltTimelinePoint) -> f64 {
    match point.norm {
        Some(norm) if norm > 0.0 => (point.value / norm * 1000.0).clamp(0.0, 999.0),
        _ => point.value,
    }
}

fn gdelt_alert_level(count: f64, density: f64) -> Option<(RiskLevel, f64)> {
    if count >= 18_000.0 || density >= 95.0 {
        Some((RiskLevel::Warning, 74.0))
    } else if count >= 14_000.0 || density >= 82.0 {
        Some((RiskLevel::Stress, 62.0))
    } else if count >= 10_000.0 || density >= 72.0 {
        Some((RiskLevel::Watch, 42.0))
    } else {
        None
    }
}

fn parse_gdelt_date(value: &str) -> Result<NaiveDate, ConnectorError> {
    NaiveDateTime::parse_from_str(value, "%Y%m%dT%H%M%SZ")
        .map(|datetime| datetime.date())
        .map_err(|error| ConnectorError::Parse(error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::{
        build_alerts, build_observations, relative_density, GdeltTimelinePoint,
        GdeltTimelineResponse,
    };

    const SAMPLE: &str = r#"{
      "timeline": [
        {
          "series": "Article Count",
          "data": [
            { "date": "20260501T000000Z", "value": 8161, "norm": 113729 },
            { "date": "20260502T000000Z", "value": 15392, "norm": 187280 },
            { "date": "20260503T000000Z", "value": 5236, "norm": 84060 }
          ]
        }
      ]
    }"#;

    #[test]
    fn parses_gdelt_timeline_into_observations_and_alerts() {
        let parsed: GdeltTimelineResponse = serde_json::from_str(SAMPLE).unwrap();
        let observations = build_observations(&parsed.timeline).unwrap();
        let alerts = build_alerts(&parsed.timeline).unwrap();

        assert_eq!(observations.len(), 3);
        assert_eq!(
            observations[0].indicator_id,
            "global_news_financial_stress_count"
        );
        assert_eq!(alerts.len(), 1);
        assert_eq!(
            alerts[0].related_indicators[0],
            "global_news_financial_stress_count"
        );
    }

    #[test]
    fn normalizes_gdelt_density_from_norm_field() {
        let point = GdeltTimelinePoint {
            date: "20260502T000000Z".to_string(),
            value: 15392.0,
            norm: Some(187280.0),
        };

        assert!(relative_density(&point) > 80.0);
    }
}
