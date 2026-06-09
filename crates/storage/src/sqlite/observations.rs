use chrono::NaiveDate;
use fc_domain::{Indicator, Observation};
use sqlx::Row;
use uuid::Uuid;

use crate::{
    format_dimension, format_frequency, format_risk_direction, parse_dimension, parse_frequency,
    parse_risk_direction, StorageError,
};

use super::{
    format_datetime, parse_date, parse_optional_date, parse_optional_datetime,
    ObservationLineageRecord, SqliteStore,
};

impl SqliteStore {
    pub async fn load_indicators(&self) -> Result<Vec<Indicator>, StorageError> {
        let rows = sqlx::query(
            r#"

            SELECT

                indicator_id,

                display_name,

                dimension,

                description,

                unit,

                frequency,

                risk_direction,

                default_source_id,

                quality_tier

            FROM metadata_indicators

            WHERE enabled = 1

            ORDER BY dimension, indicator_id

            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|row| {
                Ok(Indicator {
                    indicator_id: row.try_get("indicator_id")?,

                    display_name: row.try_get("display_name")?,

                    dimension: parse_dimension(row.try_get::<String, _>("dimension")?.as_str())?,

                    description: row.try_get("description")?,

                    unit: row.try_get("unit")?,

                    frequency: parse_frequency(row.try_get::<String, _>("frequency")?.as_str())?,

                    risk_direction: parse_risk_direction(
                        row.try_get::<String, _>("risk_direction")?.as_str(),
                    )?,

                    default_source_id: row.try_get("default_source_id")?,

                    quality_tier: row.try_get("quality_tier")?,
                })
            })
            .collect()
    }

    pub async fn load_observations(
        &self,

        entity_id: &str,

        as_of_date: NaiveDate,
    ) -> Result<Vec<Observation>, StorageError> {
        self.load_observations_for_entities(&[entity_id], as_of_date)
            .await
    }

    pub async fn load_observations_for_entities(
        &self,

        entity_ids: &[&str],

        as_of_date: NaiveDate,
    ) -> Result<Vec<Observation>, StorageError> {
        if entity_ids.is_empty() {
            return Ok(Vec::new());
        }

        let placeholders = (0..entity_ids.len())
            .map(|index| format!("?{}", index + 1))
            .collect::<Vec<_>>()
            .join(", ");

        let date_placeholder = format!("?{}", entity_ids.len() + 1);

        let query = format!(
            r#"

            SELECT

                indicator_id,

                entity_id,

                as_of_date,

                period_start,

                period_end,

                frequency,

                value,

                unit,

                source_id,

                dataset_id,

                revision_time,

                publication_time,

                quality_score,

                quality_flags_json

            FROM (

                SELECT

                    observation.indicator_id,

                    observation.entity_id,

                    observation.as_of_date,

                    observation.period_start,

                    observation.period_end,

                    observation.frequency,

                    observation.value,

                    observation.unit,

                    observation.source_id,

                    observation.dataset_id,

                    observation.revision_time,

                    observation.publication_time,

                    observation.quality_score,

                    observation.quality_flags_json,

                    ROW_NUMBER() OVER (

                        PARTITION BY
                            observation.indicator_id,
                            observation.entity_id,
                            observation.as_of_date,
                            observation.frequency

                        ORDER BY
                            CASE
                                WHEN indicator.default_source_id IS NOT NULL
                                 AND observation.source_id = indicator.default_source_id
                                    THEN 0
                                ELSE 1
                            END,
                            COALESCE(mapping.priority, 9999),
                            observation.quality_score DESC,
                            COALESCE(observation.publication_time, '') DESC,
                            observation.source_id

                    ) AS source_rank

                FROM ts_indicator_observations observation

                LEFT JOIN metadata_indicators indicator
                    ON indicator.indicator_id = observation.indicator_id

                LEFT JOIN metadata_external_indicator_mappings mapping
                    ON mapping.indicator_id = observation.indicator_id
                   AND mapping.source_id = observation.source_id
                   AND mapping.dataset_id = observation.dataset_id

                WHERE observation.entity_id IN ({placeholders})

                  AND observation.as_of_date <= {date_placeholder}

            )

            WHERE source_rank = 1

            ORDER BY indicator_id, entity_id, as_of_date

            "#
        );

        let mut statement = sqlx::query(&query);

        for entity_id in entity_ids {
            statement = statement.bind(entity_id);
        }

        let rows = statement
            .bind(as_of_date.to_string())
            .fetch_all(&self.pool)
            .await?;

        rows.into_iter()
            .map(|row| {
                let flags_json: String = row.try_get("quality_flags_json")?;

                let quality_flags = serde_json::from_str(&flags_json).unwrap_or_default();

                Ok(Observation {
                    indicator_id: row.try_get("indicator_id")?,

                    entity_id: row.try_get("entity_id")?,

                    as_of_date: parse_date(row.try_get::<String, _>("as_of_date")?.as_str())?,

                    period_start: parse_optional_date(
                        row.try_get::<Option<String>, _>("period_start")?,
                    )?,

                    period_end: parse_optional_date(
                        row.try_get::<Option<String>, _>("period_end")?,
                    )?,

                    frequency: parse_frequency(row.try_get::<String, _>("frequency")?.as_str())?,

                    value: row.try_get("value")?,

                    unit: row.try_get("unit")?,

                    source_id: row.try_get("source_id")?,

                    dataset_id: row.try_get("dataset_id")?,

                    revision_time: parse_optional_datetime(Some(
                        row.try_get::<String, _>("revision_time")?,
                    ))?,

                    publication_time: parse_optional_datetime(
                        row.try_get::<Option<String>, _>("publication_time")?,
                    )?,

                    quality_score: row.try_get("quality_score")?,

                    quality_flags,
                })
            })
            .collect()
    }

