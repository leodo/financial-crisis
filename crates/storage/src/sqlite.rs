use std::{collections::HashSet, fs, path::Path, str::FromStr};

use chrono::{DateTime, NaiveDate, Utc};
use fc_domain::{
    ActiveModelPointer, AlertStatus, AlertType, FeatureSnapshotRecord, FormalDatasetManifest,
    FormalDatasetRecord, FormalDatasetRowRecord, Frequency, HistoricalAssessmentPointRecord,
    HistoricalReplayRunRecord, Indicator, ModelReleaseManifest, ModelReleaseRecord, Observation,
    PredictionSnapshotRecord, RiskDimension, RiskDirection, RiskLevel,
};
use sqlx::{
    sqlite::{SqliteConnectOptions, SqlitePoolOptions, SqliteRow},
    Row, SqlitePool,
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
mod historical_replay;
mod metadata;
mod observations;
mod operational;
mod prediction_snapshots;
mod releases;

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
        self.ensure_prediction_snapshot_clause_columns().await?;
        self.ensure_formal_dataset_regime_columns().await?;
        self.ensure_formal_dataset_action_label_columns().await?;
        self.ensure_formal_dataset_action_episode_columns().await?;
        Ok(())
    }

    async fn ensure_prediction_snapshot_clause_columns(&self) -> Result<(), StorageError> {
        let columns = sqlx::query("PRAGMA table_info(analytics_prediction_snapshots)")
            .fetch_all(&self.pool)
            .await?;
        let column_names = columns
            .into_iter()
            .map(|row| row.try_get::<String, _>("name"))
            .collect::<Result<HashSet<_>, _>>()?;

        for (column_name, alter_sql) in [
            (
                "posture_trigger_codes_json",
                "ALTER TABLE analytics_prediction_snapshots ADD COLUMN posture_trigger_codes_json TEXT NOT NULL DEFAULT '[]'",
            ),
            (
                "posture_blocker_codes_json",
                "ALTER TABLE analytics_prediction_snapshots ADD COLUMN posture_blocker_codes_json TEXT NOT NULL DEFAULT '[]'",
            ),
        ] {
            if !column_names.contains(column_name) {
                sqlx::query(alter_sql).execute(&self.pool).await?;
            }
        }

        Ok(())
    }

    async fn ensure_formal_dataset_regime_columns(&self) -> Result<(), StorageError> {
        let columns = sqlx::query("PRAGMA table_info(analytics_formal_dataset_rows)")
            .fetch_all(&self.pool)
            .await?;
        let column_names = columns
            .into_iter()
            .map(|row| row.try_get::<String, _>("name"))
            .collect::<Result<HashSet<_>, _>>()?;

        for (column_name, alter_sql) in [
            (
                "regime_5d",
                "ALTER TABLE analytics_formal_dataset_rows ADD COLUMN regime_5d TEXT NOT NULL DEFAULT 'normal'",
            ),
            (
                "regime_20d",
                "ALTER TABLE analytics_formal_dataset_rows ADD COLUMN regime_20d TEXT NOT NULL DEFAULT 'normal'",
            ),
            (
                "regime_60d",
                "ALTER TABLE analytics_formal_dataset_rows ADD COLUMN regime_60d TEXT NOT NULL DEFAULT 'normal'",
            ),
        ] {
            if !column_names.contains(column_name) {
                sqlx::query(alter_sql).execute(&self.pool).await?;
            }
        }

        Ok(())
    }

    async fn ensure_formal_dataset_action_label_columns(&self) -> Result<(), StorageError> {
        let columns = sqlx::query("PRAGMA table_info(analytics_formal_dataset_rows)")
            .fetch_all(&self.pool)
            .await?;
        let column_names = columns
            .into_iter()
            .map(|row| row.try_get::<String, _>("name"))
            .collect::<Result<HashSet<_>, _>>()?;

        for (column_name, alter_sql) in [
            (
                "action_label_5d",
                "ALTER TABLE analytics_formal_dataset_rows ADD COLUMN action_label_5d INTEGER NOT NULL DEFAULT 0",
            ),
            (
                "action_label_20d",
                "ALTER TABLE analytics_formal_dataset_rows ADD COLUMN action_label_20d INTEGER NOT NULL DEFAULT 0",
            ),
            (
                "action_label_60d",
                "ALTER TABLE analytics_formal_dataset_rows ADD COLUMN action_label_60d INTEGER NOT NULL DEFAULT 0",
            ),
        ] {
            if !column_names.contains(column_name) {
                sqlx::query(alter_sql).execute(&self.pool).await?;
            }
        }

        Ok(())
    }

    async fn ensure_formal_dataset_action_episode_columns(&self) -> Result<(), StorageError> {
        let columns = sqlx::query("PRAGMA table_info(analytics_formal_dataset_rows)")
            .fetch_all(&self.pool)
            .await?;
        let column_names = columns
            .into_iter()
            .map(|row| row.try_get::<String, _>("name"))
            .collect::<Result<HashSet<_>, _>>()?;

        for (column_name, alter_sql) in [
            (
                "scenario_training_role",
                "ALTER TABLE analytics_formal_dataset_rows ADD COLUMN scenario_training_role TEXT",
            ),
            (
                "prepare_episode_label",
                "ALTER TABLE analytics_formal_dataset_rows ADD COLUMN prepare_episode_label INTEGER NOT NULL DEFAULT 0",
            ),
            (
                "hedge_episode_label",
                "ALTER TABLE analytics_formal_dataset_rows ADD COLUMN hedge_episode_label INTEGER NOT NULL DEFAULT 0",
            ),
            (
                "defend_episode_label",
                "ALTER TABLE analytics_formal_dataset_rows ADD COLUMN defend_episode_label INTEGER NOT NULL DEFAULT 0",
            ),
            (
                "primary_action_level",
                "ALTER TABLE analytics_formal_dataset_rows ADD COLUMN primary_action_level TEXT",
            ),
            (
                "action_episode_id",
                "ALTER TABLE analytics_formal_dataset_rows ADD COLUMN action_episode_id TEXT",
            ),
            (
                "action_episode_phase",
                "ALTER TABLE analytics_formal_dataset_rows ADD COLUMN action_episode_phase TEXT NOT NULL DEFAULT 'outside'",
            ),
            (
                "protected_action_window",
                "ALTER TABLE analytics_formal_dataset_rows ADD COLUMN protected_action_window INTEGER NOT NULL DEFAULT 0",
            ),
        ] {
            if !column_names.contains(column_name) {
                sqlx::query(alter_sql).execute(&self.pool).await?;
            }
        }

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
    let posture_trigger_codes = serde_json::from_str::<Vec<String>>(
        row.try_get::<String, _>("posture_trigger_codes_json")?
            .as_str(),
    )
    .map_err(|error| StorageError::Database(sqlx::Error::Decode(Box::new(error))))?;
    let posture_blocker_codes = serde_json::from_str::<Vec<String>>(
        row.try_get::<String, _>("posture_blocker_codes_json")?
            .as_str(),
    )
    .map_err(|error| StorageError::Database(sqlx::Error::Decode(Box::new(error))))?;
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
        posture_trigger_codes,
        posture_blocker_codes,
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
        scenario_training_role: row.try_get("scenario_training_role")?,
        label_5d: row.try_get::<i64, _>("label_5d")? as u8,
        label_20d: row.try_get::<i64, _>("label_20d")? as u8,
        label_60d: row.try_get::<i64, _>("label_60d")? as u8,
        regime_5d: row.try_get("regime_5d")?,
        regime_20d: row.try_get("regime_20d")?,
        regime_60d: row.try_get("regime_60d")?,
        action_label_5d: row.try_get::<i64, _>("action_label_5d")? as u8,
        action_label_20d: row.try_get::<i64, _>("action_label_20d")? as u8,
        action_label_60d: row.try_get::<i64, _>("action_label_60d")? as u8,
        prepare_episode_label: row.try_get::<i64, _>("prepare_episode_label")? as u8,
        hedge_episode_label: row.try_get::<i64, _>("hedge_episode_label")? as u8,
        defend_episode_label: row.try_get::<i64, _>("defend_episode_label")? as u8,
        primary_action_level: row.try_get("primary_action_level")?,
        action_episode_id: row.try_get("action_episode_id")?,
        action_episode_phase: row.try_get("action_episode_phase")?,
        protected_action_window: row.try_get::<i64, _>("protected_action_window")? != 0,
        features,
        created_at: parse_required_datetime(row.try_get::<String, _>("created_at")?.as_str())?,
    })
}

