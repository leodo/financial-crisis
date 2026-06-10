use chrono::NaiveDate;
use fc_domain::AlertEvent;
use sqlx::Row;
use uuid::Uuid;

use crate::{format_dimension, parse_dimension, StorageError};

use super::{
    format_alert_status, format_alert_type, format_datetime, format_risk_level, parse_alert_status,
    parse_alert_type, parse_date, parse_optional_date, parse_optional_datetime, parse_risk_level,
    IngestionRunRecord, IngestionSourceHealthSummary, RawResponseRecord, SqliteStore,
};

impl SqliteStore {
    pub async fn load_alerts_recent(
        &self,
        since_date: NaiveDate,
        as_of_date: NaiveDate,
    ) -> Result<Vec<AlertEvent>, StorageError> {
        let rows = sqlx::query(
            r#"
            SELECT
                alert_id,
                event_type,
                scope,
                entity_id,
                dimension,
                level,
                status,
                triggered_at,
                triggered_as_of_date,
                resolved_at,
                score,
                previous_score,
                trigger_reason,
                top_contributors_json,
                related_indicators_json,
                method_version
            FROM alerts_events
            WHERE triggered_as_of_date >= ?1
              AND triggered_as_of_date <= ?2
            ORDER BY triggered_as_of_date DESC, triggered_at DESC
            "#,
        )
        .bind(since_date.to_string())
        .bind(as_of_date.to_string())
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|row| {
                let top_contributors_json: String = row.try_get("top_contributors_json")?;
                let related_indicators_json: String = row.try_get("related_indicators_json")?;
                Ok(AlertEvent {
                    alert_id: Uuid::parse_str(row.try_get::<String, _>("alert_id")?.as_str())
                        .map_err(|error| {
                            StorageError::Database(sqlx::Error::Decode(Box::new(error)))
                        })?,
                    event_type: parse_alert_type(row.try_get::<String, _>("event_type")?.as_str())?,
                    scope: row.try_get("scope")?,
                    entity_id: row.try_get("entity_id")?,
                    dimension: row
                        .try_get::<Option<String>, _>("dimension")?
                        .map(|value| parse_dimension(&value))
                        .transpose()?,
                    level: parse_risk_level(row.try_get::<String, _>("level")?.as_str())?,
                    status: parse_alert_status(row.try_get::<String, _>("status")?.as_str())?,
                    triggered_at: parse_optional_datetime(Some(
                        row.try_get::<String, _>("triggered_at")?,
                    ))?
                    .ok_or_else(|| StorageError::Database(sqlx::Error::RowNotFound))?,
                    triggered_as_of_date: parse_date(
                        row.try_get::<String, _>("triggered_as_of_date")?.as_str(),
                    )?,
                    resolved_at: parse_optional_datetime(
                        row.try_get::<Option<String>, _>("resolved_at")?,
                    )?,
                    score: row.try_get("score")?,
                    previous_score: row.try_get("previous_score")?,
                    trigger_reason: row.try_get("trigger_reason")?,
                    top_contributors: serde_json::from_str(&top_contributors_json)
                        .unwrap_or_default(),
                    related_indicators: serde_json::from_str(&related_indicators_json)
                        .unwrap_or_default(),
                    method_version: row.try_get("method_version")?,
                })
            })
            .collect()
    }

    pub async fn load_watermark_date(
        &self,
        source_id: &str,
        dataset_id: &str,
        target_id: &str,
    ) -> Result<Option<NaiveDate>, StorageError> {
        let row = sqlx::query(
            r#"
            SELECT last_successful_period
            FROM ingest_watermarks
            WHERE source_id = ?1
              AND dataset_id = ?2
              AND target_id = ?3
            "#,
        )
        .bind(source_id)
        .bind(dataset_id)
        .bind(target_id)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => {
                parse_optional_date(row.try_get::<Option<String>, _>("last_successful_period")?)
            }
            None => Ok(None),
        }
    }

    pub async fn replace_alerts_for_scope(
        &self,
        scope: &str,
        start: NaiveDate,
        end: NaiveDate,
        alerts: &[AlertEvent],
    ) -> Result<(), StorageError> {
        let mut transaction = self.pool.begin().await?;
        sqlx::query(
            r#"
            DELETE FROM alerts_events
            WHERE scope = ?1
              AND triggered_as_of_date >= ?2
              AND triggered_as_of_date <= ?3
            "#,
        )
        .bind(scope)
        .bind(start.to_string())
        .bind(end.to_string())
        .execute(&mut *transaction)
        .await?;

        for alert in alerts {
            let top_contributors_json =
                serde_json::to_string(&alert.top_contributors).unwrap_or_else(|_| "[]".to_string());
            let related_indicators_json = serde_json::to_string(&alert.related_indicators)
                .unwrap_or_else(|_| "[]".to_string());
            sqlx::query(
                r#"
                INSERT INTO alerts_events (
                    alert_id,
                    event_type,
                    scope,
                    entity_id,
                    dimension,
                    level,
                    status,
                    triggered_at,
                    triggered_as_of_date,
                    resolved_at,
                    score,
                    previous_score,
                    trigger_reason,
                    top_contributors_json,
                    related_indicators_json,
                    method_version
                )
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)
                ON CONFLICT(alert_id) DO UPDATE SET
                    event_type = excluded.event_type,
                    scope = excluded.scope,
                    entity_id = excluded.entity_id,
                    dimension = excluded.dimension,
                    level = excluded.level,
                    status = excluded.status,
                    triggered_at = excluded.triggered_at,
                    triggered_as_of_date = excluded.triggered_as_of_date,
                    resolved_at = excluded.resolved_at,
                    score = excluded.score,
                    previous_score = excluded.previous_score,
                    trigger_reason = excluded.trigger_reason,
                    top_contributors_json = excluded.top_contributors_json,
                    related_indicators_json = excluded.related_indicators_json,
                    method_version = excluded.method_version
                "#,
            )
            .bind(alert.alert_id.to_string())
            .bind(format_alert_type(alert.event_type))
            .bind(&alert.scope)
            .bind(&alert.entity_id)
            .bind(alert.dimension.map(format_dimension))
            .bind(format_risk_level(alert.level))
            .bind(format_alert_status(alert.status))
            .bind(format_datetime(alert.triggered_at))
            .bind(alert.triggered_as_of_date.to_string())
            .bind(alert.resolved_at.map(format_datetime))
            .bind(alert.score)
            .bind(alert.previous_score)
            .bind(&alert.trigger_reason)
            .bind(top_contributors_json)
            .bind(related_indicators_json)
            .bind(&alert.method_version)
            .execute(&mut *transaction)
            .await?;
        }

        transaction.commit().await?;
        Ok(())
    }

    pub async fn insert_raw_response(
        &self,
        record: &RawResponseRecord,
    ) -> Result<(), StorageError> {
        sqlx::query(
            r#"
            INSERT INTO raw_responses (
                raw_payload_id,
                run_id,
                source_id,
                dataset_id,
                request_url,
                request_params_hash,
                response_hash,
                content_type,
                content_length,
                raw_file_path,
                fetched_at
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
            ON CONFLICT(raw_payload_id) DO UPDATE SET
                run_id = excluded.run_id,
                response_hash = excluded.response_hash,
                content_length = excluded.content_length,
                raw_file_path = excluded.raw_file_path,
                fetched_at = excluded.fetched_at
            "#,
        )
        .bind(record.raw_payload_id.to_string())
        .bind(&record.run_id)
        .bind(&record.source_id)
        .bind(&record.dataset_id)
        .bind(&record.request_url)
        .bind(&record.request_params_hash)
        .bind(&record.response_hash)
        .bind(&record.content_type)
        .bind(record.content_length)
        .bind(&record.raw_file_path)
        .bind(format_datetime(record.fetched_at))
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn insert_ingestion_run(
        &self,
        record: &IngestionRunRecord,
    ) -> Result<(), StorageError> {
        sqlx::query(
            r#"
            INSERT INTO ingest_runs (
                run_id,
                job_id,
                source_id,
                dataset_id,
                target_id,
                run_mode,
                status,
                started_at,
                finished_at,
                attempt,
                watermark_before_json,
                watermark_after_json,
                records_read,
                records_written,
                error_type,
                error_message
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)
            ON CONFLICT(run_id) DO UPDATE SET
                job_id = excluded.job_id,
                source_id = excluded.source_id,
                dataset_id = excluded.dataset_id,
                target_id = excluded.target_id,
                run_mode = excluded.run_mode,
                status = excluded.status,
                started_at = excluded.started_at,
                finished_at = excluded.finished_at,
                attempt = excluded.attempt,
                watermark_before_json = excluded.watermark_before_json,
                watermark_after_json = excluded.watermark_after_json,
                records_read = excluded.records_read,
                records_written = excluded.records_written,
                error_type = excluded.error_type,
                error_message = excluded.error_message
            "#,
        )
        .bind(&record.run_id)
        .bind(&record.job_id)
        .bind(&record.source_id)
        .bind(&record.dataset_id)
        .bind(&record.target_id)
        .bind(&record.run_mode)
        .bind(&record.status)
        .bind(format_datetime(record.started_at))
        .bind(record.finished_at.map(format_datetime))
        .bind(record.attempt)
        .bind(&record.watermark_before_json)
        .bind(&record.watermark_after_json)
        .bind(record.records_read)
        .bind(record.records_written)
        .bind(&record.error_type)
        .bind(&record.error_message)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn load_ingestion_source_health_summaries(
        &self,
    ) -> Result<Vec<IngestionSourceHealthSummary>, StorageError> {
        let rows = sqlx::query(
            r#"
            WITH source_summary AS (
                SELECT
                    source_id,
                    COUNT(*) AS total_run_count,
                    SUM(CASE WHEN status = 'success' THEN 1 ELSE 0 END) AS successful_run_count,
                    SUM(CASE WHEN status = 'failed' THEN 1 ELSE 0 END) AS failed_run_count,
                    MAX(CASE WHEN status = 'success' THEN finished_at END) AS last_success_at
                FROM ingest_runs
                GROUP BY source_id
            ),
            latest_run AS (
                SELECT
                    source_id,
                    dataset_id,
                    target_id,
                    status,
                    started_at,
                    finished_at,
                    error_message
                FROM (
                    SELECT
                        source_id,
                        dataset_id,
                        target_id,
                        status,
                        started_at,
                        finished_at,
                        error_message,
                        ROW_NUMBER() OVER (
                            PARTITION BY source_id
                            ORDER BY started_at DESC, run_id DESC
                        ) AS row_number
                    FROM ingest_runs
                )
                WHERE row_number = 1
            ),
            watermark_summary AS (
                SELECT
                    source_id,
                    MAX(last_successful_period) AS last_successful_period
                FROM ingest_watermarks
                GROUP BY source_id
            )
            SELECT
                summary.source_id,
                latest.dataset_id AS latest_dataset_id,
                latest.target_id AS latest_target_id,
                latest.status AS latest_status,
                latest.started_at AS latest_started_at,
                latest.finished_at AS latest_finished_at,
                summary.last_success_at,
                watermark.last_successful_period,
                summary.total_run_count,
                summary.successful_run_count,
                summary.failed_run_count,
                (
                    SELECT COUNT(*)
                    FROM ingest_runs failures
                    WHERE failures.source_id = summary.source_id
                      AND failures.status = 'failed'
                      AND (
                          summary.last_success_at IS NULL
                          OR failures.started_at > summary.last_success_at
                      )
                ) AS failures_after_last_success,
                latest.error_message AS latest_error_message
            FROM source_summary summary
            LEFT JOIN latest_run latest
                ON latest.source_id = summary.source_id
            LEFT JOIN watermark_summary watermark
                ON watermark.source_id = summary.source_id
            ORDER BY summary.source_id
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|row| {
                Ok(IngestionSourceHealthSummary {
                    source_id: row.try_get("source_id")?,
                    latest_dataset_id: row.try_get("latest_dataset_id")?,
                    latest_target_id: row.try_get("latest_target_id")?,
                    latest_status: row.try_get("latest_status")?,
                    latest_started_at: parse_optional_datetime(
                        row.try_get::<Option<String>, _>("latest_started_at")?,
                    )?,
                    latest_finished_at: parse_optional_datetime(
                        row.try_get::<Option<String>, _>("latest_finished_at")?,
                    )?,
                    last_success_at: parse_optional_datetime(
                        row.try_get::<Option<String>, _>("last_success_at")?,
                    )?,
                    last_successful_period: parse_optional_date(
                        row.try_get::<Option<String>, _>("last_successful_period")?,
                    )?,
                    total_run_count: row.try_get("total_run_count")?,
                    successful_run_count: row.try_get("successful_run_count")?,
                    failed_run_count: row.try_get("failed_run_count")?,
                    failures_after_last_success: row.try_get("failures_after_last_success")?,
                    latest_error_message: row.try_get("latest_error_message")?,
                })
            })
            .collect()
    }

    pub async fn upsert_watermark(
        &self,
        source_id: &str,
        dataset_id: &str,
        target_id: &str,
        last_successful_period: NaiveDate,
    ) -> Result<(), StorageError> {
        sqlx::query(
            r#"
            INSERT INTO ingest_watermarks (
                source_id,
                dataset_id,
                target_id,
                last_successful_period,
                updated_at
            )
            VALUES (?1, ?2, ?3, ?4, CURRENT_TIMESTAMP)
            ON CONFLICT(source_id, dataset_id, target_id) DO UPDATE SET
                last_successful_period = excluded.last_successful_period,
                updated_at = CURRENT_TIMESTAMP
            "#,
        )
        .bind(source_id)
        .bind(dataset_id)
        .bind(target_id)
        .bind(last_successful_period.to_string())
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}