    pub async fn load_latest_observation_lineage(
        &self,
        indicator_id: &str,
        entity_id: &str,
        as_of_date: NaiveDate,
    ) -> Result<Option<ObservationLineageRecord>, StorageError> {
        let row = sqlx::query(
            r#"
            SELECT
                observation.indicator_id,
                observation.entity_id,
                observation.as_of_date,
                observation.raw_payload_id,
                raw.run_id,
                raw.response_hash,
                raw.raw_file_path,
                raw.fetched_at,
                run.status AS run_status,
                run.records_written
            FROM ts_indicator_observations observation
            LEFT JOIN metadata_indicators indicator
                ON indicator.indicator_id = observation.indicator_id
            LEFT JOIN metadata_external_indicator_mappings mapping
                ON mapping.indicator_id = observation.indicator_id
               AND mapping.source_id = observation.source_id
               AND mapping.dataset_id = observation.dataset_id
            LEFT JOIN raw_responses raw
                ON raw.raw_payload_id = observation.raw_payload_id
            LEFT JOIN ingest_runs run
                ON run.run_id = raw.run_id
            WHERE observation.indicator_id = ?1
              AND observation.entity_id = ?2
              AND observation.as_of_date <= ?3
            ORDER BY
                observation.as_of_date DESC,
                CASE
                    WHEN indicator.default_source_id IS NOT NULL
                     AND observation.source_id = indicator.default_source_id
                        THEN 0
                    ELSE 1
                END,
                COALESCE(mapping.priority, 9999),
                observation.quality_score DESC,
                COALESCE(observation.publication_time, '') DESC,
                observation.source_id
            LIMIT 1
            "#,
        )
        .bind(indicator_id)
        .bind(entity_id)
        .bind(as_of_date.to_string())
        .fetch_optional(&self.pool)
        .await?;

        row.map(|row| {
            Ok(ObservationLineageRecord {
                indicator_id: row.try_get("indicator_id")?,
                entity_id: row.try_get("entity_id")?,
                as_of_date: parse_date(row.try_get::<String, _>("as_of_date")?.as_str())?,
                raw_payload_id: row.try_get("raw_payload_id")?,
                run_id: row.try_get("run_id")?,
                run_status: row.try_get("run_status")?,
                fetched_at: parse_optional_datetime(
                    row.try_get::<Option<String>, _>("fetched_at")?,
                )?,
                records_written: row.try_get("records_written")?,
                response_hash: row.try_get("response_hash")?,
                raw_file_path: row.try_get("raw_file_path")?,
            })
        })
        .transpose()
    }

