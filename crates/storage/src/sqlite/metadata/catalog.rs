use crate::StorageError;

use super::super::{
    boj_indicator_seeds, fred_indicator_seeds, gdelt_indicator_seeds, sec_event_indicator_seeds,
    world_bank_indicator_seeds, SqliteStore, BOJ_FX_DATASET_ID, BOJ_MONEY_MARKET_DATASET_ID,
    FRED_DATASET_ID, GDELT_DOC_DATASET_ID, SEC_EVENTS_DATASET_ID, SEC_SUBMISSIONS_DATASET_ID,
    TREASURY_YIELD_DATASET_ID, WORLD_BANK_DATASET_ID,
};

#[derive(Debug, Clone, Copy)]
struct MetadataSourceSeed {
    source_id: &'static str,
    display_name: &'static str,
    source_type: &'static str,
    official_url: &'static str,
    documentation_url: &'static str,
    access_method: &'static str,
    auth_required: bool,
    auth_secret_ref: Option<&'static str>,
    rate_limit_policy_json: &'static str,
    license_note: &'static str,
    commercial_use_status: &'static str,
    production_allowed: bool,
    enabled: bool,
}

#[derive(Debug, Clone, Copy)]
struct MetadataDatasetSeed {
    dataset_id: &'static str,
    source_id: &'static str,
    display_name: &'static str,
    frequency_set_json: &'static str,
    region_set_json: &'static str,
    supports_backfill: bool,
    supports_incremental: bool,
    supports_vintage: bool,
    expected_latency_seconds: i64,
    config_version: &'static str,
    enabled: bool,
}

#[derive(Debug, Clone, Copy)]
struct MetadataEntitySeed {
    entity_id: &'static str,
    entity_type: &'static str,
    display_name: &'static str,
    iso_country_code: &'static str,
    currency: &'static str,
    metadata_json: &'static str,
}

const fn sqlite_flag(value: bool) -> i64 {
    if value {
        1
    } else {
        0
    }
}

fn metadata_source_seeds() -> [MetadataSourceSeed; 6] {
    [
        MetadataSourceSeed {
            source_id: "fred",
            display_name: "FRED",
            source_type: "macro_financial_timeseries",
            official_url: "https://fred.stlouisfed.org/",
            documentation_url: "https://fred.stlouisfed.org/graph/fredgraph.csv",
            access_method: "graph_csv",
            auth_required: false,
            auth_secret_ref: None,
            rate_limit_policy_json:
                r#"{"policy":"public_graph_csv","note":"No API key; cache locally and keep conservative cadence."}"#,
            license_note:
                "Use according to FRED source-specific notes; public graph CSV has no vintage fields.",
            commercial_use_status: "review_required",
            production_allowed: true,
            enabled: true,
        },
        MetadataSourceSeed {
            source_id: "treasury",
            display_name: "U.S. Treasury",
            source_type: "government_timeseries",
            official_url: "https://home.treasury.gov/",
            documentation_url:
                "https://home.treasury.gov/resource-center/data-chart-center/interest-rates",
            access_method: "xml_download",
            auth_required: false,
            auth_secret_ref: None,
            rate_limit_policy_json:
                r#"{"policy":"public_xml","note":"Fetch by month and cache locally."}"#,
            license_note: "Official U.S. Treasury daily yield curve publication.",
            commercial_use_status: "public_official",
            production_allowed: true,
            enabled: true,
        },
        MetadataSourceSeed {
            source_id: "world_bank",
            display_name: "World Bank Indicators",
            source_type: "global_macro",
            official_url: "https://api.worldbank.org/",
            documentation_url:
                "https://datahelpdesk.worldbank.org/knowledgebase/articles/889392",
            access_method: "rest_api",
            auth_required: false,
            auth_secret_ref: None,
            rate_limit_policy_json:
                r#"{"policy":"public_rest_api","note":"Annual slow variables; no API key required."}"#,
            license_note: "Official World Bank Indicators API.",
            commercial_use_status: "public_official",
            production_allowed: true,
            enabled: true,
        },
        MetadataSourceSeed {
            source_id: "boj",
            display_name: "Bank of Japan Statistics API",
            source_type: "government_timeseries",
            official_url: "https://www.boj.or.jp/en/statistics/",
            documentation_url: "https://www.stat-search.boj.or.jp/info/api_manual_en.pdf",
            access_method: "rest_csv",
            auth_required: false,
            auth_secret_ref: None,
            rate_limit_policy_json:
                r#"{"policy":"public_rest_csv","note":"Official BOJ API, no key required. Prefer BOJ for USDJPY and Japan short rates, cache locally."}"#,
            license_note: "Official BOJ statistics API for FX daily and money market time series.",
            commercial_use_status: "public_official",
            production_allowed: true,
            enabled: true,
        },
        MetadataSourceSeed {
            source_id: "sec_edgar",
            display_name: "SEC EDGAR",
            source_type: "filings_events",
            official_url: "https://www.sec.gov/edgar/search/",
            documentation_url: "https://www.sec.gov/edgar/sec-api-documentation",
            access_method: "json_download",
            auth_required: false,
            auth_secret_ref: None,
            rate_limit_policy_json:
                r#"{"policy":"fair_access","note":"Sequential requests, local cache, and archived submissions only when the requested range overlaps."}"#,
            license_note:
                "Official SEC submissions JSON. Local event features are aggregated from filing metadata only; no paid key required.",
            commercial_use_status: "public_official",
            production_allowed: true,
            enabled: true,
        },
        MetadataSourceSeed {
            source_id: "gdelt",
            display_name: "GDELT",
            source_type: "news_events",
            official_url: "https://api.gdeltproject.org/",
            documentation_url: "https://blog.gdeltproject.org/gdelt-doc-2-0-api-debuts/amp/",
            access_method: "rest_api",
            auth_required: false,
            auth_secret_ref: None,
            rate_limit_policy_json:
                r#"{"policy":"public_doc_api","note":"Strictly one request every 5+ seconds, cache locally, and keep it as a low-confidence auxiliary source."}"#,
            license_note:
                "Public GDELT DOC API used only for aggregate news counts. Keep it as an auxiliary prototype signal.",
            commercial_use_status: "review_required",
            production_allowed: false,
            enabled: true,
        },
    ]
}

