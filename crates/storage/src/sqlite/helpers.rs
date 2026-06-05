use chrono::{DateTime, NaiveDate, Utc};
use fc_domain::{AlertStatus, AlertType, RiskLevel};

use crate::StorageError;

pub(super) fn parse_date(value: &str) -> Result<NaiveDate, StorageError> {
    NaiveDate::parse_from_str(value, "%Y-%m-%d")
        .map_err(|error| StorageError::Database(sqlx::Error::Decode(Box::new(error))))
}

pub(super) fn parse_optional_date(
    value: Option<String>,
) -> Result<Option<NaiveDate>, StorageError> {
    value
        .filter(|value| !value.is_empty())
        .map(|value| parse_date(&value))
        .transpose()
}

pub(super) fn parse_optional_datetime(
    value: Option<String>,
) -> Result<Option<DateTime<Utc>>, StorageError> {
    value
        .filter(|value| !value.is_empty())
        .map(|value| {
            DateTime::parse_from_rfc3339(&value)
                .map(|datetime| datetime.with_timezone(&Utc))
                .map_err(|error| StorageError::Database(sqlx::Error::Decode(Box::new(error))))
        })
        .transpose()
}

pub(super) fn format_datetime(value: DateTime<Utc>) -> String {
    value.to_rfc3339()
}

pub(super) fn parse_required_datetime(value: &str) -> Result<DateTime<Utc>, StorageError> {
    DateTime::parse_from_rfc3339(value)
        .map(|datetime| datetime.with_timezone(&Utc))
        .map_err(|error| StorageError::Database(sqlx::Error::Decode(Box::new(error))))
}

pub(super) fn prediction_snapshot_id(
    entity_id: &str,
    market_scope: &str,
    as_of_date: NaiveDate,
    release_id: Option<&str>,
    point_in_time_mode: &str,
) -> String {
    format!(
        "{}:{}:{}:{}:{}",
        market_scope,
        entity_id,
        as_of_date,
        release_id.unwrap_or("inline"),
        point_in_time_mode
    )
}

pub(super) fn feature_snapshot_id(
    entity_id: &str,
    market_scope: &str,
    as_of_date: NaiveDate,
    feature_set_version: &str,
    point_in_time_mode: &str,
) -> String {
    format!("{market_scope}:{entity_id}:{as_of_date}:{feature_set_version}:{point_in_time_mode}")
}

pub(super) fn formal_dataset_key(dataset_id: &str, dataset_version: &str) -> String {
    format!("{dataset_id}:{dataset_version}")
}

pub(super) fn formal_dataset_row_id(
    dataset_key: &str,
    as_of_date: NaiveDate,
    split_name: &str,
) -> String {
    format!("{dataset_key}:{split_name}:{as_of_date}")
}

pub(super) fn historical_assessment_point_id(
    replay_run_id: &str,
    entity_id: &str,
    as_of_date: NaiveDate,
) -> String {
    format!("{replay_run_id}:{entity_id}:{as_of_date}")
}

pub(super) fn parse_risk_level(value: &str) -> Result<RiskLevel, StorageError> {
    match value {
        "normal" => Ok(RiskLevel::Normal),
        "watch" => Ok(RiskLevel::Watch),
        "stress" => Ok(RiskLevel::Stress),
        "warning" => Ok(RiskLevel::Warning),
        "crisis" => Ok(RiskLevel::Crisis),
        other => Err(StorageError::UnknownRiskLevel(other.to_string())),
    }
}

pub(super) fn format_risk_level(value: RiskLevel) -> &'static str {
    match value {
        RiskLevel::Normal => "normal",
        RiskLevel::Watch => "watch",
        RiskLevel::Stress => "stress",
        RiskLevel::Warning => "warning",
        RiskLevel::Crisis => "crisis",
    }
}

pub(super) fn parse_alert_type(value: &str) -> Result<AlertType, StorageError> {
    match value {
        "risk_watch" => Ok(AlertType::RiskWatch),
        "risk_stress" => Ok(AlertType::RiskStress),
        "risk_warning" => Ok(AlertType::RiskWarning),
        "risk_crisis" => Ok(AlertType::RiskCrisis),
        "data_quality_issue" => Ok(AlertType::DataQualityIssue),
        "source_health_issue" => Ok(AlertType::SourceHealthIssue),
        other => Err(StorageError::UnknownAlertType(other.to_string())),
    }
}

pub(super) fn format_alert_type(value: AlertType) -> &'static str {
    match value {
        AlertType::RiskWatch => "risk_watch",
        AlertType::RiskStress => "risk_stress",
        AlertType::RiskWarning => "risk_warning",
        AlertType::RiskCrisis => "risk_crisis",
        AlertType::DataQualityIssue => "data_quality_issue",
        AlertType::SourceHealthIssue => "source_health_issue",
    }
}

pub(super) fn parse_alert_status(value: &str) -> Result<AlertStatus, StorageError> {
    match value {
        "open" => Ok(AlertStatus::Open),
        "acknowledged" => Ok(AlertStatus::Acknowledged),
        "monitoring" => Ok(AlertStatus::Monitoring),
        "escalated" => Ok(AlertStatus::Escalated),
        "deescalated" => Ok(AlertStatus::Deescalated),
        "resolved" => Ok(AlertStatus::Resolved),
        "archived" => Ok(AlertStatus::Archived),
        other => Err(StorageError::UnknownAlertStatus(other.to_string())),
    }
}

pub(super) fn format_alert_status(value: AlertStatus) -> &'static str {
    match value {
        AlertStatus::Open => "open",
        AlertStatus::Acknowledged => "acknowledged",
        AlertStatus::Monitoring => "monitoring",
        AlertStatus::Escalated => "escalated",
        AlertStatus::Deescalated => "deescalated",
        AlertStatus::Resolved => "resolved",
        AlertStatus::Archived => "archived",
    }
}