    pub async fn upsert_indicator(&self, indicator: &Indicator) -> Result<(), StorageError> {
        sqlx::query(
            r#"

            INSERT INTO metadata_indicators (

                indicator_id,

                display_name,

                dimension,

                description,

                unit,

                frequency,

                risk_direction,

                default_source_id,

                quality_tier,

                enabled

            )

            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 1)

            ON CONFLICT(indicator_id) DO UPDATE SET

                display_name = excluded.display_name,

                dimension = excluded.dimension,

                description = excluded.description,

                unit = excluded.unit,

                frequency = excluded.frequency,

                risk_direction = excluded.risk_direction,

                default_source_id = excluded.default_source_id,

                quality_tier = excluded.quality_tier,

                enabled = 1

            "#,
        )
        .bind(&indicator.indicator_id)
        .bind(&indicator.display_name)
        .bind(format_dimension(indicator.dimension))
        .bind(&indicator.description)
        .bind(&indicator.unit)
        .bind(format_frequency(indicator.frequency))
        .bind(format_risk_direction(indicator.risk_direction))
        .bind(&indicator.default_source_id)
        .bind(&indicator.quality_tier)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn insert_observations(
        &self,

        observations: &[Observation],
    ) -> Result<(), StorageError> {
        self.insert_observations_with_raw_payload(observations, None)
            .await
    }

    pub async fn insert_observations_with_raw_payload(
        &self,

        observations: &[Observation],

        raw_payload_id: Option<Uuid>,
    ) -> Result<(), StorageError> {
        let mut transaction = self.pool.begin().await?;

        let raw_payload_id = raw_payload_id.map(|value| value.to_string());

        for observation in observations {
            let quality_flags_json = serde_json::to_string(&observation.quality_flags)
                .unwrap_or_else(|_| "[]".to_string());

            let revision_time = observation
                .revision_time
                .map(format_datetime)
                .unwrap_or_default();

            let vintage_date = observation
                .revision_time
                .map(|datetime| datetime.date_naive().to_string())
                .unwrap_or_else(|| observation.as_of_date.to_string());

            sqlx::query(
                r#"

                INSERT INTO ts_indicator_observations (

                    indicator_id,

                    entity_id,

                    as_of_date,

                    period_start,

                    period_end,

                    frequency,

                    value,

                    unit,

                    source_id,

                    dataset_id,

                    revision_time,

                    publication_time,

                    vintage_date,

                    raw_payload_id,

                    quality_score,

                    quality_flags_json

                )

                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)

                ON CONFLICT(indicator_id, entity_id, as_of_date, frequency, source_id, vintage_date)

                DO UPDATE SET

                    value = excluded.value,

                    unit = excluded.unit,

                    dataset_id = excluded.dataset_id,

                    publication_time = excluded.publication_time,

                    raw_payload_id = excluded.raw_payload_id,

                    quality_score = excluded.quality_score,

                    quality_flags_json = excluded.quality_flags_json

                "#,
            )
            .bind(&observation.indicator_id)
            .bind(&observation.entity_id)
            .bind(observation.as_of_date.to_string())
            .bind(observation.period_start.map(|date| date.to_string()))
            .bind(observation.period_end.map(|date| date.to_string()))
            .bind(format_frequency(observation.frequency))
            .bind(observation.value)
            .bind(&observation.unit)
            .bind(&observation.source_id)
            .bind(&observation.dataset_id)
            .bind(revision_time)
            .bind(observation.publication_time.map(format_datetime))
            .bind(vintage_date)
            .bind(&raw_payload_id)
            .bind(observation.quality_score)
            .bind(quality_flags_json)
            .execute(&mut *transaction)
            .await?;
        }

        transaction.commit().await?;

        Ok(())
    }
}
