use std::{fs, path::Path, str::FromStr};

use chrono::{DateTime, NaiveDate, Utc};
use fc_domain::{Frequency, Indicator, Observation};
use sqlx::{
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
    SqlitePool,
};
use uuid::Uuid;

use crate::{RiskStore, StorageError};

pub const FRED_DATASET_ID: &str = "fred_series_observations";
pub const BOJ_FX_DATASET_ID: &str = "boj_fx_daily";
pub const BOJ_MONEY_MARKET_DATASET_ID: &str = "boj_money_market_rates";
pub const TREASURY_YIELD_DATASET_ID: &str = "treasury_daily_yield_curve";
pub const WORLD_BANK_DATASET_ID: &str = "world_bank_country_indicators";
pub const SEC_SUBMISSIONS_DATASET_ID: &str = "sec_company_submissions";
pub const SEC_EVENTS_DATASET_ID: &str = "sec_filing_events";
pub const GDELT_DOC_DATASET_ID: &str = "gdelt_doc_timeline";

mod feature_snapshots;
mod formal_datasets;
mod helpers;
mod historical_replay;
mod metadata;
mod migrations;
mod observations;
mod operational;
mod prediction_snapshots;
mod releases;
mod rows;
mod seeds;

#[cfg(test)]
mod tests;

use helpers::{
    feature_snapshot_id, formal_dataset_key, formal_dataset_row_id, format_alert_status,
    format_alert_type, format_datetime, format_risk_level, historical_assessment_point_id,
    parse_alert_status, parse_alert_type, parse_date, parse_optional_date, parse_optional_datetime,
    parse_required_datetime, parse_risk_level, prediction_snapshot_id,
};
use rows::{
    map_active_pointer_row, map_feature_snapshot_row, map_formal_dataset_row,
    map_formal_dataset_row_record, map_historical_assessment_point_row,
    map_historical_replay_run_row, map_model_release_row, map_prediction_snapshot_row,
};
use seeds::{
    boj_indicator_seeds, fred_indicator_seeds, gdelt_indicator_seeds, sec_event_indicator_seeds,
    world_bank_indicator_seeds,
};

const SQLITE_INIT_SQL: &str = include_str!("../../../migrations/sqlite/0001_init.sql");

#[derive(Debug, Clone)]
pub struct SqliteStore {
    pool: SqlitePool,
}

#[derive(Debug, Clone)]
pub struct ExternalIndicatorMapping {
    pub indicator_id: String,
    pub external_code: String,
    pub frequency: Frequency,
}

#[derive(Debug, Clone)]
pub struct RawResponseRecord {
    pub raw_payload_id: Uuid,
    pub run_id: Option<String>,
    pub source_id: String,
    pub dataset_id: String,
    pub request_url: String,
    pub request_params_hash: Option<String>,
    pub response_hash: String,
    pub content_type: String,
    pub content_length: i64,
    pub raw_file_path: String,
    pub fetched_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct IngestionRunRecord {
    pub run_id: String,
    pub job_id: Option<String>,
    pub source_id: String,
    pub dataset_id: String,
    pub target_id: Option<String>,
    pub run_mode: String,
    pub status: String,
    pub started_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
    pub attempt: i64,
    pub watermark_before_json: Option<String>,
    pub watermark_after_json: Option<String>,
    pub records_read: i64,
    pub records_written: i64,
    pub error_type: Option<String>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ObservationLineageRecord {
    pub indicator_id: String,
    pub entity_id: String,
    pub as_of_date: NaiveDate,
    pub raw_payload_id: Option<String>,
    pub run_id: Option<String>,
    pub run_status: Option<String>,
    pub fetched_at: Option<DateTime<Utc>>,
    pub records_written: Option<i64>,
    pub response_hash: Option<String>,
    pub raw_file_path: Option<String>,
}

impl SqliteStore {
    pub async fn connect(database_path: impl AsRef<Path>) -> Result<Self, StorageError> {
        let database_path = database_path.as_ref();
        if let Some(parent) = database_path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent).map_err(|error| {
                    StorageError::Database(sqlx::Error::Io(std::io::Error::new(
                        error.kind(),
                        error.to_string(),
                    )))
                })?;
            }
        }
        let options = SqliteConnectOptions::new()
            .filename(database_path)
            .create_if_missing(true);
        Self::connect_options(options).await
    }

    pub async fn connect_url(database_url: &str) -> Result<Self, StorageError> {
        let options = SqliteConnectOptions::from_str(database_url)?.create_if_missing(true);
        Self::connect_options(options).await
    }

    async fn connect_options(options: SqliteConnectOptions) -> Result<Self, StorageError> {
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(options)
            .await?;
        let store = Self { pool };
        store.initialize_connection().await?;
        Ok(store)
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }
}

#[async_trait::async_trait]
impl RiskStore for SqliteStore {
    async fn load_indicators(&self) -> Result<Vec<Indicator>, StorageError> {
        self.load_indicators().await
    }

    async fn load_observations(
        &self,
        entity_id: &str,
        as_of_date: NaiveDate,
    ) -> Result<Vec<Observation>, StorageError> {
        self.load_observations(entity_id, as_of_date).await
    }

    async fn upsert_indicator(&self, indicator: &Indicator) -> Result<(), StorageError> {
        self.upsert_indicator(indicator).await
    }

    async fn insert_observations(&self, observations: &[Observation]) -> Result<(), StorageError> {
        self.insert_observations(observations).await
    }
}
