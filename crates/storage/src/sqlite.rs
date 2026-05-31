use std::{fs, path::Path, str::FromStr};

use chrono::{DateTime, NaiveDate, Utc};
use fc_domain::{
    ActiveModelPointer, AlertEvent, AlertStatus, AlertType, FeatureSnapshotRecord,
    FormalDatasetManifest, FormalDatasetRecord, FormalDatasetRowRecord, Frequency, Indicator,
    ModelReleaseManifest, ModelReleaseRecord, Observation, PredictionSnapshotRecord, RiskDimension,
    RiskDirection, RiskLevel,
};
use sqlx::{
    sqlite::{SqliteConnectOptions, SqlitePoolOptions, SqliteRow},
    Row, SqlitePool,
};
use uuid::Uuid;

use crate::{
    format_dimension, format_frequency, format_risk_direction, parse_dimension, parse_frequency,
    parse_risk_direction, RiskStore, StorageError,
};

pub const FRED_DATASET_ID: &str = "fred_series_observations";
pub const BOJ_FX_DATASET_ID: &str = "boj_fx_daily";
pub const BOJ_MONEY_MARKET_DATASET_ID: &str = "boj_money_market_rates";
pub const TREASURY_YIELD_DATASET_ID: &str = "treasury_daily_yield_curve";
pub const WORLD_BANK_DATASET_ID: &str = "world_bank_country_indicators";
pub const SEC_SUBMISSIONS_DATASET_ID: &str = "sec_company_submissions";
pub const SEC_EVENTS_DATASET_ID: &str = "sec_filing_events";
pub const GDELT_DOC_DATASET_ID: &str = "gdelt_doc_timeline";

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

    pub async fn migrate(&self) -> Result<(), StorageError> {
        for statement in SQLITE_INIT_SQL.split(';') {
            let statement = statement.trim();
            if !statement.is_empty() {
                sqlx::query(statement).execute(&self.pool).await?;
            }
        }
        Ok(())
    }

    pub async fn seed_fred_metadata(&self) -> Result<(), StorageError> {
        sqlx::query(
            r#"
            INSERT INTO metadata_sources (
                source_id,
                display_name,
                source_type,
                official_url,
                documentation_url,
                access_method,
                auth_required,
                auth_secret_ref,
                rate_limit_policy_json,
                license_note,
                commercial_use_status,
                production_allowed,
                enabled
            )
            VALUES (
                'fred',
                'FRED',
                'macro_financial_timeseries',
                'https://fred.stlouisfed.org/',
                'https://fred.stlouisfed.org/graph/fredgraph.csv',
                'graph_csv',
                0,
                NULL,
                '{"policy":"public_graph_csv","note":"No API key; cache locally and keep conservative cadence."}',
                'Use according to FRED source-specific notes; public graph CSV has no vintage fields.',
                'review_required',
                1,
                1
            )
            ON CONFLICT(source_id) DO UPDATE SET
                display_name = excluded.display_name,
                access_method = excluded.access_method,
                documentation_url = excluded.documentation_url,
                auth_required = excluded.auth_required,
                auth_secret_ref = excluded.auth_secret_ref,
                rate_limit_policy_json = excluded.rate_limit_policy_json,
                license_note = excluded.license_note,
                production_allowed = excluded.production_allowed,
                enabled = excluded.enabled,
                updated_at = CURRENT_TIMESTAMP
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            INSERT INTO metadata_datasets (
                dataset_id,
                source_id,
                display_name,
                frequency_set_json,
                region_set_json,
                supports_backfill,
                supports_incremental,
                supports_vintage,
                expected_latency_seconds,
                config_version,
                enabled
            )
            VALUES (
                ?1,
                'fred',
                'FRED series observations',
                '["daily","weekly","monthly","quarterly"]',
                '["us"]',
                1,
                1,
                0,
                86400,
                'fred_graph_csv_seed_v2_20260530',
                1
            )
            ON CONFLICT(dataset_id) DO UPDATE SET
                display_name = excluded.display_name,
                frequency_set_json = excluded.frequency_set_json,
                region_set_json = excluded.region_set_json,
                supports_backfill = excluded.supports_backfill,
                supports_incremental = excluded.supports_incremental,
                supports_vintage = excluded.supports_vintage,
                config_version = excluded.config_version,
                enabled = excluded.enabled
            "#,
        )
        .bind(FRED_DATASET_ID)
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            INSERT INTO metadata_sources (
                source_id,
                display_name,
                source_type,
                official_url,
                documentation_url,
                access_method,
                auth_required,
                auth_secret_ref,
                rate_limit_policy_json,
                license_note,
                commercial_use_status,
                production_allowed,
                enabled
            )
            VALUES (
                'treasury',
                'U.S. Treasury',
                'government_timeseries',
                'https://home.treasury.gov/',
                'https://home.treasury.gov/resource-center/data-chart-center/interest-rates',
                'xml_download',
                0,
                NULL,
                '{"policy":"public_xml","note":"Fetch by month and cache locally."}',
                'Official U.S. Treasury daily yield curve publication.',
                'public_official',
                1,
                1
            )
            ON CONFLICT(source_id) DO UPDATE SET
                display_name = excluded.display_name,
                documentation_url = excluded.documentation_url,
                access_method = excluded.access_method,
                auth_required = excluded.auth_required,
                auth_secret_ref = excluded.auth_secret_ref,
                rate_limit_policy_json = excluded.rate_limit_policy_json,
                license_note = excluded.license_note,
                production_allowed = excluded.production_allowed,
                enabled = excluded.enabled,
                updated_at = CURRENT_TIMESTAMP
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            INSERT INTO metadata_datasets (
                dataset_id,
                source_id,
                display_name,
                frequency_set_json,
                region_set_json,
                supports_backfill,
                supports_incremental,
                supports_vintage,
                expected_latency_seconds,
                config_version,
                enabled
            )
            VALUES (
                ?1,
                'treasury',
                'Daily Treasury yield curve',
                '["daily"]',
                '["us"]',
                1,
                1,
                0,
                86400,
                'treasury_yield_seed_v1_20260530',
                1
            )
            ON CONFLICT(dataset_id) DO UPDATE SET
                display_name = excluded.display_name,
                frequency_set_json = excluded.frequency_set_json,
                region_set_json = excluded.region_set_json,
                supports_backfill = excluded.supports_backfill,
                supports_incremental = excluded.supports_incremental,
                supports_vintage = excluded.supports_vintage,
                config_version = excluded.config_version,
                enabled = excluded.enabled
            "#,
        )
        .bind(TREASURY_YIELD_DATASET_ID)
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            INSERT INTO metadata_sources (
                source_id,
                display_name,
                source_type,
                official_url,
                documentation_url,
                access_method,
                auth_required,
                auth_secret_ref,
                rate_limit_policy_json,
                license_note,
                commercial_use_status,
                production_allowed,
                enabled
            )
            VALUES (
                'world_bank',
                'World Bank Indicators',
                'global_macro',
                'https://api.worldbank.org/',
                'https://datahelpdesk.worldbank.org/knowledgebase/articles/889392',
                'rest_api',
                0,
                NULL,
                '{"policy":"public_rest_api","note":"Annual slow variables; no API key required."}',
                'Official World Bank Indicators API.',
                'public_official',
                1,
                1
            )
            ON CONFLICT(source_id) DO UPDATE SET
                display_name = excluded.display_name,
                documentation_url = excluded.documentation_url,
                access_method = excluded.access_method,
                auth_required = excluded.auth_required,
                auth_secret_ref = excluded.auth_secret_ref,
                rate_limit_policy_json = excluded.rate_limit_policy_json,
                license_note = excluded.license_note,
                production_allowed = excluded.production_allowed,
                enabled = excluded.enabled,
                updated_at = CURRENT_TIMESTAMP
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            INSERT INTO metadata_datasets (
                dataset_id,
                source_id,
                display_name,
                frequency_set_json,
                region_set_json,
                supports_backfill,
                supports_incremental,
                supports_vintage,
                expected_latency_seconds,
                config_version,
                enabled
            )
            VALUES (
                ?1,
                'world_bank',
                'World Bank country indicators',
                '["annual"]',
                '["us"]',
                1,
                1,
                0,
                86400,
                'world_bank_seed_v1_20260530',
                1
            )
            ON CONFLICT(dataset_id) DO UPDATE SET
                display_name = excluded.display_name,
                frequency_set_json = excluded.frequency_set_json,
                region_set_json = excluded.region_set_json,
                supports_backfill = excluded.supports_backfill,
                supports_incremental = excluded.supports_incremental,
                supports_vintage = excluded.supports_vintage,
                config_version = excluded.config_version,
                enabled = excluded.enabled
            "#,
        )
        .bind(WORLD_BANK_DATASET_ID)
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            INSERT INTO metadata_sources (
                source_id,
                display_name,
                source_type,
                official_url,
                documentation_url,
                access_method,
                auth_required,
                auth_secret_ref,
                rate_limit_policy_json,
                license_note,
                commercial_use_status,
                production_allowed,
                enabled
            )
            VALUES (
                'boj',
                'Bank of Japan Statistics API',
                'government_timeseries',
                'https://www.boj.or.jp/en/statistics/',
                'https://www.stat-search.boj.or.jp/info/api_manual_en.pdf',
                'rest_csv',
                0,
                NULL,
                '{"policy":"public_rest_csv","note":"Official BOJ API, no key required. Prefer BOJ for USDJPY and Japan short rates, cache locally."}',
                'Official BOJ statistics API for FX daily and money market time series.',
                'public_official',
                1,
                1
            )
            ON CONFLICT(source_id) DO UPDATE SET
                display_name = excluded.display_name,
                documentation_url = excluded.documentation_url,
                access_method = excluded.access_method,
                auth_required = excluded.auth_required,
                auth_secret_ref = excluded.auth_secret_ref,
                rate_limit_policy_json = excluded.rate_limit_policy_json,
                license_note = excluded.license_note,
                production_allowed = excluded.production_allowed,
                enabled = excluded.enabled,
                updated_at = CURRENT_TIMESTAMP
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            INSERT INTO metadata_datasets (
                dataset_id,
                source_id,
                display_name,
                frequency_set_json,
                region_set_json,
                supports_backfill,
                supports_incremental,
                supports_vintage,
                expected_latency_seconds,
                config_version,
                enabled
            )
            VALUES (
                ?1,
                'boj',
                'BOJ foreign exchange daily series',
                '["daily"]',
                '["jp","us"]',
                1,
                1,
                0,
                86400,
                'boj_fx_seed_v1_20260530',
                1
            )
            ON CONFLICT(dataset_id) DO UPDATE SET
                display_name = excluded.display_name,
                frequency_set_json = excluded.frequency_set_json,
                region_set_json = excluded.region_set_json,
                supports_backfill = excluded.supports_backfill,
                supports_incremental = excluded.supports_incremental,
                supports_vintage = excluded.supports_vintage,
                config_version = excluded.config_version,
                enabled = excluded.enabled
            "#,
        )
        .bind(BOJ_FX_DATASET_ID)
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            INSERT INTO metadata_datasets (
                dataset_id,
                source_id,
                display_name,
                frequency_set_json,
                region_set_json,
                supports_backfill,
                supports_incremental,
                supports_vintage,
                expected_latency_seconds,
                config_version,
                enabled
            )
            VALUES (
                ?1,
                'boj',
                'BOJ money market call rate series',
                '["daily"]',
                '["jp"]',
                1,
                1,
                0,
                86400,
                'boj_money_market_seed_v1_20260530',
                1
            )
            ON CONFLICT(dataset_id) DO UPDATE SET
                display_name = excluded.display_name,
                frequency_set_json = excluded.frequency_set_json,
                region_set_json = excluded.region_set_json,
                supports_backfill = excluded.supports_backfill,
                supports_incremental = excluded.supports_incremental,
                supports_vintage = excluded.supports_vintage,
                config_version = excluded.config_version,
                enabled = excluded.enabled
            "#,
        )
        .bind(BOJ_MONEY_MARKET_DATASET_ID)
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            INSERT INTO metadata_sources (
                source_id,
                display_name,
                source_type,
                official_url,
                documentation_url,
                access_method,
                auth_required,
                auth_secret_ref,
                rate_limit_policy_json,
                license_note,
                commercial_use_status,
                production_allowed,
                enabled
            )
            VALUES (
                'sec_edgar',
                'SEC EDGAR',
                'filings_events',
                'https://www.sec.gov/edgar/search/',
                'https://www.sec.gov/edgar/sec-api-documentation',
                'json_download',
                0,
                NULL,
                '{"policy":"fair_access","note":"Sequential requests, local cache, and archived submissions only when the requested range overlaps."}',
                'Official SEC submissions JSON. Local event features are aggregated from filing metadata only; no paid key required.',
                'public_official',
                1,
                1
            )
            ON CONFLICT(source_id) DO UPDATE SET
                display_name = excluded.display_name,
                documentation_url = excluded.documentation_url,
                access_method = excluded.access_method,
                auth_required = excluded.auth_required,
                auth_secret_ref = excluded.auth_secret_ref,
                rate_limit_policy_json = excluded.rate_limit_policy_json,
                license_note = excluded.license_note,
                production_allowed = excluded.production_allowed,
                enabled = excluded.enabled,
                updated_at = CURRENT_TIMESTAMP
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            INSERT INTO metadata_datasets (
                dataset_id,
                source_id,
                display_name,
                frequency_set_json,
                region_set_json,
                supports_backfill,
                supports_incremental,
                supports_vintage,
                expected_latency_seconds,
                config_version,
                enabled
            )
            VALUES (
                ?1,
                'sec_edgar',
                'SEC company submissions metadata',
                '["event"]',
                '["us"]',
                1,
                1,
                0,
                86400,
                'sec_submissions_seed_v1_20260531',
                1
            )
            ON CONFLICT(dataset_id) DO UPDATE SET
                display_name = excluded.display_name,
                frequency_set_json = excluded.frequency_set_json,
                region_set_json = excluded.region_set_json,
                supports_backfill = excluded.supports_backfill,
                supports_incremental = excluded.supports_incremental,
                supports_vintage = excluded.supports_vintage,
                config_version = excluded.config_version,
                enabled = excluded.enabled
            "#,
        )
        .bind(SEC_SUBMISSIONS_DATASET_ID)
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            INSERT INTO metadata_datasets (
                dataset_id,
                source_id,
                display_name,
                frequency_set_json,
                region_set_json,
                supports_backfill,
                supports_incremental,
                supports_vintage,
                expected_latency_seconds,
                config_version,
                enabled
            )
            VALUES (
                ?1,
                'sec_edgar',
                'SEC filing event aggregates',
                '["daily"]',
                '["us"]',
                1,
                1,
                0,
                86400,
                'sec_events_seed_v1_20260531',
                1
            )
            ON CONFLICT(dataset_id) DO UPDATE SET
                display_name = excluded.display_name,
                frequency_set_json = excluded.frequency_set_json,
                region_set_json = excluded.region_set_json,
                supports_backfill = excluded.supports_backfill,
                supports_incremental = excluded.supports_incremental,
                supports_vintage = excluded.supports_vintage,
                config_version = excluded.config_version,
                enabled = excluded.enabled
            "#,
        )
        .bind(SEC_EVENTS_DATASET_ID)
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            INSERT INTO metadata_sources (
                source_id,
                display_name,
                source_type,
                official_url,
                documentation_url,
                access_method,
                auth_required,
                auth_secret_ref,
                rate_limit_policy_json,
                license_note,
                commercial_use_status,
                production_allowed,
                enabled
            )
            VALUES (
                'gdelt',
                'GDELT',
                'news_events',
                'https://api.gdeltproject.org/',
                'https://blog.gdeltproject.org/gdelt-doc-2-0-api-debuts/amp/',
                'rest_api',
                0,
                NULL,
                '{"policy":"public_doc_api","note":"Strictly one request every 5+ seconds, cache locally, and keep it as a low-confidence auxiliary source."}',
                'Public GDELT DOC API used only for aggregate news counts. Keep it as an auxiliary prototype signal.',
                'review_required',
                0,
                1
            )
            ON CONFLICT(source_id) DO UPDATE SET
                display_name = excluded.display_name,
                documentation_url = excluded.documentation_url,
                access_method = excluded.access_method,
                auth_required = excluded.auth_required,
                auth_secret_ref = excluded.auth_secret_ref,
                rate_limit_policy_json = excluded.rate_limit_policy_json,
                license_note = excluded.license_note,
                production_allowed = excluded.production_allowed,
                enabled = excluded.enabled,
                updated_at = CURRENT_TIMESTAMP
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            INSERT INTO metadata_datasets (
                dataset_id,
                source_id,
                display_name,
                frequency_set_json,
                region_set_json,
                supports_backfill,
                supports_incremental,
                supports_vintage,
                expected_latency_seconds,
                config_version,
                enabled
            )
            VALUES (
                ?1,
                'gdelt',
                'GDELT DOC API timeline aggregates',
                '["daily"]',
                '["us","global"]',
                1,
                1,
                0,
                86400,
                'gdelt_doc_seed_v1_20260531',
                1
            )
            ON CONFLICT(dataset_id) DO UPDATE SET
                display_name = excluded.display_name,
                frequency_set_json = excluded.frequency_set_json,
                region_set_json = excluded.region_set_json,
                supports_backfill = excluded.supports_backfill,
                supports_incremental = excluded.supports_incremental,
                supports_vintage = excluded.supports_vintage,
                config_version = excluded.config_version,
                enabled = excluded.enabled
            "#,
        )
        .bind(GDELT_DOC_DATASET_ID)
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            INSERT INTO metadata_entities (
                entity_id,
                entity_type,
                display_name,
                iso_country_code,
                currency,
                metadata_json
            )
            VALUES ('us', 'country', 'United States', 'USA', 'USD', '{}')
            ON CONFLICT(entity_id) DO UPDATE SET
                display_name = excluded.display_name,
                iso_country_code = excluded.iso_country_code,
                currency = excluded.currency
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            INSERT INTO metadata_entities (
                entity_id,
                entity_type,
                display_name,
                iso_country_code,
                currency,
                metadata_json
            )
            VALUES ('jp', 'country', 'Japan', 'JPN', 'JPY', '{}')
            ON CONFLICT(entity_id) DO UPDATE SET
                display_name = excluded.display_name,
                iso_country_code = excluded.iso_country_code,
                currency = excluded.currency
            "#,
        )
        .execute(&self.pool)
        .await?;

        for seed in fred_indicator_seeds() {
            let indicator = seed.indicator();
            self.upsert_indicator(&indicator).await?;
            self.upsert_fred_mapping(&indicator.indicator_id, seed.external_code, seed.priority)
                .await?;
        }
        for seed in boj_indicator_seeds() {
            let indicator = seed.indicator();
            self.upsert_indicator(&indicator).await?;
            self.upsert_external_mapping(
                &indicator.indicator_id,
                "boj",
                seed.dataset_id,
                seed.external_code,
                seed.priority,
            )
            .await?;
        }
        self.upsert_external_mapping(
            "us_rates_yield_curve_10y2y",
            "treasury",
            TREASURY_YIELD_DATASET_ID,
            "T10Y2Y",
            90,
        )
        .await?;
        for seed in world_bank_indicator_seeds() {
            let indicator = seed.indicator();
            self.upsert_indicator(&indicator).await?;
            self.upsert_external_mapping(
                &indicator.indicator_id,
                "world_bank",
                WORLD_BANK_DATASET_ID,
                seed.external_code,
                100,
            )
            .await?;
        }
        for seed in sec_event_indicator_seeds() {
            let indicator = seed.indicator();
            self.upsert_indicator(&indicator).await?;
        }
        for seed in gdelt_indicator_seeds() {
            let indicator = seed.indicator();
            self.upsert_indicator(&indicator).await?;
        }

        Ok(())
    }

    pub async fn load_fred_mappings(&self) -> Result<Vec<ExternalIndicatorMapping>, StorageError> {
        self.load_external_mappings("fred", FRED_DATASET_ID).await
    }

    pub async fn load_treasury_yield_mappings(
        &self,
    ) -> Result<Vec<ExternalIndicatorMapping>, StorageError> {
        self.load_external_mappings("treasury", TREASURY_YIELD_DATASET_ID)
            .await
    }

    pub async fn load_world_bank_mappings(
        &self,
    ) -> Result<Vec<ExternalIndicatorMapping>, StorageError> {
        self.load_external_mappings("world_bank", WORLD_BANK_DATASET_ID)
            .await
    }

    pub async fn load_jpy_carry_mappings(
        &self,
    ) -> Result<Vec<ExternalIndicatorMapping>, StorageError> {
        let boj = self
            .load_external_mappings("boj", BOJ_FX_DATASET_ID)
            .await?
            .into_iter()
            .filter(|mapping| mapping.indicator_id == "us_external_usdjpy_level");
        let fred = self
            .load_external_mappings("fred", FRED_DATASET_ID)
            .await?
            .into_iter()
            .filter(|mapping| mapping.indicator_id == "us_external_usdjpy_level");
        Ok(boj.chain(fred).collect())
    }

    pub async fn load_boj_money_market_mappings(
        &self,
    ) -> Result<Vec<ExternalIndicatorMapping>, StorageError> {
        self.load_external_mappings("boj", BOJ_MONEY_MARKET_DATASET_ID)
            .await
    }

    pub async fn load_external_mappings(
        &self,
        source_id: &str,
        dataset_id: &str,
    ) -> Result<Vec<ExternalIndicatorMapping>, StorageError> {
        let rows = sqlx::query(
            r#"
            SELECT
                map.indicator_id,
                map.external_code,
                ind.frequency
            FROM metadata_external_indicator_mappings map
            JOIN metadata_indicators ind ON ind.indicator_id = map.indicator_id
            WHERE map.source_id = ?1
              AND map.dataset_id = ?2
              AND ind.enabled = 1
            ORDER BY map.priority, map.indicator_id
            "#,
        )
        .bind(source_id)
        .bind(dataset_id)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|row| {
                Ok(ExternalIndicatorMapping {
                    indicator_id: row.try_get("indicator_id")?,
                    external_code: row.try_get("external_code")?,
                    frequency: parse_frequency(row.try_get::<String, _>("frequency")?.as_str())?,
                })
            })
            .collect()
    }

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

    pub async fn upsert_model_release(
        &self,
        release: &ModelReleaseRecord,
    ) -> Result<(), StorageError> {
        let manifest_json =
            serde_json::to_string(&release.manifest).unwrap_or_else(|_| "{}".to_string());
        sqlx::query(
            r#"
            INSERT INTO analytics_model_releases (
                release_id,
                market_scope,
                status,
                probability_mode,
                serving_status,
                bundle_uri,
                manifest_json,
                feature_set_version,
                label_version,
                prob_model_version,
                calibration_version,
                posture_policy_version,
                action_playbook_version,
                point_in_time_mode,
                training_range_start,
                training_range_end,
                calibration_range_start,
                calibration_range_end,
                evaluation_range_start,
                evaluation_range_end,
                brier_score,
                log_loss,
                ece,
                note,
                created_at,
                activated_at,
                retired_at
            )
            VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18,
                ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26, ?27
            )
            ON CONFLICT(release_id) DO UPDATE SET
                market_scope = excluded.market_scope,
                status = excluded.status,
                probability_mode = excluded.probability_mode,
                serving_status = excluded.serving_status,
                bundle_uri = excluded.bundle_uri,
                manifest_json = excluded.manifest_json,
                feature_set_version = excluded.feature_set_version,
                label_version = excluded.label_version,
                prob_model_version = excluded.prob_model_version,
                calibration_version = excluded.calibration_version,
                posture_policy_version = excluded.posture_policy_version,
                action_playbook_version = excluded.action_playbook_version,
                point_in_time_mode = excluded.point_in_time_mode,
                training_range_start = excluded.training_range_start,
                training_range_end = excluded.training_range_end,
                calibration_range_start = excluded.calibration_range_start,
                calibration_range_end = excluded.calibration_range_end,
                evaluation_range_start = excluded.evaluation_range_start,
                evaluation_range_end = excluded.evaluation_range_end,
                brier_score = excluded.brier_score,
                log_loss = excluded.log_loss,
                ece = excluded.ece,
                note = excluded.note,
                activated_at = excluded.activated_at,
                retired_at = excluded.retired_at
            "#,
        )
        .bind(&release.manifest.release_id)
        .bind(&release.manifest.market_scope)
        .bind(&release.manifest.status)
        .bind(&release.manifest.probability_mode)
        .bind(&release.manifest.serving_status)
        .bind(&release.manifest.bundle_uri)
        .bind(manifest_json)
        .bind(&release.manifest.feature_set_version)
        .bind(&release.manifest.label_version)
        .bind(&release.manifest.prob_model_version)
        .bind(&release.manifest.calibration_version)
        .bind(&release.manifest.posture_policy_version)
        .bind(&release.manifest.action_playbook_version)
        .bind(&release.manifest.point_in_time_mode)
        .bind(
            release
                .manifest
                .training_range_start
                .map(|date| date.to_string()),
        )
        .bind(
            release
                .manifest
                .training_range_end
                .map(|date| date.to_string()),
        )
        .bind(
            release
                .manifest
                .calibration_range_start
                .map(|date| date.to_string()),
        )
        .bind(
            release
                .manifest
                .calibration_range_end
                .map(|date| date.to_string()),
        )
        .bind(
            release
                .manifest
                .evaluation_range_start
                .map(|date| date.to_string()),
        )
        .bind(
            release
                .manifest
                .evaluation_range_end
                .map(|date| date.to_string()),
        )
        .bind(release.manifest.brier_score)
        .bind(release.manifest.log_loss)
        .bind(release.manifest.ece)
        .bind(&release.manifest.note)
        .bind(format_datetime(release.created_at))
        .bind(release.activated_at.map(format_datetime))
        .bind(release.retired_at.map(format_datetime))
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn list_model_releases(
        &self,
        market_scope: Option<&str>,
    ) -> Result<Vec<ModelReleaseRecord>, StorageError> {
        let rows = if let Some(market_scope) = market_scope {
            sqlx::query(
                r#"
                SELECT *
                FROM analytics_model_releases
                WHERE market_scope = ?1
                ORDER BY created_at DESC, release_id DESC
                "#,
            )
            .bind(market_scope)
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query(
                r#"
                SELECT *
                FROM analytics_model_releases
                ORDER BY created_at DESC, release_id DESC
                "#,
            )
            .fetch_all(&self.pool)
            .await?
        };
        rows.into_iter().map(map_model_release_row).collect()
    }

    pub async fn load_model_release(
        &self,
        release_id: &str,
    ) -> Result<Option<ModelReleaseRecord>, StorageError> {
        let row = sqlx::query(
            r#"
            SELECT *
            FROM analytics_model_releases
            WHERE release_id = ?1
            "#,
        )
        .bind(release_id)
        .fetch_optional(&self.pool)
        .await?;
        row.map(map_model_release_row).transpose()
    }

    pub async fn load_active_model_pointer(
        &self,
        market_scope: &str,
    ) -> Result<Option<ActiveModelPointer>, StorageError> {
        let row = sqlx::query(
            r#"
            SELECT market_scope, release_id, updated_at, updated_by
            FROM analytics_active_model_pointers
            WHERE market_scope = ?1
            "#,
        )
        .bind(market_scope)
        .fetch_optional(&self.pool)
        .await?;
        row.map(map_active_pointer_row).transpose()
    }

    pub async fn load_active_model_release(
        &self,
        market_scope: &str,
    ) -> Result<Option<ModelReleaseRecord>, StorageError> {
        let row = sqlx::query(
            r#"
            SELECT r.*
            FROM analytics_active_model_pointers p
            JOIN analytics_model_releases r ON r.release_id = p.release_id
            WHERE p.market_scope = ?1
            "#,
        )
        .bind(market_scope)
        .fetch_optional(&self.pool)
        .await?;
        row.map(map_model_release_row).transpose()
    }

    pub async fn activate_model_release(
        &self,
        market_scope: &str,
        release_id: &str,
        actor: &str,
    ) -> Result<ModelReleaseRecord, StorageError> {
        let now = Utc::now();
        let mut transaction = self.pool.begin().await?;
        let current_active = sqlx::query(
            r#"
            SELECT release_id
            FROM analytics_active_model_pointers
            WHERE market_scope = ?1
            "#,
        )
        .bind(market_scope)
        .fetch_optional(&mut *transaction)
        .await?;
        if let Some(current_active) = current_active {
            let current_release_id: String = current_active.try_get("release_id")?;
            if current_release_id != release_id {
                sqlx::query(
                    r#"
                    UPDATE analytics_model_releases
                    SET status = 'retired',
                        retired_at = ?2
                    WHERE release_id = ?1
                    "#,
                )
                .bind(current_release_id)
                .bind(format_datetime(now))
                .execute(&mut *transaction)
                .await?;
            }
        }

        let updated = sqlx::query(
            r#"
            UPDATE analytics_model_releases
            SET status = 'active',
                activated_at = ?2,
                retired_at = NULL
            WHERE release_id = ?1
              AND market_scope = ?3
            "#,
        )
        .bind(release_id)
        .bind(format_datetime(now))
        .bind(market_scope)
        .execute(&mut *transaction)
        .await?;
        if updated.rows_affected() == 0 {
            return Err(StorageError::Database(sqlx::Error::RowNotFound));
        }

        sqlx::query(
            r#"
            INSERT INTO analytics_active_model_pointers (
                market_scope, release_id, updated_at, updated_by
            )
            VALUES (?1, ?2, ?3, ?4)
            ON CONFLICT(market_scope) DO UPDATE SET
                release_id = excluded.release_id,
                updated_at = excluded.updated_at,
                updated_by = excluded.updated_by
            "#,
        )
        .bind(market_scope)
        .bind(release_id)
        .bind(format_datetime(now))
        .bind(actor)
        .execute(&mut *transaction)
        .await?;
        transaction.commit().await?;
        self.load_model_release(release_id)
            .await?
            .ok_or(StorageError::Database(sqlx::Error::RowNotFound))
    }

    pub async fn rollback_model_release(
        &self,
        market_scope: &str,
        to_release_id: &str,
        actor: &str,
    ) -> Result<ModelReleaseRecord, StorageError> {
        let now = Utc::now();
        let mut transaction = self.pool.begin().await?;
        let current_active = sqlx::query(
            r#"
            SELECT release_id
            FROM analytics_active_model_pointers
            WHERE market_scope = ?1
            "#,
        )
        .bind(market_scope)
        .fetch_optional(&mut *transaction)
        .await?;
        if let Some(current_active) = current_active {
            let current_release_id: String = current_active.try_get("release_id")?;
            if current_release_id != to_release_id {
                sqlx::query(
                    r#"
                    UPDATE analytics_model_releases
                    SET status = 'rolled_back',
                        retired_at = ?2
                    WHERE release_id = ?1
                    "#,
                )
                .bind(current_release_id)
                .bind(format_datetime(now))
                .execute(&mut *transaction)
                .await?;
            }
        }

        let updated = sqlx::query(
            r#"
            UPDATE analytics_model_releases
            SET status = 'active',
                activated_at = ?2,
                retired_at = NULL
            WHERE release_id = ?1
              AND market_scope = ?3
            "#,
        )
        .bind(to_release_id)
        .bind(format_datetime(now))
        .bind(market_scope)
        .execute(&mut *transaction)
        .await?;
        if updated.rows_affected() == 0 {
            return Err(StorageError::Database(sqlx::Error::RowNotFound));
        }

        sqlx::query(
            r#"
            INSERT INTO analytics_active_model_pointers (
                market_scope, release_id, updated_at, updated_by
            )
            VALUES (?1, ?2, ?3, ?4)
            ON CONFLICT(market_scope) DO UPDATE SET
                release_id = excluded.release_id,
                updated_at = excluded.updated_at,
                updated_by = excluded.updated_by
            "#,
        )
        .bind(market_scope)
        .bind(to_release_id)
        .bind(format_datetime(now))
        .bind(actor)
        .execute(&mut *transaction)
        .await?;
        transaction.commit().await?;
        self.load_model_release(to_release_id)
            .await?
            .ok_or(StorageError::Database(sqlx::Error::RowNotFound))
    }

    pub async fn upsert_prediction_snapshots(
        &self,
        snapshots: &[PredictionSnapshotRecord],
    ) -> Result<(), StorageError> {
        let mut transaction = self.pool.begin().await?;
        for snapshot in snapshots {
            let snapshot_id = prediction_snapshot_id(
                &snapshot.entity_id,
                &snapshot.market_scope,
                snapshot.as_of_date,
                snapshot.release_id.as_deref(),
                &snapshot.point_in_time_mode,
            );
            sqlx::query(
                r#"
                INSERT INTO analytics_prediction_snapshots (
                    snapshot_id,
                    entity_id,
                    market_scope,
                    as_of_date,
                    release_id,
                    probability_mode,
                    release_status,
                    point_in_time_mode,
                    overall_score,
                    external_shock_score,
                    raw_p_5d,
                    raw_p_20d,
                    raw_p_60d,
                    calibrated_p_5d,
                    calibrated_p_20d,
                    calibrated_p_60d,
                    posture,
                    time_to_risk_bucket,
                    feature_set_version,
                    label_version,
                    coverage_score,
                    freshness_status,
                    method_version,
                    recorded_at
                )
                VALUES (
                    ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16,
                    ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24
                )
                ON CONFLICT(snapshot_id) DO UPDATE SET
                    release_id = excluded.release_id,
                    probability_mode = excluded.probability_mode,
                    release_status = excluded.release_status,
                    point_in_time_mode = excluded.point_in_time_mode,
                    overall_score = excluded.overall_score,
                    external_shock_score = excluded.external_shock_score,
                    raw_p_5d = excluded.raw_p_5d,
                    raw_p_20d = excluded.raw_p_20d,
                    raw_p_60d = excluded.raw_p_60d,
                    calibrated_p_5d = excluded.calibrated_p_5d,
                    calibrated_p_20d = excluded.calibrated_p_20d,
                    calibrated_p_60d = excluded.calibrated_p_60d,
                    posture = excluded.posture,
                    time_to_risk_bucket = excluded.time_to_risk_bucket,
                    feature_set_version = excluded.feature_set_version,
                    label_version = excluded.label_version,
                    coverage_score = excluded.coverage_score,
                    freshness_status = excluded.freshness_status,
                    method_version = excluded.method_version,
                    recorded_at = excluded.recorded_at
                "#,
            )
            .bind(snapshot_id)
            .bind(&snapshot.entity_id)
            .bind(&snapshot.market_scope)
            .bind(snapshot.as_of_date.to_string())
            .bind(snapshot.release_id.as_deref())
            .bind(&snapshot.probability_mode)
            .bind(&snapshot.release_status)
            .bind(&snapshot.point_in_time_mode)
            .bind(snapshot.overall_score)
            .bind(snapshot.external_shock_score)
            .bind(snapshot.raw_p_5d)
            .bind(snapshot.raw_p_20d)
            .bind(snapshot.raw_p_60d)
            .bind(snapshot.calibrated_p_5d)
            .bind(snapshot.calibrated_p_20d)
            .bind(snapshot.calibrated_p_60d)
            .bind(&snapshot.posture)
            .bind(&snapshot.time_to_risk_bucket)
            .bind(&snapshot.feature_set_version)
            .bind(&snapshot.label_version)
            .bind(snapshot.coverage_score)
            .bind(&snapshot.freshness_status)
            .bind(&snapshot.method_version)
            .bind(format_datetime(snapshot.recorded_at))
            .execute(&mut *transaction)
            .await?;
        }
        transaction.commit().await?;
        Ok(())
    }

    pub async fn list_prediction_snapshots(
        &self,
        market_scope: Option<&str>,
        release_id: Option<&str>,
        from: Option<NaiveDate>,
        to: Option<NaiveDate>,
        limit: Option<usize>,
    ) -> Result<Vec<PredictionSnapshotRecord>, StorageError> {
        let mut query = String::from(
            r#"
            SELECT
                entity_id,
                market_scope,
                as_of_date,
                release_id,
                probability_mode,
                release_status,
                point_in_time_mode,
                overall_score,
                external_shock_score,
                raw_p_5d,
                raw_p_20d,
                raw_p_60d,
                calibrated_p_5d,
                calibrated_p_20d,
                calibrated_p_60d,
                posture,
                time_to_risk_bucket,
                feature_set_version,
                label_version,
                coverage_score,
                freshness_status,
                method_version,
                recorded_at
            FROM analytics_prediction_snapshots
            WHERE 1 = 1
            "#,
        );
        let mut param_index = 1;
        if market_scope.is_some() {
            query.push_str(&format!(" AND market_scope = ?{param_index}"));
            param_index += 1;
        }
        if release_id.is_some() {
            query.push_str(&format!(" AND release_id = ?{param_index}"));
            param_index += 1;
        }
        if from.is_some() {
            query.push_str(&format!(" AND as_of_date >= ?{param_index}"));
            param_index += 1;
        }
        if to.is_some() {
            query.push_str(&format!(" AND as_of_date <= ?{param_index}"));
            param_index += 1;
        }
        query.push_str(" ORDER BY as_of_date DESC, recorded_at DESC");
        if limit.is_some() {
            query.push_str(&format!(" LIMIT ?{param_index}"));
        }

        let mut statement = sqlx::query(&query);
        if let Some(scope) = market_scope {
            statement = statement.bind(scope);
        }
        if let Some(release_id) = release_id {
            statement = statement.bind(release_id);
        }
        if let Some(start_date) = from {
            statement = statement.bind(start_date.to_string());
        }
        if let Some(end_date) = to {
            statement = statement.bind(end_date.to_string());
        }
        if let Some(limit) = limit {
            statement = statement.bind(limit as i64);
        }

        let rows = statement.fetch_all(&self.pool).await?;
        rows.into_iter().map(map_prediction_snapshot_row).collect()
    }

    pub async fn upsert_feature_snapshots(
        &self,
        snapshots: &[FeatureSnapshotRecord],
    ) -> Result<(), StorageError> {
        let mut transaction = self.pool.begin().await?;
        for snapshot in snapshots {
            let snapshot_id = feature_snapshot_id(
                &snapshot.entity_id,
                &snapshot.market_scope,
                snapshot.as_of_date,
                &snapshot.feature_set_version,
                &snapshot.point_in_time_mode,
            );
            let features_json = serde_json::to_string(&snapshot.features)
                .map_err(|error| StorageError::Database(sqlx::Error::Decode(Box::new(error))))?;
            sqlx::query(
                r#"
                INSERT INTO analytics_feature_snapshots (
                    snapshot_id,
                    entity_id,
                    market_scope,
                    as_of_date,
                    feature_set_version,
                    point_in_time_mode,
                    visibility_status,
                    latest_visible_at,
                    coverage_score,
                    core_feature_coverage,
                    trigger_feature_coverage,
                    external_feature_coverage,
                    feature_count,
                    features_json,
                    created_at
                )
                VALUES (
                    ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15
                )
                ON CONFLICT(snapshot_id) DO UPDATE SET
                    visibility_status = excluded.visibility_status,
                    latest_visible_at = excluded.latest_visible_at,
                    coverage_score = excluded.coverage_score,
                    core_feature_coverage = excluded.core_feature_coverage,
                    trigger_feature_coverage = excluded.trigger_feature_coverage,
                    external_feature_coverage = excluded.external_feature_coverage,
                    feature_count = excluded.feature_count,
                    features_json = excluded.features_json,
                    created_at = excluded.created_at
                "#,
            )
            .bind(snapshot_id)
            .bind(&snapshot.entity_id)
            .bind(&snapshot.market_scope)
            .bind(snapshot.as_of_date.to_string())
            .bind(&snapshot.feature_set_version)
            .bind(&snapshot.point_in_time_mode)
            .bind(&snapshot.visibility_status)
            .bind(snapshot.latest_visible_at.map(format_datetime))
            .bind(snapshot.coverage_score)
            .bind(snapshot.core_feature_coverage)
            .bind(snapshot.trigger_feature_coverage)
            .bind(snapshot.external_feature_coverage)
            .bind(snapshot.feature_count as i64)
            .bind(features_json)
            .bind(format_datetime(snapshot.created_at))
            .execute(&mut *transaction)
            .await?;
        }
        transaction.commit().await?;
        Ok(())
    }

    pub async fn list_feature_snapshots(
        &self,
        market_scope: Option<&str>,
        feature_set_version: Option<&str>,
        from: Option<NaiveDate>,
        to: Option<NaiveDate>,
        limit: Option<usize>,
    ) -> Result<Vec<FeatureSnapshotRecord>, StorageError> {
        let mut query = String::from(
            r#"
            SELECT
                entity_id,
                market_scope,
                as_of_date,
                feature_set_version,
                point_in_time_mode,
                visibility_status,
                latest_visible_at,
                coverage_score,
                core_feature_coverage,
                trigger_feature_coverage,
                external_feature_coverage,
                feature_count,
                features_json,
                created_at
            FROM analytics_feature_snapshots
            WHERE 1 = 1
            "#,
        );
        let mut param_index = 1;
        if market_scope.is_some() {
            query.push_str(&format!(" AND market_scope = ?{param_index}"));
            param_index += 1;
        }
        if feature_set_version.is_some() {
            query.push_str(&format!(" AND feature_set_version = ?{param_index}"));
            param_index += 1;
        }
        if from.is_some() {
            query.push_str(&format!(" AND as_of_date >= ?{param_index}"));
            param_index += 1;
        }
        if to.is_some() {
            query.push_str(&format!(" AND as_of_date <= ?{param_index}"));
            param_index += 1;
        }
        query.push_str(" ORDER BY as_of_date DESC, created_at DESC");
        if limit.is_some() {
            query.push_str(&format!(" LIMIT ?{param_index}"));
        }

        let mut statement = sqlx::query(&query);
        if let Some(scope) = market_scope {
            statement = statement.bind(scope);
        }
        if let Some(version) = feature_set_version {
            statement = statement.bind(version);
        }
        if let Some(start_date) = from {
            statement = statement.bind(start_date.to_string());
        }
        if let Some(end_date) = to {
            statement = statement.bind(end_date.to_string());
        }
        if let Some(limit) = limit {
            statement = statement.bind(limit as i64);
        }

        let rows = statement.fetch_all(&self.pool).await?;
        rows.into_iter().map(map_feature_snapshot_row).collect()
    }

    pub async fn list_feature_snapshots_for_mode(
        &self,
        market_scope: &str,
        feature_set_version: &str,
        point_in_time_mode: &str,
        from: Option<NaiveDate>,
        to: Option<NaiveDate>,
    ) -> Result<Vec<FeatureSnapshotRecord>, StorageError> {
        let mut query = String::from(
            r#"
            SELECT
                entity_id,
                market_scope,
                as_of_date,
                feature_set_version,
                point_in_time_mode,
                visibility_status,
                latest_visible_at,
                coverage_score,
                core_feature_coverage,
                trigger_feature_coverage,
                external_feature_coverage,
                feature_count,
                features_json,
                created_at
            FROM analytics_feature_snapshots
            WHERE market_scope = ?1
              AND feature_set_version = ?2
              AND point_in_time_mode = ?3
            "#,
        );
        let mut param_index = 4;
        if from.is_some() {
            query.push_str(&format!(" AND as_of_date >= ?{param_index}"));
            param_index += 1;
        }
        if to.is_some() {
            query.push_str(&format!(" AND as_of_date <= ?{param_index}"));
        }
        query.push_str(" ORDER BY as_of_date ASC, created_at DESC");

        let mut statement = sqlx::query(&query)
            .bind(market_scope)
            .bind(feature_set_version)
            .bind(point_in_time_mode);
        if let Some(start_date) = from {
            statement = statement.bind(start_date.to_string());
        }
        if let Some(end_date) = to {
            statement = statement.bind(end_date.to_string());
        }

        let rows = statement.fetch_all(&self.pool).await?;
        rows.into_iter().map(map_feature_snapshot_row).collect()
    }

    pub async fn upsert_formal_dataset(
        &self,
        dataset: &FormalDatasetRecord,
    ) -> Result<(), StorageError> {
        let dataset_key = formal_dataset_key(
            &dataset.manifest.dataset_id,
            &dataset.manifest.dataset_version,
        );
        let manifest_json = serde_json::to_string(&dataset.manifest)
            .map_err(|error| StorageError::Database(sqlx::Error::Decode(Box::new(error))))?;
        sqlx::query(
            r#"
            INSERT INTO analytics_formal_datasets (
                dataset_key,
                dataset_id,
                dataset_version,
                market_scope,
                feature_set_version,
                label_version,
                scenario_set_version,
                point_in_time_mode,
                from_date,
                to_date,
                train_end_date,
                calibration_end_date,
                evaluation_start_date,
                row_count,
                note,
                manifest_json,
                created_at
            )
            VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17
            )
            ON CONFLICT(dataset_key) DO UPDATE SET
                feature_set_version = excluded.feature_set_version,
                label_version = excluded.label_version,
                scenario_set_version = excluded.scenario_set_version,
                point_in_time_mode = excluded.point_in_time_mode,
                from_date = excluded.from_date,
                to_date = excluded.to_date,
                train_end_date = excluded.train_end_date,
                calibration_end_date = excluded.calibration_end_date,
                evaluation_start_date = excluded.evaluation_start_date,
                row_count = excluded.row_count,
                note = excluded.note,
                manifest_json = excluded.manifest_json,
                created_at = excluded.created_at
            "#,
        )
        .bind(&dataset_key)
        .bind(&dataset.manifest.dataset_id)
        .bind(&dataset.manifest.dataset_version)
        .bind(&dataset.manifest.market_scope)
        .bind(&dataset.manifest.feature_set_version)
        .bind(&dataset.manifest.label_version)
        .bind(&dataset.manifest.scenario_set_version)
        .bind(&dataset.manifest.point_in_time_mode)
        .bind(dataset.manifest.from_date.map(|value| value.to_string()))
        .bind(dataset.manifest.to_date.map(|value| value.to_string()))
        .bind(
            dataset
                .manifest
                .train_end_date
                .map(|value| value.to_string()),
        )
        .bind(
            dataset
                .manifest
                .calibration_end_date
                .map(|value| value.to_string()),
        )
        .bind(
            dataset
                .manifest
                .evaluation_start_date
                .map(|value| value.to_string()),
        )
        .bind(dataset.manifest.row_count as i64)
        .bind(&dataset.manifest.note)
        .bind(manifest_json)
        .bind(format_datetime(dataset.created_at))
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn load_formal_dataset(
        &self,
        dataset_key: &str,
    ) -> Result<Option<FormalDatasetRecord>, StorageError> {
        let row = sqlx::query(
            r#"
            SELECT
                dataset_id,
                dataset_version,
                market_scope,
                feature_set_version,
                label_version,
                scenario_set_version,
                point_in_time_mode,
                from_date,
                to_date,
                train_end_date,
                calibration_end_date,
                evaluation_start_date,
                row_count,
                note,
                manifest_json,
                created_at
            FROM analytics_formal_datasets
            WHERE dataset_key = ?1
            "#,
        )
        .bind(dataset_key)
        .fetch_optional(&self.pool)
        .await?;

        row.map(map_formal_dataset_row).transpose()
    }

    pub async fn list_formal_datasets(
        &self,
        market_scope: Option<&str>,
        dataset_id: Option<&str>,
        limit: Option<usize>,
    ) -> Result<Vec<FormalDatasetRecord>, StorageError> {
        let mut query = String::from(
            r#"
            SELECT
                dataset_id,
                dataset_version,
                market_scope,
                feature_set_version,
                label_version,
                scenario_set_version,
                point_in_time_mode,
                from_date,
                to_date,
                train_end_date,
                calibration_end_date,
                evaluation_start_date,
                row_count,
                note,
                manifest_json,
                created_at
            FROM analytics_formal_datasets
            WHERE 1 = 1
            "#,
        );
        let mut param_index = 1;
        if market_scope.is_some() {
            query.push_str(&format!(" AND market_scope = ?{param_index}"));
            param_index += 1;
        }
        if dataset_id.is_some() {
            query.push_str(&format!(" AND dataset_id = ?{param_index}"));
            param_index += 1;
        }
        query.push_str(" ORDER BY created_at DESC");
        if limit.is_some() {
            query.push_str(&format!(" LIMIT ?{param_index}"));
        }

        let mut statement = sqlx::query(&query);
        if let Some(scope) = market_scope {
            statement = statement.bind(scope);
        }
        if let Some(dataset_id) = dataset_id {
            statement = statement.bind(dataset_id);
        }
        if let Some(limit) = limit {
            statement = statement.bind(limit as i64);
        }

        let rows = statement.fetch_all(&self.pool).await?;
        rows.into_iter().map(map_formal_dataset_row).collect()
    }

    pub async fn replace_formal_dataset_rows(
        &self,
        dataset_key: &str,
        rows: &[FormalDatasetRowRecord],
    ) -> Result<(), StorageError> {
        let mut transaction = self.pool.begin().await?;
        sqlx::query(
            r#"
            DELETE FROM analytics_formal_dataset_rows
            WHERE dataset_key = ?1
            "#,
        )
        .bind(dataset_key)
        .execute(&mut *transaction)
        .await?;

        for row in rows {
            let row_id = formal_dataset_row_id(dataset_key, row.as_of_date, &row.split_name);
            let features_json = serde_json::to_string(&row.features)
                .map_err(|error| StorageError::Database(sqlx::Error::Decode(Box::new(error))))?;
            sqlx::query(
                r#"
                INSERT INTO analytics_formal_dataset_rows (
                    row_id,
                    dataset_key,
                    split_name,
                    entity_id,
                    market_scope,
                    as_of_date,
                    point_in_time_mode,
                    latest_visible_at,
                    coverage_score,
                    core_feature_coverage,
                    trigger_feature_coverage,
                    external_feature_coverage,
                    sample_quality_grade,
                    primary_scenario_id,
                    scenario_family,
                    label_5d,
                    label_20d,
                    label_60d,
                    features_json,
                    created_at
                )
                VALUES (
                    ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17,
                    ?18, ?19, ?20
                )
                "#,
            )
            .bind(row_id)
            .bind(dataset_key)
            .bind(&row.split_name)
            .bind(&row.entity_id)
            .bind(&row.market_scope)
            .bind(row.as_of_date.to_string())
            .bind(&row.point_in_time_mode)
            .bind(row.latest_visible_at.map(format_datetime))
            .bind(row.coverage_score)
            .bind(row.core_feature_coverage)
            .bind(row.trigger_feature_coverage)
            .bind(row.external_feature_coverage)
            .bind(&row.sample_quality_grade)
            .bind(row.primary_scenario_id.as_deref())
            .bind(row.scenario_family.as_deref())
            .bind(row.label_5d as i64)
            .bind(row.label_20d as i64)
            .bind(row.label_60d as i64)
            .bind(features_json)
            .bind(format_datetime(row.created_at))
            .execute(&mut *transaction)
            .await?;
        }

        transaction.commit().await?;
        Ok(())
    }

    pub async fn list_formal_dataset_rows(
        &self,
        dataset_key: &str,
        split_name: Option<&str>,
        limit: Option<usize>,
    ) -> Result<Vec<FormalDatasetRowRecord>, StorageError> {
        let mut query = String::from(
            r#"
            SELECT
                dataset_key,
                split_name,
                entity_id,
                market_scope,
                as_of_date,
                point_in_time_mode,
                latest_visible_at,
                coverage_score,
                core_feature_coverage,
                trigger_feature_coverage,
                external_feature_coverage,
                sample_quality_grade,
                primary_scenario_id,
                scenario_family,
                label_5d,
                label_20d,
                label_60d,
                features_json,
                created_at
            FROM analytics_formal_dataset_rows
            WHERE dataset_key = ?1
            "#,
        );
        let mut param_index = 2;
        if split_name.is_some() {
            query.push_str(&format!(" AND split_name = ?{param_index}"));
            param_index += 1;
        }
        query.push_str(" ORDER BY as_of_date ASC");
        if limit.is_some() {
            query.push_str(&format!(" LIMIT ?{param_index}"));
        }

        let mut statement = sqlx::query(&query).bind(dataset_key);
        if let Some(split_name) = split_name {
            statement = statement.bind(split_name);
        }
        if let Some(limit) = limit {
            statement = statement.bind(limit as i64);
        }

        let rows = statement.fetch_all(&self.pool).await?;
        rows.into_iter()
            .map(map_formal_dataset_row_record)
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
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
            ON CONFLICT(raw_payload_id) DO UPDATE SET
                response_hash = excluded.response_hash,
                content_length = excluded.content_length,
                raw_file_path = excluded.raw_file_path,
                fetched_at = excluded.fetched_at
            "#,
        )
        .bind(record.raw_payload_id.to_string())
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
            FROM ts_indicator_observations
            WHERE entity_id IN ({placeholders})
              AND as_of_date <= {date_placeholder}
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

    async fn initialize_connection(&self) -> Result<(), StorageError> {
        sqlx::query("PRAGMA journal_mode = WAL")
            .execute(&self.pool)
            .await?;
        sqlx::query("PRAGMA foreign_keys = ON")
            .execute(&self.pool)
            .await?;
        sqlx::query("PRAGMA busy_timeout = 5000")
            .execute(&self.pool)
            .await?;
        sqlx::query("PRAGMA synchronous = NORMAL")
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn upsert_fred_mapping(
        &self,
        indicator_id: &str,
        external_code: &str,
        priority: i64,
    ) -> Result<(), StorageError> {
        self.upsert_external_mapping(
            indicator_id,
            "fred",
            FRED_DATASET_ID,
            external_code,
            priority,
        )
        .await
    }

    async fn upsert_external_mapping(
        &self,
        indicator_id: &str,
        source_id: &str,
        dataset_id: &str,
        external_code: &str,
        priority: i64,
    ) -> Result<(), StorageError> {
        sqlx::query(
            r#"
            INSERT INTO metadata_external_indicator_mappings (
                mapping_id,
                indicator_id,
                source_id,
                dataset_id,
                external_code,
                external_params_json,
                priority
            )
            VALUES (?1, ?2, ?3, ?4, ?5, '{}', ?6)
            ON CONFLICT(indicator_id, source_id, dataset_id, external_code) DO UPDATE SET
                external_params_json = excluded.external_params_json,
                priority = excluded.priority
            "#,
        )
        .bind(Uuid::new_v4().to_string())
        .bind(indicator_id)
        .bind(source_id)
        .bind(dataset_id)
        .bind(external_code)
        .bind(priority)
        .execute(&self.pool)
        .await?;
        Ok(())
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

struct FredIndicatorSeed {
    indicator_id: &'static str,
    display_name: &'static str,
    dimension: RiskDimension,
    description: &'static str,
    unit: &'static str,
    frequency: Frequency,
    risk_direction: RiskDirection,
    external_code: &'static str,
    priority: i64,
}

struct BojIndicatorSeed {
    indicator_id: &'static str,
    display_name: &'static str,
    dimension: RiskDimension,
    description: &'static str,
    unit: &'static str,
    frequency: Frequency,
    risk_direction: RiskDirection,
    dataset_id: &'static str,
    external_code: &'static str,
    default_source_id: &'static str,
    quality_tier: &'static str,
    priority: i64,
}

struct WorldBankIndicatorSeed {
    indicator_id: &'static str,
    display_name: &'static str,
    dimension: RiskDimension,
    description: &'static str,
    unit: &'static str,
    frequency: Frequency,
    risk_direction: RiskDirection,
    external_code: &'static str,
}

struct SecEventIndicatorSeed {
    indicator_id: &'static str,
    display_name: &'static str,
    description: &'static str,
    unit: &'static str,
    risk_direction: RiskDirection,
}

struct GdeltIndicatorSeed {
    indicator_id: &'static str,
    display_name: &'static str,
    description: &'static str,
    unit: &'static str,
    risk_direction: RiskDirection,
}

impl FredIndicatorSeed {
    fn indicator(&self) -> Indicator {
        Indicator {
            indicator_id: self.indicator_id.to_string(),
            display_name: self.display_name.to_string(),
            dimension: self.dimension,
            description: self.description.to_string(),
            unit: self.unit.to_string(),
            frequency: self.frequency,
            risk_direction: self.risk_direction,
            default_source_id: "fred".to_string(),
            quality_tier: "core".to_string(),
        }
    }
}

impl BojIndicatorSeed {
    fn indicator(&self) -> Indicator {
        Indicator {
            indicator_id: self.indicator_id.to_string(),
            display_name: self.display_name.to_string(),
            dimension: self.dimension,
            description: self.description.to_string(),
            unit: self.unit.to_string(),
            frequency: self.frequency,
            risk_direction: self.risk_direction,
            default_source_id: self.default_source_id.to_string(),
            quality_tier: self.quality_tier.to_string(),
        }
    }
}

impl WorldBankIndicatorSeed {
    fn indicator(&self) -> Indicator {
        Indicator {
            indicator_id: self.indicator_id.to_string(),
            display_name: self.display_name.to_string(),
            dimension: self.dimension,
            description: self.description.to_string(),
            unit: self.unit.to_string(),
            frequency: self.frequency,
            risk_direction: self.risk_direction,
            default_source_id: "world_bank".to_string(),
            quality_tier: "core".to_string(),
        }
    }
}

impl SecEventIndicatorSeed {
    fn indicator(&self) -> Indicator {
        Indicator {
            indicator_id: self.indicator_id.to_string(),
            display_name: self.display_name.to_string(),
            dimension: RiskDimension::EventsSentiment,
            description: self.description.to_string(),
            unit: self.unit.to_string(),
            frequency: Frequency::Daily,
            risk_direction: self.risk_direction,
            default_source_id: "sec_edgar".to_string(),
            quality_tier: "supplemental".to_string(),
        }
    }
}

impl GdeltIndicatorSeed {
    fn indicator(&self) -> Indicator {
        Indicator {
            indicator_id: self.indicator_id.to_string(),
            display_name: self.display_name.to_string(),
            dimension: RiskDimension::EventsSentiment,
            description: self.description.to_string(),
            unit: self.unit.to_string(),
            frequency: Frequency::Daily,
            risk_direction: self.risk_direction,
            default_source_id: "gdelt".to_string(),
            quality_tier: "supplemental".to_string(),
        }
    }
}

fn boj_indicator_seeds() -> Vec<BojIndicatorSeed> {
    vec![
        BojIndicatorSeed {
            indicator_id: "us_external_usdjpy_level",
            display_name: "USDJPY 汇率",
            dimension: RiskDimension::ExternalSector,
            description: "BOJ 官方美元兑日元汇率水平，用于识别日元套息交易的潜在平仓压力。",
            unit: "jpy_per_usd",
            frequency: Frequency::Daily,
            risk_direction: RiskDirection::TwoSided,
            dataset_id: BOJ_FX_DATASET_ID,
            external_code: "FXERD01",
            default_source_id: "boj",
            quality_tier: "core",
            priority: 10,
        },
        BojIndicatorSeed {
            indicator_id: "jp_rates_call_rate",
            display_name: "日本无担保隔夜拆借利率",
            dimension: RiskDimension::ExternalSector,
            description: "BOJ 官方无担保隔夜拆借利率，可作为日元融资成本与 BOJ 政策变化代理。",
            unit: "percent",
            frequency: Frequency::Daily,
            risk_direction: RiskDirection::RisingFastIsRiskier,
            dataset_id: BOJ_MONEY_MARKET_DATASET_ID,
            external_code: "STRDCLUCON",
            default_source_id: "boj",
            quality_tier: "extended",
            priority: 20,
        },
    ]
}

fn sec_event_indicator_seeds() -> Vec<SecEventIndicatorSeed> {
    vec![
        SecEventIndicatorSeed {
            indicator_id: "us_event_bank_8k_count",
            display_name: "白名单银行 8-K 数量",
            description: "Daily count of 8-K filings from the SEC EDGAR bank watchlist.",
            unit: "count",
            risk_direction: RiskDirection::ManualRule,
        },
        SecEventIndicatorSeed {
            indicator_id: "us_event_risk_keyword_count",
            display_name: "SEC 风险关键词/规则命中数",
            description:
                "Daily count of SEC filing metadata keyword hits plus high-risk 8-K item rule matches.",
            unit: "count",
            risk_direction: RiskDirection::ManualRule,
        },
        SecEventIndicatorSeed {
            indicator_id: "us_banking_filing_stress_count",
            display_name: "银行 filing 压力计数",
            description:
                "Daily count of filings whose rule-based severity passes the stress threshold.",
            unit: "count",
            risk_direction: RiskDirection::ManualRule,
        },
        SecEventIndicatorSeed {
            indicator_id: "us_event_official_filing_severity",
            display_name: "SEC 官方公告严重度",
            description:
                "Daily severity index aggregated from SEC filing form types, items, and watchlist breadth.",
            unit: "score",
            risk_direction: RiskDirection::ManualRule,
        },
    ]
}

fn gdelt_indicator_seeds() -> Vec<GdeltIndicatorSeed> {
    vec![GdeltIndicatorSeed {
        indicator_id: "global_news_financial_stress_count",
        display_name: "金融压力新闻数量",
        description:
            "Daily GDELT DOC API count for banking, liquidity, funding, and credit-stress coverage.",
        unit: "count",
        risk_direction: RiskDirection::HigherIsRiskier,
    }]
}

fn fred_indicator_seeds() -> Vec<FredIndicatorSeed> {
    vec![
        FredIndicatorSeed {
            indicator_id: "us_external_usdjpy_level",
            display_name: "USDJPY 汇率",
            dimension: RiskDimension::ExternalSector,
            description: "美元兑日元汇率水平，用于识别日元套息交易的潜在平仓压力。",
            unit: "jpy_per_usd",
            frequency: Frequency::Daily,
            risk_direction: RiskDirection::TwoSided,
            external_code: "DEXJPUS",
            priority: 100,
        },
        FredIndicatorSeed {
            indicator_id: "us_market_vix_close",
            display_name: "VIX 收盘价",
            dimension: RiskDimension::MarketStress,
            description: "美国市场隐含波动率。",
            unit: "index",
            frequency: Frequency::Daily,
            risk_direction: RiskDirection::HigherIsRiskier,
            external_code: "VIXCLS",
            priority: 100,
        },
        FredIndicatorSeed {
            indicator_id: "us_credit_high_yield_oas",
            display_name: "高收益债 OAS",
            dimension: RiskDimension::LeverageCredit,
            description: "美国高收益债期权调整利差。",
            unit: "percent",
            frequency: Frequency::Daily,
            risk_direction: RiskDirection::HigherIsRiskier,
            external_code: "BAMLH0A0HYM2",
            priority: 100,
        },
        FredIndicatorSeed {
            indicator_id: "us_credit_baa_10y_spread",
            display_name: "Baa-10Y 信用利差",
            dimension: RiskDimension::LeverageCredit,
            description: "Baa 企业债与 10 年期美国国债利差。",
            unit: "percent",
            frequency: Frequency::Daily,
            risk_direction: RiskDirection::HigherIsRiskier,
            external_code: "BAA10Y",
            priority: 100,
        },
        FredIndicatorSeed {
            indicator_id: "us_rates_yield_curve_10y2y",
            display_name: "10Y-2Y 期限利差",
            dimension: RiskDimension::MarketStress,
            description: "美国 10 年期和 2 年期国债收益率利差。",
            unit: "percent",
            frequency: Frequency::Daily,
            risk_direction: RiskDirection::LowerIsRiskier,
            external_code: "T10Y2Y",
            priority: 100,
        },
        FredIndicatorSeed {
            indicator_id: "us_liquidity_financial_stress_stl",
            display_name: "圣路易斯金融压力指数",
            dimension: RiskDimension::LiquidityFunding,
            description: "St. Louis Fed Financial Stress Index。",
            unit: "index",
            frequency: Frequency::Weekly,
            risk_direction: RiskDirection::HigherIsRiskier,
            external_code: "STLFSI4",
            priority: 100,
        },
        FredIndicatorSeed {
            indicator_id: "us_liquidity_national_financial_conditions",
            display_name: "NFCI 金融条件指数",
            dimension: RiskDimension::LiquidityFunding,
            description: "Chicago Fed National Financial Conditions Index。",
            unit: "index",
            frequency: Frequency::Weekly,
            risk_direction: RiskDirection::HigherIsRiskier,
            external_code: "NFCI",
            priority: 100,
        },
        FredIndicatorSeed {
            indicator_id: "us_macro_unemployment_rate",
            display_name: "失业率",
            dimension: RiskDimension::MacroFragility,
            description: "美国失业率。",
            unit: "percent",
            frequency: Frequency::Monthly,
            risk_direction: RiskDirection::HigherIsRiskier,
            external_code: "UNRATE",
            priority: 100,
        },
        FredIndicatorSeed {
            indicator_id: "us_liquidity_sofr",
            display_name: "SOFR",
            dimension: RiskDimension::LiquidityFunding,
            description: "Secured Overnight Financing Rate。",
            unit: "percent",
            frequency: Frequency::Daily,
            risk_direction: RiskDirection::RisingFastIsRiskier,
            external_code: "SOFR",
            priority: 100,
        },
        FredIndicatorSeed {
            indicator_id: "us_liquidity_effr",
            display_name: "有效联邦基金利率",
            dimension: RiskDimension::LiquidityFunding,
            description:
                "Daily Effective Federal Funds Rate (legacy DFF fallback for pre-EFFR history).",
            unit: "percent",
            frequency: Frequency::Daily,
            risk_direction: RiskDirection::RisingFastIsRiskier,
            external_code: "DFF",
            priority: 80,
        },
        FredIndicatorSeed {
            indicator_id: "us_liquidity_effr",
            display_name: "有效联邦基金利率",
            dimension: RiskDimension::LiquidityFunding,
            description: "Effective Federal Funds Rate。",
            unit: "percent",
            frequency: Frequency::Daily,
            risk_direction: RiskDirection::RisingFastIsRiskier,
            external_code: "EFFR",
            priority: 100,
        },
        FredIndicatorSeed {
            indicator_id: "us_real_estate_housing_starts",
            display_name: "新屋开工",
            dimension: RiskDimension::RealEstate,
            description: "美国新屋开工总数。",
            unit: "thousands",
            frequency: Frequency::Monthly,
            risk_direction: RiskDirection::LowerIsRiskier,
            external_code: "HOUST",
            priority: 100,
        },
        FredIndicatorSeed {
            indicator_id: "us_real_estate_home_price",
            display_name: "Case-Shiller 房价指数",
            dimension: RiskDimension::RealEstate,
            description: "美国全国 Case-Shiller 房价指数。",
            unit: "index",
            frequency: Frequency::Monthly,
            risk_direction: RiskDirection::TwoSided,
            external_code: "CSUSHPISA",
            priority: 100,
        },
        FredIndicatorSeed {
            indicator_id: "us_liquidity_money_supply_m2",
            display_name: "M2 货币供应",
            dimension: RiskDimension::LiquidityFunding,
            description: "美国 M2 货币供应量。",
            unit: "billions",
            frequency: Frequency::Monthly,
            risk_direction: RiskDirection::FallingFastIsRiskier,
            external_code: "M2SL",
            priority: 100,
        },
    ]
}

fn world_bank_indicator_seeds() -> Vec<WorldBankIndicatorSeed> {
    vec![
        WorldBankIndicatorSeed {
            indicator_id: "global_macro_gdp_growth",
            display_name: "GDP 实际增速",
            dimension: RiskDimension::MacroFragility,
            description: "World Bank 年频 GDP 实际增速，当前默认抓取美国。",
            unit: "percent",
            frequency: Frequency::Annual,
            risk_direction: RiskDirection::LowerIsRiskier,
            external_code: "US__NY.GDP.MKTP.KD.ZG",
        },
        WorldBankIndicatorSeed {
            indicator_id: "global_macro_inflation_yoy",
            display_name: "CPI 通胀",
            dimension: RiskDimension::MacroFragility,
            description: "World Bank 年频 CPI 通胀，当前默认抓取美国。",
            unit: "percent",
            frequency: Frequency::Annual,
            risk_direction: RiskDirection::TwoSided,
            external_code: "US__FP.CPI.TOTL.ZG",
        },
        WorldBankIndicatorSeed {
            indicator_id: "global_external_current_account_gdp",
            display_name: "经常账户/GDP",
            dimension: RiskDimension::ExternalSector,
            description: "World Bank 年频经常账户余额占 GDP 比重，当前默认抓取美国。",
            unit: "percent",
            frequency: Frequency::Annual,
            risk_direction: RiskDirection::LowerIsRiskier,
            external_code: "US__BN.CAB.XOKA.GD.ZS",
        },
    ]
}

fn parse_date(value: &str) -> Result<NaiveDate, StorageError> {
    NaiveDate::parse_from_str(value, "%Y-%m-%d")
        .map_err(|error| StorageError::Database(sqlx::Error::Decode(Box::new(error))))
}

fn parse_optional_date(value: Option<String>) -> Result<Option<NaiveDate>, StorageError> {
    value
        .filter(|value| !value.is_empty())
        .map(|value| parse_date(&value))
        .transpose()
}

fn parse_optional_datetime(value: Option<String>) -> Result<Option<DateTime<Utc>>, StorageError> {
    value
        .filter(|value| !value.is_empty())
        .map(|value| {
            DateTime::parse_from_rfc3339(&value)
                .map(|datetime| datetime.with_timezone(&Utc))
                .map_err(|error| StorageError::Database(sqlx::Error::Decode(Box::new(error))))
        })
        .transpose()
}

fn format_datetime(value: DateTime<Utc>) -> String {
    value.to_rfc3339()
}

fn parse_required_datetime(value: &str) -> Result<DateTime<Utc>, StorageError> {
    DateTime::parse_from_rfc3339(value)
        .map(|datetime| datetime.with_timezone(&Utc))
        .map_err(|error| StorageError::Database(sqlx::Error::Decode(Box::new(error))))
}

fn map_model_release_row(row: SqliteRow) -> Result<ModelReleaseRecord, StorageError> {
    let manifest_json: String = row.try_get("manifest_json")?;
    let mut manifest = serde_json::from_str::<ModelReleaseManifest>(&manifest_json)
        .map_err(|error| StorageError::Database(sqlx::Error::Decode(Box::new(error))))?;
    manifest.release_id = row.try_get("release_id")?;
    manifest.market_scope = row.try_get("market_scope")?;
    manifest.status = row.try_get("status")?;
    manifest.probability_mode = row.try_get("probability_mode")?;
    manifest.serving_status = row.try_get("serving_status")?;
    manifest.bundle_uri = row.try_get("bundle_uri")?;
    manifest.feature_set_version = row.try_get("feature_set_version")?;
    manifest.label_version = row.try_get("label_version")?;
    manifest.prob_model_version = row.try_get("prob_model_version")?;
    manifest.calibration_version = row.try_get("calibration_version")?;
    manifest.posture_policy_version = row.try_get("posture_policy_version")?;
    manifest.action_playbook_version = row.try_get("action_playbook_version")?;
    manifest.point_in_time_mode = row.try_get("point_in_time_mode")?;
    manifest.training_range_start =
        parse_optional_date(row.try_get::<Option<String>, _>("training_range_start")?)?;
    manifest.training_range_end =
        parse_optional_date(row.try_get::<Option<String>, _>("training_range_end")?)?;
    manifest.calibration_range_start =
        parse_optional_date(row.try_get::<Option<String>, _>("calibration_range_start")?)?;
    manifest.calibration_range_end =
        parse_optional_date(row.try_get::<Option<String>, _>("calibration_range_end")?)?;
    manifest.evaluation_range_start =
        parse_optional_date(row.try_get::<Option<String>, _>("evaluation_range_start")?)?;
    manifest.evaluation_range_end =
        parse_optional_date(row.try_get::<Option<String>, _>("evaluation_range_end")?)?;
    manifest.brier_score = row.try_get("brier_score")?;
    manifest.log_loss = row.try_get("log_loss")?;
    manifest.ece = row.try_get("ece")?;
    manifest.note = row.try_get("note")?;
    Ok(ModelReleaseRecord {
        manifest,
        created_at: parse_required_datetime(row.try_get::<String, _>("created_at")?.as_str())?,
        activated_at: parse_optional_datetime(row.try_get::<Option<String>, _>("activated_at")?)?,
        retired_at: parse_optional_datetime(row.try_get::<Option<String>, _>("retired_at")?)?,
    })
}

fn map_active_pointer_row(row: SqliteRow) -> Result<ActiveModelPointer, StorageError> {
    Ok(ActiveModelPointer {
        market_scope: row.try_get("market_scope")?,
        release_id: row.try_get("release_id")?,
        updated_at: parse_required_datetime(row.try_get::<String, _>("updated_at")?.as_str())?,
        updated_by: row.try_get("updated_by")?,
    })
}

fn map_prediction_snapshot_row(row: SqliteRow) -> Result<PredictionSnapshotRecord, StorageError> {
    Ok(PredictionSnapshotRecord {
        as_of_date: parse_date(row.try_get::<String, _>("as_of_date")?.as_str())?,
        entity_id: row.try_get("entity_id")?,
        market_scope: row.try_get("market_scope")?,
        release_id: row.try_get("release_id")?,
        probability_mode: row.try_get("probability_mode")?,
        release_status: row.try_get("release_status")?,
        point_in_time_mode: row.try_get("point_in_time_mode")?,
        overall_score: row.try_get("overall_score")?,
        external_shock_score: row.try_get("external_shock_score")?,
        raw_p_5d: row.try_get("raw_p_5d")?,
        raw_p_20d: row.try_get("raw_p_20d")?,
        raw_p_60d: row.try_get("raw_p_60d")?,
        calibrated_p_5d: row.try_get("calibrated_p_5d")?,
        calibrated_p_20d: row.try_get("calibrated_p_20d")?,
        calibrated_p_60d: row.try_get("calibrated_p_60d")?,
        posture: row.try_get("posture")?,
        time_to_risk_bucket: row.try_get("time_to_risk_bucket")?,
        feature_set_version: row.try_get("feature_set_version")?,
        label_version: row.try_get("label_version")?,
        coverage_score: row.try_get("coverage_score")?,
        freshness_status: row.try_get("freshness_status")?,
        method_version: row.try_get("method_version")?,
        recorded_at: parse_required_datetime(row.try_get::<String, _>("recorded_at")?.as_str())?,
    })
}

fn map_feature_snapshot_row(row: SqliteRow) -> Result<FeatureSnapshotRecord, StorageError> {
    let features_json: String = row.try_get("features_json")?;
    let features = serde_json::from_str(&features_json)
        .map_err(|error| StorageError::Database(sqlx::Error::Decode(Box::new(error))))?;
    Ok(FeatureSnapshotRecord {
        as_of_date: parse_date(row.try_get::<String, _>("as_of_date")?.as_str())?,
        entity_id: row.try_get("entity_id")?,
        market_scope: row.try_get("market_scope")?,
        feature_set_version: row.try_get("feature_set_version")?,
        point_in_time_mode: row.try_get("point_in_time_mode")?,
        visibility_status: row.try_get("visibility_status")?,
        latest_visible_at: parse_optional_datetime(
            row.try_get::<Option<String>, _>("latest_visible_at")?,
        )?,
        coverage_score: row.try_get("coverage_score")?,
        core_feature_coverage: row.try_get("core_feature_coverage")?,
        trigger_feature_coverage: row.try_get("trigger_feature_coverage")?,
        external_feature_coverage: row.try_get("external_feature_coverage")?,
        feature_count: row.try_get::<i64, _>("feature_count")? as usize,
        features,
        created_at: parse_required_datetime(row.try_get::<String, _>("created_at")?.as_str())?,
    })
}

fn map_formal_dataset_row(row: SqliteRow) -> Result<FormalDatasetRecord, StorageError> {
    let manifest_json: String = row.try_get("manifest_json")?;
    let mut manifest = serde_json::from_str::<FormalDatasetManifest>(&manifest_json)
        .map_err(|error| StorageError::Database(sqlx::Error::Decode(Box::new(error))))?;
    manifest.dataset_id = row.try_get("dataset_id")?;
    manifest.dataset_version = row.try_get("dataset_version")?;
    manifest.market_scope = row.try_get("market_scope")?;
    manifest.feature_set_version = row.try_get("feature_set_version")?;
    manifest.label_version = row.try_get("label_version")?;
    manifest.scenario_set_version = row.try_get("scenario_set_version")?;
    manifest.point_in_time_mode = row.try_get("point_in_time_mode")?;
    manifest.from_date = parse_optional_date(row.try_get::<Option<String>, _>("from_date")?)?;
    manifest.to_date = parse_optional_date(row.try_get::<Option<String>, _>("to_date")?)?;
    manifest.train_end_date =
        parse_optional_date(row.try_get::<Option<String>, _>("train_end_date")?)?;
    manifest.calibration_end_date =
        parse_optional_date(row.try_get::<Option<String>, _>("calibration_end_date")?)?;
    manifest.evaluation_start_date =
        parse_optional_date(row.try_get::<Option<String>, _>("evaluation_start_date")?)?;
    manifest.row_count = row.try_get::<i64, _>("row_count")? as usize;
    manifest.note = row.try_get("note")?;
    Ok(FormalDatasetRecord {
        manifest,
        created_at: parse_required_datetime(row.try_get::<String, _>("created_at")?.as_str())?,
    })
}

fn map_formal_dataset_row_record(row: SqliteRow) -> Result<FormalDatasetRowRecord, StorageError> {
    let features_json: String = row.try_get("features_json")?;
    let features = serde_json::from_str(&features_json)
        .map_err(|error| StorageError::Database(sqlx::Error::Decode(Box::new(error))))?;
    Ok(FormalDatasetRowRecord {
        dataset_key: row.try_get("dataset_key")?,
        split_name: row.try_get("split_name")?,
        entity_id: row.try_get("entity_id")?,
        market_scope: row.try_get("market_scope")?,
        as_of_date: parse_date(row.try_get::<String, _>("as_of_date")?.as_str())?,
        point_in_time_mode: row.try_get("point_in_time_mode")?,
        latest_visible_at: parse_optional_datetime(
            row.try_get::<Option<String>, _>("latest_visible_at")?,
        )?,
        coverage_score: row.try_get("coverage_score")?,
        core_feature_coverage: row.try_get("core_feature_coverage")?,
        trigger_feature_coverage: row.try_get("trigger_feature_coverage")?,
        external_feature_coverage: row.try_get("external_feature_coverage")?,
        sample_quality_grade: row.try_get("sample_quality_grade")?,
        primary_scenario_id: row.try_get("primary_scenario_id")?,
        scenario_family: row.try_get("scenario_family")?,
        label_5d: row.try_get::<i64, _>("label_5d")? as u8,
        label_20d: row.try_get::<i64, _>("label_20d")? as u8,
        label_60d: row.try_get::<i64, _>("label_60d")? as u8,
        features,
        created_at: parse_required_datetime(row.try_get::<String, _>("created_at")?.as_str())?,
    })
}

fn prediction_snapshot_id(
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

fn feature_snapshot_id(
    entity_id: &str,
    market_scope: &str,
    as_of_date: NaiveDate,
    feature_set_version: &str,
    point_in_time_mode: &str,
) -> String {
    format!("{market_scope}:{entity_id}:{as_of_date}:{feature_set_version}:{point_in_time_mode}")
}

fn formal_dataset_key(dataset_id: &str, dataset_version: &str) -> String {
    format!("{dataset_id}:{dataset_version}")
}

fn formal_dataset_row_id(dataset_key: &str, as_of_date: NaiveDate, split_name: &str) -> String {
    format!("{dataset_key}:{split_name}:{as_of_date}")
}

fn parse_risk_level(value: &str) -> Result<RiskLevel, StorageError> {
    match value {
        "normal" => Ok(RiskLevel::Normal),
        "watch" => Ok(RiskLevel::Watch),
        "stress" => Ok(RiskLevel::Stress),
        "warning" => Ok(RiskLevel::Warning),
        "crisis" => Ok(RiskLevel::Crisis),
        other => Err(StorageError::UnknownRiskLevel(other.to_string())),
    }
}

fn format_risk_level(value: RiskLevel) -> &'static str {
    match value {
        RiskLevel::Normal => "normal",
        RiskLevel::Watch => "watch",
        RiskLevel::Stress => "stress",
        RiskLevel::Warning => "warning",
        RiskLevel::Crisis => "crisis",
    }
}

fn parse_alert_type(value: &str) -> Result<AlertType, StorageError> {
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

fn format_alert_type(value: AlertType) -> &'static str {
    match value {
        AlertType::RiskWatch => "risk_watch",
        AlertType::RiskStress => "risk_stress",
        AlertType::RiskWarning => "risk_warning",
        AlertType::RiskCrisis => "risk_crisis",
        AlertType::DataQualityIssue => "data_quality_issue",
        AlertType::SourceHealthIssue => "source_health_issue",
    }
}

fn parse_alert_status(value: &str) -> Result<AlertStatus, StorageError> {
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

fn format_alert_status(value: AlertStatus) -> &'static str {
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

#[cfg(test)]
mod tests {
    use chrono::{NaiveDate, Utc};
    use fc_domain::{
        AlertEvent, AlertStatus, AlertType, FeatureSnapshotRecord, FormalDatasetManifest,
        FormalDatasetRecord, FormalDatasetRowRecord, ModelReleaseManifest, ModelReleaseRecord,
        PredictionSnapshotRecord, RiskContributor, RiskDimension, RiskLevel,
    };
    use uuid::Uuid;

    use crate::SqliteStore;

    #[tokio::test]
    async fn sqlite_store_round_trips_seeded_observations() {
        let store = SqliteStore::connect_url("sqlite::memory:").await.unwrap();
        store.migrate().await.unwrap();
        store.seed_fred_metadata().await.unwrap();

        let indicators = store.load_indicators().await.unwrap();
        assert!(indicators.len() >= 10);

        let indicator = indicators
            .iter()
            .find(|indicator| indicator.indicator_id == "us_market_vix_close")
            .unwrap()
            .clone();
        let observation = fc_domain::Observation {
            indicator_id: indicator.indicator_id,
            entity_id: "us".to_string(),
            as_of_date: NaiveDate::from_ymd_opt(2020, 3, 16).unwrap(),
            period_start: Some(NaiveDate::from_ymd_opt(2020, 3, 16).unwrap()),
            period_end: Some(NaiveDate::from_ymd_opt(2020, 3, 16).unwrap()),
            frequency: indicator.frequency,
            value: 82.69,
            unit: indicator.unit,
            source_id: "fred".to_string(),
            dataset_id: super::FRED_DATASET_ID.to_string(),
            revision_time: None,
            publication_time: None,
            quality_score: 95.0,
            quality_flags: Vec::new(),
        };
        store.insert_observations(&[observation]).await.unwrap();
        let observations = store
            .load_observations("us", NaiveDate::from_ymd_opt(2020, 3, 17).unwrap())
            .await
            .unwrap();

        assert_eq!(observations.len(), 1);
        assert_eq!(observations[0].value, 82.69);
    }

    #[tokio::test]
    async fn sqlite_store_round_trips_alerts() {
        let store = SqliteStore::connect_url("sqlite::memory:").await.unwrap();
        store.migrate().await.unwrap();

        let alert = AlertEvent {
            alert_id: Uuid::new_v4(),
            event_type: AlertType::RiskStress,
            scope: "sec_edgar_daily".to_string(),
            entity_id: "us".to_string(),
            dimension: Some(RiskDimension::EventsSentiment),
            level: RiskLevel::Stress,
            status: AlertStatus::Open,
            triggered_at: Utc::now(),
            triggered_as_of_date: NaiveDate::from_ymd_opt(2026, 5, 30).unwrap(),
            resolved_at: None,
            score: 61.0,
            previous_score: Some(28.0),
            trigger_reason: "SEC filing stress cluster".to_string(),
            top_contributors: vec![RiskContributor {
                indicator_id: "us_event_official_filing_severity".to_string(),
                display_name: "SEC 官方公告严重度".to_string(),
                dimension: RiskDimension::EventsSentiment,
                score: 61.0,
                contribution: 61.0,
                explanation: "bank filing spike".to_string(),
            }],
            related_indicators: vec![
                "us_event_bank_8k_count".to_string(),
                "us_event_official_filing_severity".to_string(),
            ],
            method_version: "sec_rules_v1".to_string(),
        };

        store
            .replace_alerts_for_scope(
                "sec_edgar_daily",
                NaiveDate::from_ymd_opt(2026, 5, 1).unwrap(),
                NaiveDate::from_ymd_opt(2026, 5, 31).unwrap(),
                &[alert.clone()],
            )
            .await
            .unwrap();

        let alerts = store
            .load_alerts_recent(
                NaiveDate::from_ymd_opt(2026, 5, 1).unwrap(),
                NaiveDate::from_ymd_opt(2026, 5, 31).unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].alert_id, alert.alert_id);
        assert_eq!(alerts[0].related_indicators.len(), 2);
        assert_eq!(
            alerts[0].top_contributors[0].dimension,
            RiskDimension::EventsSentiment
        );
    }

    #[tokio::test]
    async fn sqlite_store_round_trips_model_releases_and_active_pointer() {
        let store = SqliteStore::connect_url("sqlite::memory:").await.unwrap();
        store.migrate().await.unwrap();
        let created_at = Utc::now();
        let release = ModelReleaseRecord {
            manifest: ModelReleaseManifest {
                release_id: "heuristic_bootstrap_20260531".to_string(),
                market_scope: "financial_system".to_string(),
                status: "candidate".to_string(),
                probability_mode: "heuristic_mvp".to_string(),
                serving_status: "degraded".to_string(),
                bundle_uri: "config/model-releases/us-heuristic-bootstrap.json".to_string(),
                feature_set_version: "feature_v2_20260531".to_string(),
                label_version: "label_v1_20260530".to_string(),
                prob_model_version: "prob_v1_20260531".to_string(),
                calibration_version: "calib_v1_20260531".to_string(),
                posture_policy_version: "posture_v1_20260530".to_string(),
                action_playbook_version: "action_playbook_v1_20260531".to_string(),
                point_in_time_mode: "best_effort".to_string(),
                training_range_start: Some(NaiveDate::from_ymd_opt(2007, 1, 1).unwrap()),
                training_range_end: Some(NaiveDate::from_ymd_opt(2021, 12, 31).unwrap()),
                calibration_range_start: Some(NaiveDate::from_ymd_opt(2022, 1, 1).unwrap()),
                calibration_range_end: Some(NaiveDate::from_ymd_opt(2023, 12, 31).unwrap()),
                evaluation_range_start: Some(NaiveDate::from_ymd_opt(2024, 1, 1).unwrap()),
                evaluation_range_end: Some(NaiveDate::from_ymd_opt(2025, 12, 31).unwrap()),
                brier_score: Some(0.082),
                log_loss: Some(0.241),
                ece: Some(0.031),
                note: "bootstrap heuristic release".to_string(),
            },
            created_at,
            activated_at: None,
            retired_at: None,
        };
        store.upsert_model_release(&release).await.unwrap();

        let releases = store
            .list_model_releases(Some("financial_system"))
            .await
            .unwrap();
        assert_eq!(releases.len(), 1);
        assert_eq!(releases[0].manifest.release_id, release.manifest.release_id);

        let active = store
            .activate_model_release("financial_system", &release.manifest.release_id, "test")
            .await
            .unwrap();
        assert_eq!(active.manifest.status, "active");

        let active_pointer = store
            .load_active_model_pointer("financial_system")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(active_pointer.release_id, release.manifest.release_id);

        let loaded = store
            .load_active_model_release("financial_system")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(loaded.manifest.bundle_uri, release.manifest.bundle_uri);
    }

    #[tokio::test]
    async fn sqlite_store_round_trips_prediction_snapshots() {
        let store = SqliteStore::connect_url("sqlite::memory:").await.unwrap();
        store.migrate().await.unwrap();
        let recorded_at = Utc::now();
        let snapshot = PredictionSnapshotRecord {
            as_of_date: NaiveDate::from_ymd_opt(2026, 5, 31).unwrap(),
            entity_id: "us".to_string(),
            market_scope: "financial_system".to_string(),
            release_id: Some("us_heuristic_bootstrap_20260531".to_string()),
            probability_mode: "heuristic_mvp".to_string(),
            release_status: "degraded".to_string(),
            point_in_time_mode: "best_effort".to_string(),
            overall_score: 67.4,
            external_shock_score: 58.1,
            raw_p_5d: 0.18,
            raw_p_20d: 0.34,
            raw_p_60d: 0.41,
            calibrated_p_5d: 0.18,
            calibrated_p_20d: 0.34,
            calibrated_p_60d: 0.41,
            posture: "prepare".to_string(),
            time_to_risk_bucket: "weeks".to_string(),
            feature_set_version: "feature_v2_20260531".to_string(),
            label_version: "label_v1_20260530".to_string(),
            coverage_score: 0.87,
            freshness_status: "fresh".to_string(),
            method_version: "scoring_v2_20260531".to_string(),
            recorded_at,
        };

        store
            .upsert_prediction_snapshots(std::slice::from_ref(&snapshot))
            .await
            .unwrap();

        let rows = store
            .list_prediction_snapshots(
                Some("financial_system"),
                Some("us_heuristic_bootstrap_20260531"),
                Some(NaiveDate::from_ymd_opt(2026, 5, 1).unwrap()),
                Some(NaiveDate::from_ymd_opt(2026, 5, 31).unwrap()),
                Some(10),
            )
            .await
            .unwrap();

        assert_eq!(rows.len(), 1);
        assert_eq!(
            rows[0].release_id.as_deref(),
            Some("us_heuristic_bootstrap_20260531")
        );
        assert_eq!(rows[0].time_to_risk_bucket, "weeks");
        assert_eq!(rows[0].freshness_status, "fresh");
    }

    #[tokio::test]
    async fn sqlite_store_round_trips_feature_snapshots_and_formal_datasets() {
        let store = SqliteStore::connect_url("sqlite::memory:").await.unwrap();
        store.migrate().await.unwrap();
        let created_at = Utc::now();

        let snapshot = FeatureSnapshotRecord {
            as_of_date: NaiveDate::from_ymd_opt(2026, 5, 30).unwrap(),
            entity_id: "us".to_string(),
            market_scope: "financial_system".to_string(),
            feature_set_version: "feature_formal_v1".to_string(),
            point_in_time_mode: "best_effort".to_string(),
            visibility_status: "best_effort".to_string(),
            latest_visible_at: Some(created_at),
            coverage_score: 0.91,
            core_feature_coverage: 0.94,
            trigger_feature_coverage: 0.88,
            external_feature_coverage: 0.81,
            feature_count: 4,
            features: [
                ("us_vix_level".to_string(), 22.4),
                ("us_curve_10y2y_level".to_string(), -0.42),
                ("structural_score".to_string(), 0.61),
                ("trigger_score".to_string(), 0.64),
            ]
            .into_iter()
            .collect(),
            created_at,
        };

        store
            .upsert_feature_snapshots(std::slice::from_ref(&snapshot))
            .await
            .unwrap();

        let snapshots = store
            .list_feature_snapshots(
                Some("financial_system"),
                Some("feature_formal_v1"),
                Some(NaiveDate::from_ymd_opt(2026, 5, 1).unwrap()),
                Some(NaiveDate::from_ymd_opt(2026, 5, 31).unwrap()),
                Some(10),
            )
            .await
            .unwrap();
        assert_eq!(snapshots.len(), 1);
        assert_eq!(snapshots[0].feature_count, 4);
        assert!(snapshots[0].features.contains_key("us_vix_level"));

        let exact_snapshots = store
            .list_feature_snapshots_for_mode(
                "financial_system",
                "feature_formal_v1",
                "best_effort",
                Some(NaiveDate::from_ymd_opt(2026, 5, 1).unwrap()),
                Some(NaiveDate::from_ymd_opt(2026, 5, 31).unwrap()),
            )
            .await
            .unwrap();
        assert_eq!(exact_snapshots.len(), 1);
        assert_eq!(exact_snapshots[0].point_in_time_mode, "best_effort");

        let dataset = FormalDatasetRecord {
            manifest: FormalDatasetManifest {
                dataset_id: "formal_v1_main_1990_daily".to_string(),
                dataset_version: "20260531T120000".to_string(),
                market_scope: "financial_system".to_string(),
                feature_set_version: "feature_formal_v1".to_string(),
                label_version: "formal_label_v1_main".to_string(),
                scenario_set_version: "scenario_v1".to_string(),
                point_in_time_mode: "best_effort".to_string(),
                from_date: Some(NaiveDate::from_ymd_opt(1990, 1, 2).unwrap()),
                to_date: Some(NaiveDate::from_ymd_opt(2026, 5, 30).unwrap()),
                train_end_date: Some(NaiveDate::from_ymd_opt(2014, 12, 31).unwrap()),
                calibration_end_date: Some(NaiveDate::from_ymd_opt(2019, 12, 31).unwrap()),
                evaluation_start_date: Some(NaiveDate::from_ymd_opt(2020, 1, 1).unwrap()),
                row_count: 1,
                note: "unit test dataset".to_string(),
            },
            created_at,
        };
        store.upsert_formal_dataset(&dataset).await.unwrap();
        let dataset_key = super::formal_dataset_key(
            &dataset.manifest.dataset_id,
            &dataset.manifest.dataset_version,
        );
        let row = FormalDatasetRowRecord {
            dataset_key: dataset_key.clone(),
            split_name: "evaluation".to_string(),
            entity_id: "us".to_string(),
            market_scope: "financial_system".to_string(),
            as_of_date: NaiveDate::from_ymd_opt(2026, 5, 30).unwrap(),
            point_in_time_mode: "best_effort".to_string(),
            latest_visible_at: Some(created_at),
            coverage_score: 0.91,
            core_feature_coverage: 0.94,
            trigger_feature_coverage: 0.88,
            external_feature_coverage: 0.81,
            sample_quality_grade: "a".to_string(),
            primary_scenario_id: None,
            scenario_family: None,
            label_5d: 0,
            label_20d: 0,
            label_60d: 0,
            features: snapshot.features.clone(),
            created_at,
        };
        store
            .replace_formal_dataset_rows(&dataset_key, &[row.clone()])
            .await
            .unwrap();

        let loaded_dataset = store
            .load_formal_dataset(&dataset_key)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(loaded_dataset.manifest.row_count, 1);
        assert_eq!(
            loaded_dataset.manifest.dataset_id,
            "formal_v1_main_1990_daily"
        );

        let rows = store
            .list_formal_dataset_rows(&dataset_key, Some("evaluation"), Some(10))
            .await
            .unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].split_name, "evaluation");
        assert_eq!(rows[0].dataset_key, dataset_key);
        assert_eq!(rows[0].features["us_vix_level"], 22.4);
    }
}