fn metadata_dataset_seeds() -> [MetadataDatasetSeed; 8] {
    [
        MetadataDatasetSeed {
            dataset_id: FRED_DATASET_ID,
            source_id: "fred",
            display_name: "FRED series observations",
            frequency_set_json: r#"["daily","weekly","monthly","quarterly"]"#,
            region_set_json: r#"["us"]"#,
            supports_backfill: true,
            supports_incremental: true,
            supports_vintage: false,
            expected_latency_seconds: 86_400,
            config_version: "fred_graph_csv_seed_v2_20260530",
            enabled: true,
        },
        MetadataDatasetSeed {
            dataset_id: TREASURY_YIELD_DATASET_ID,
            source_id: "treasury",
            display_name: "Daily Treasury yield curve",
            frequency_set_json: r#"["daily"]"#,
            region_set_json: r#"["us"]"#,
            supports_backfill: true,
            supports_incremental: true,
            supports_vintage: false,
            expected_latency_seconds: 86_400,
            config_version: "treasury_yield_seed_v1_20260530",
            enabled: true,
        },
        MetadataDatasetSeed {
            dataset_id: WORLD_BANK_DATASET_ID,
            source_id: "world_bank",
            display_name: "World Bank country indicators",
            frequency_set_json: r#"["annual"]"#,
            region_set_json: r#"["us"]"#,
            supports_backfill: true,
            supports_incremental: true,
            supports_vintage: false,
            expected_latency_seconds: 86_400,
            config_version: "world_bank_seed_v1_20260530",
            enabled: true,
        },
        MetadataDatasetSeed {
            dataset_id: BOJ_FX_DATASET_ID,
            source_id: "boj",
            display_name: "BOJ foreign exchange daily series",
            frequency_set_json: r#"["daily"]"#,
            region_set_json: r#"["jp","us"]"#,
            supports_backfill: true,
            supports_incremental: true,
            supports_vintage: false,
            expected_latency_seconds: 86_400,
            config_version: "boj_fx_seed_v1_20260530",
            enabled: true,
        },
        MetadataDatasetSeed {
            dataset_id: BOJ_MONEY_MARKET_DATASET_ID,
            source_id: "boj",
            display_name: "BOJ money market call rate series",
            frequency_set_json: r#"["daily"]"#,
            region_set_json: r#"["jp"]"#,
            supports_backfill: true,
            supports_incremental: true,
            supports_vintage: false,
            expected_latency_seconds: 86_400,
            config_version: "boj_money_market_seed_v1_20260530",
            enabled: true,
        },
        MetadataDatasetSeed {
            dataset_id: SEC_SUBMISSIONS_DATASET_ID,
            source_id: "sec_edgar",
            display_name: "SEC company submissions metadata",
            frequency_set_json: r#"["event"]"#,
            region_set_json: r#"["us"]"#,
            supports_backfill: true,
            supports_incremental: true,
            supports_vintage: false,
            expected_latency_seconds: 86_400,
            config_version: "sec_submissions_seed_v1_20260531",
            enabled: true,
        },
        MetadataDatasetSeed {
            dataset_id: SEC_EVENTS_DATASET_ID,
            source_id: "sec_edgar",
            display_name: "SEC filing event aggregates",
            frequency_set_json: r#"["daily"]"#,
            region_set_json: r#"["us"]"#,
            supports_backfill: true,
            supports_incremental: true,
            supports_vintage: false,
            expected_latency_seconds: 86_400,
            config_version: "sec_events_seed_v1_20260531",
            enabled: true,
        },
        MetadataDatasetSeed {
            dataset_id: GDELT_DOC_DATASET_ID,
            source_id: "gdelt",
            display_name: "GDELT DOC API timeline aggregates",
            frequency_set_json: r#"["daily"]"#,
            region_set_json: r#"["us","global"]"#,
            supports_backfill: true,
            supports_incremental: true,
            supports_vintage: false,
            expected_latency_seconds: 86_400,
            config_version: "gdelt_doc_seed_v1_20260531",
            enabled: true,
        },
    ]
}