fn map_historical_replay_run_row(
    row: SqliteRow,
) -> Result<HistoricalReplayRunRecord, StorageError> {
    Ok(HistoricalReplayRunRecord {
        replay_run_id: row.try_get("replay_run_id")?,
        release_id: row.try_get("release_id")?,
        market_scope: row.try_get("market_scope")?,
        from_date: parse_date(row.try_get::<String, _>("from_date")?.as_str())?,
        to_date: parse_date(row.try_get::<String, _>("to_date")?.as_str())?,
        history_cache_key: row.try_get("history_cache_key")?,
        feature_set_version: row.try_get("feature_set_version")?,
        label_version: row.try_get("label_version")?,
        point_in_time_mode: row.try_get("point_in_time_mode")?,
        runtime_policy_version: row.try_get("runtime_policy_version")?,
        action_playbook_version: row.try_get("action_playbook_version")?,
        protected_window_catalog_id: row.try_get("protected_window_catalog_id")?,
        source_watermark: row.try_get("source_watermark")?,
        status: row.try_get("status")?,
        point_count: row.try_get::<i64, _>("point_count")? as usize,
        failure_reason: row.try_get("failure_reason")?,
        created_at: parse_required_datetime(row.try_get::<String, _>("created_at")?.as_str())?,
    })
}

