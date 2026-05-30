use std::{fs, path::Path, str::FromStr};

use chrono::{DateTime, NaiveDate, Utc};
use fc_domain::{Frequency, Indicator, Observation, RiskDimension, RiskDirection};
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
pub const TREASURY_YIELD_DATASET_ID: &str = "treasury_daily_yield_curve";

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

        for seed in fred_indicator_seeds() {
            let indicator = seed.indicator();
            self.upsert_indicator(&indicator).await?;
            self.upsert_fred_mapping(&indicator.indicator_id, seed.external_code)
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
        let rows = sqlx::query(
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
            WHERE entity_id = ?1
              AND as_of_date <= ?2
            ORDER BY indicator_id, as_of_date
            "#,
        )
        .bind(entity_id)
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
                    revision_time: parse_optional_datetime(
                        row.try_get::<Option<String>, _>("revision_time")?,
                    )?,
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

fn fred_indicator_seeds() -> Vec<FredIndicatorSeed> {
    vec![
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

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;

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
}
