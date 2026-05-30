use std::{fs, path::Path, str::FromStr};

use chrono::{DateTime, NaiveDate, Utc};
use fc_domain::{
    AlertEvent, AlertStatus, AlertType, Frequency, Indicator, Observation, RiskDimension,
    RiskDirection, RiskLevel,
};
use sqlx::{
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
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
            self.upsert_fred_mapping(&indicator.indicator_id, seed.external_code)
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
    ) -> Result<(), StorageError> {
        self.upsert_external_mapping(indicator_id, "fred", FRED_DATASET_ID, external_code, 100)
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
        AlertEvent, AlertStatus, AlertType, RiskContributor, RiskDimension, RiskLevel,
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
}