fn map_historical_assessment_point_row(
    row: SqliteRow,
) -> Result<HistoricalAssessmentPointRecord, StorageError> {
    let posture_trigger_codes = serde_json::from_str::<Vec<String>>(
        row.try_get::<String, _>("posture_trigger_codes_json")?
            .as_str(),
    )
    .map_err(|error| StorageError::Database(sqlx::Error::Decode(Box::new(error))))?;
    let posture_blocker_codes = serde_json::from_str::<Vec<String>>(
        row.try_get::<String, _>("posture_blocker_codes_json")?
            .as_str(),
    )
    .map_err(|error| StorageError::Database(sqlx::Error::Decode(Box::new(error))))?;
    Ok(HistoricalAssessmentPointRecord {
        replay_run_id: row.try_get("replay_run_id")?,
        entity_id: row.try_get("entity_id")?,
        market_scope: row.try_get("market_scope")?,
        release_id: row.try_get("release_id")?,
        as_of_date: parse_date(row.try_get::<String, _>("as_of_date")?.as_str())?,
        feature_snapshot_id: row.try_get("feature_snapshot_id")?,
        point_in_time_mode: row.try_get("point_in_time_mode")?,
        runtime_policy_version: row.try_get("runtime_policy_version")?,
        action_playbook_version: row.try_get("action_playbook_version")?,
        overall_score: row.try_get("overall_score")?,
        structural_score: row.try_get("structural_score")?,
        trigger_score: row.try_get("trigger_score")?,
        external_shock_score: row.try_get("external_shock_score")?,
        raw_p_5d: row.try_get("raw_p_5d")?,
        raw_p_20d: row.try_get("raw_p_20d")?,
        raw_p_60d: row.try_get("raw_p_60d")?,
        calibrated_p_5d: row.try_get("calibrated_p_5d")?,
        calibrated_p_20d: row.try_get("calibrated_p_20d")?,
        calibrated_p_60d: row.try_get("calibrated_p_60d")?,
        posture: row.try_get("posture")?,
        time_to_risk_bucket: row.try_get("time_to_risk_bucket")?,
        actionability_prepare: row.try_get("actionability_prepare")?,
        actionability_hedge: row.try_get("actionability_hedge")?,
        actionability_defend: row.try_get("actionability_defend")?,
        posture_trigger_codes,
        posture_blocker_codes,
        coverage_score: row.try_get("coverage_score")?,
        freshness_status: row.try_get("freshness_status")?,
        generated_at: parse_required_datetime(row.try_get::<String, _>("generated_at")?.as_str())?,
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

fn historical_assessment_point_id(
    replay_run_id: &str,
    entity_id: &str,
    as_of_date: NaiveDate,
) -> String {
    format!("{replay_run_id}:{entity_id}:{as_of_date}")
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
        FormalDatasetRecord, FormalDatasetRowRecord, HistoricalAssessmentPointRecord,
        HistoricalReplayRunRecord, ModelReleaseManifest, ModelReleaseRecord,
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
            posture_trigger_codes: vec!["prepare_p60d_structural".to_string()],
            posture_blocker_codes: vec!["quality_blocked_hedge".to_string()],
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
        assert_eq!(
            rows[0].posture_trigger_codes,
            vec!["prepare_p60d_structural".to_string()]
        );
        assert_eq!(
            rows[0].posture_blocker_codes,
            vec!["quality_blocked_hedge".to_string()]
        );
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
            scenario_training_role: None,
            label_5d: 0,
            label_20d: 0,
            label_60d: 0,
            regime_5d: "normal".to_string(),
            regime_20d: "normal".to_string(),
            regime_60d: "normal".to_string(),
            action_label_5d: 0,
            action_label_20d: 0,
            action_label_60d: 0,
            prepare_episode_label: 0,
            hedge_episode_label: 0,
            defend_episode_label: 0,
            primary_action_level: None,
            action_episode_id: None,
            action_episode_phase: "outside".to_string(),
            protected_action_window: false,
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
        assert_eq!(rows[0].regime_60d, "normal");
        assert_eq!(rows[0].features["us_vix_level"], 22.4);
    }

    #[tokio::test]
    async fn sqlite_store_round_trips_historical_replay_runs_and_points() {
        let store = SqliteStore::connect_url("sqlite::memory:").await.unwrap();
        store.migrate().await.unwrap();
        let created_at = Utc::now();
        let release = ModelReleaseRecord {
            manifest: ModelReleaseManifest {
                release_id: "release-1".to_string(),
                market_scope: "financial_system".to_string(),
                status: "candidate".to_string(),
                probability_mode: "formal_bundle_v1".to_string(),
                serving_status: "shadow".to_string(),
                bundle_uri: "file:///tmp/release.json".to_string(),
                feature_set_version: "feature_formal_v1".to_string(),
                label_version: "formal_label_v1_main".to_string(),
                prob_model_version: "prob_v1".to_string(),
                calibration_version: "calib_v1".to_string(),
                posture_policy_version: "posture_v1".to_string(),
                action_playbook_version: "action_playbook_v1".to_string(),
                point_in_time_mode: "best_effort".to_string(),
                training_range_start: None,
                training_range_end: None,
                calibration_range_start: None,
                calibration_range_end: None,
                evaluation_range_start: None,
                evaluation_range_end: None,
                brier_score: None,
                log_loss: None,
                ece: None,
                note: String::new(),
            },
            created_at,
            activated_at: None,
            retired_at: None,
        };
        store.upsert_model_release(&release).await.unwrap();

        let run = HistoricalReplayRunRecord {
            replay_run_id: "replay-1".to_string(),
            release_id: Some("release-1".to_string()),
            market_scope: "financial_system".to_string(),
            from_date: NaiveDate::from_ymd_opt(2026, 5, 1).unwrap(),
            to_date: NaiveDate::from_ymd_opt(2026, 5, 30).unwrap(),
            history_cache_key: "history_cache_v3|release=release-1".to_string(),
            feature_set_version: "feature_formal_v1".to_string(),
            label_version: "formal_label_v1_main".to_string(),
            point_in_time_mode: "best_effort".to_string(),
            runtime_policy_version: "runtime_history_v1".to_string(),
            action_playbook_version: "action_playbook_v1".to_string(),
            protected_window_catalog_id: "scenario_v1_main".to_string(),
            source_watermark: "observations=2026-05-30".to_string(),
            status: "success".to_string(),
            point_count: 1,
            failure_reason: None,
            created_at,
        };
        store.upsert_historical_replay_run(&run).await.unwrap();

        let point = HistoricalAssessmentPointRecord {
            replay_run_id: run.replay_run_id.clone(),
            entity_id: "us".to_string(),
            market_scope: "financial_system".to_string(),
            release_id: Some("release-1".to_string()),
            as_of_date: NaiveDate::from_ymd_opt(2026, 5, 30).unwrap(),
            feature_snapshot_id: Some(
                "financial_system:us:2026-05-30:feature_formal_v1:best_effort".to_string(),
            ),
            point_in_time_mode: "best_effort".to_string(),
            runtime_policy_version: "runtime_history_v1".to_string(),
            action_playbook_version: "action_playbook_v1".to_string(),
            overall_score: 72.4,
            structural_score: 68.1,
            trigger_score: 64.2,
            external_shock_score: 55.8,
            raw_p_5d: 0.08,
            raw_p_20d: 0.19,
            raw_p_60d: 0.27,
            calibrated_p_5d: 0.06,
            calibrated_p_20d: 0.17,
            calibrated_p_60d: 0.24,
            posture: "prepare".to_string(),
            time_to_risk_bucket: "months".to_string(),
            actionability_prepare: 0.61,
            actionability_hedge: 0.28,
            actionability_defend: 0.09,
            posture_trigger_codes: vec!["prepare_p60d_structural".to_string()],
            posture_blocker_codes: vec!["quality_blocked_hedge".to_string()],
            coverage_score: 0.92,
            freshness_status: "fresh".to_string(),
            generated_at: created_at,
        };
        store
            .replace_historical_assessment_points(&run.replay_run_id, &[point.clone()])
            .await
            .unwrap();

        let loaded_run = store
            .load_latest_historical_replay_run(
                "financial_system",
                Some("release-1"),
                "history_cache_v3|release=release-1",
                NaiveDate::from_ymd_opt(2026, 5, 1).unwrap(),
                NaiveDate::from_ymd_opt(2026, 5, 30).unwrap(),
            )
            .await
            .unwrap()
            .unwrap();
        assert_eq!(loaded_run.replay_run_id, "replay-1");
        assert_eq!(loaded_run.point_count, 1);

        let runs = store
            .list_historical_replay_runs(
                Some("financial_system"),
                Some("release-1"),
                Some(NaiveDate::from_ymd_opt(2026, 5, 1).unwrap()),
                Some(NaiveDate::from_ymd_opt(2026, 5, 31).unwrap()),
                Some(10),
            )
            .await
            .unwrap();
        assert_eq!(runs.len(), 1);
        assert_eq!(
            runs[0].history_cache_key,
            "history_cache_v3|release=release-1"
        );

        let points = store
            .list_historical_assessment_points(
                Some("replay-1"),
                Some("financial_system"),
                Some("release-1"),
                Some(NaiveDate::from_ymd_opt(2026, 5, 1).unwrap()),
                Some(NaiveDate::from_ymd_opt(2026, 5, 31).unwrap()),
                Some(10),
            )
            .await
            .unwrap();
        assert_eq!(points.len(), 1);
        assert_eq!(points[0].posture, "prepare");
        assert_eq!(points[0].actionability_prepare, 0.61);
        assert_eq!(
            points[0].posture_trigger_codes,
            vec!["prepare_p60d_structural".to_string()]
        );
    }
}
