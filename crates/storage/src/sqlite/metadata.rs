use sqlx::Row;

use crate::{parse_frequency, StorageError};

use super::{
    boj_indicator_seeds, fred_indicator_seeds, gdelt_indicator_seeds, sec_event_indicator_seeds,
    world_bank_indicator_seeds, ExternalIndicatorMapping, SqliteStore, BOJ_FX_DATASET_ID,
    BOJ_MONEY_MARKET_DATASET_ID, FRED_DATASET_ID, GDELT_DOC_DATASET_ID, SEC_EVENTS_DATASET_ID,
    SEC_SUBMISSIONS_DATASET_ID, TREASURY_YIELD_DATASET_ID, WORLD_BANK_DATASET_ID,
};

impl SqliteStore {
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
}