fn metadata_entity_seeds() -> [MetadataEntitySeed; 2] {
    [
        MetadataEntitySeed {
            entity_id: "us",
            entity_type: "country",
            display_name: "United States",
            iso_country_code: "USA",
            currency: "USD",
            metadata_json: "{}",
        },
        MetadataEntitySeed {
            entity_id: "jp",
            entity_type: "country",
            display_name: "Japan",
            iso_country_code: "JPN",
            currency: "JPY",
            metadata_json: "{}",
        },
    ]
}

impl SqliteStore {
    pub async fn seed_fred_metadata(&self) -> Result<(), StorageError> {
        self.seed_metadata_catalog().await?;
        self.seed_metadata_entities().await?;
        self.seed_indicator_catalog().await?;
        Ok(())
    }

    async fn seed_metadata_catalog(&self) -> Result<(), StorageError> {
        for seed in metadata_source_seeds() {
            self.upsert_metadata_source(seed).await?;
        }
        for seed in metadata_dataset_seeds() {
            self.upsert_metadata_dataset(seed).await?;
        }
        Ok(())
    }

    async fn seed_metadata_entities(&self) -> Result<(), StorageError> {
        for seed in metadata_entity_seeds() {
            self.upsert_metadata_entity(seed).await?;
        }
        Ok(())
    }

    async fn seed_indicator_catalog(&self) -> Result<(), StorageError> {
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

    async fn upsert_metadata_source(&self, seed: MetadataSourceSeed) -> Result<(), StorageError> {
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
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
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
        .bind(seed.source_id)
        .bind(seed.display_name)
        .bind(seed.source_type)
        .bind(seed.official_url)
        .bind(seed.documentation_url)
        .bind(seed.access_method)
        .bind(sqlite_flag(seed.auth_required))
        .bind(seed.auth_secret_ref)
        .bind(seed.rate_limit_policy_json)
        .bind(seed.license_note)
        .bind(seed.commercial_use_status)
        .bind(sqlite_flag(seed.production_allowed))
        .bind(sqlite_flag(seed.enabled))
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn upsert_metadata_dataset(&self, seed: MetadataDatasetSeed) -> Result<(), StorageError> {
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
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
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
        .bind(seed.dataset_id)
        .bind(seed.source_id)
        .bind(seed.display_name)
        .bind(seed.frequency_set_json)
        .bind(seed.region_set_json)
        .bind(sqlite_flag(seed.supports_backfill))
        .bind(sqlite_flag(seed.supports_incremental))
        .bind(sqlite_flag(seed.supports_vintage))
        .bind(seed.expected_latency_seconds)
        .bind(seed.config_version)
        .bind(sqlite_flag(seed.enabled))
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn upsert_metadata_entity(&self, seed: MetadataEntitySeed) -> Result<(), StorageError> {
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
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            ON CONFLICT(entity_id) DO UPDATE SET
                display_name = excluded.display_name,
                iso_country_code = excluded.iso_country_code,
                currency = excluded.currency
            "#,
        )
        .bind(seed.entity_id)
        .bind(seed.entity_type)
        .bind(seed.display_name)
        .bind(seed.iso_country_code)
        .bind(seed.currency)
        .bind(seed.metadata_json)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}
