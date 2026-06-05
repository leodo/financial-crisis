use super::super::super::{
    BOJ_FX_DATASET_ID, BOJ_MONEY_MARKET_DATASET_ID, FRED_DATASET_ID, GDELT_DOC_DATASET_ID,
    SEC_EVENTS_DATASET_ID, SEC_SUBMISSIONS_DATASET_ID, TREASURY_YIELD_DATASET_ID,
    WORLD_BANK_DATASET_ID,
};

#[derive(Debug, Clone, Copy)]
pub(super) struct MetadataSourceSeed {
    pub(super) source_id: &'static str,
    pub(super) display_name: &'static str,
    pub(super) source_type: &'static str,
    pub(super) official_url: &'static str,
    pub(super) documentation_url: &'static str,
    pub(super) access_method: &'static str,
    pub(super) auth_required: bool,
    pub(super) auth_secret_ref: Option<&'static str>,
    pub(super) rate_limit_policy_json: &'static str,
    pub(super) license_note: &'static str,
    pub(super) commercial_use_status: &'static str,
    pub(super) production_allowed: bool,
    pub(super) enabled: bool,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct MetadataDatasetSeed {
    pub(super) dataset_id: &'static str,
    pub(super) source_id: &'static str,
    pub(super) display_name: &'static str,
    pub(super) frequency_set_json: &'static str,
    pub(super) region_set_json: &'static str,
    pub(super) supports_backfill: bool,
    pub(super) supports_incremental: bool,
    pub(super) supports_vintage: bool,
    pub(super) expected_latency_seconds: i64,
    pub(super) config_version: &'static str,
    pub(super) enabled: bool,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct MetadataEntitySeed {
    pub(super) entity_id: &'static str,
    pub(super) entity_type: &'static str,
    pub(super) display_name: &'static str,
    pub(super) iso_country_code: &'static str,
    pub(super) currency: &'static str,
    pub(super) metadata_json: &'static str,
}

pub(super) fn metadata_source_seeds() -> [MetadataSourceSeed; 6] {
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

pub(super) fn metadata_dataset_seeds() -> [MetadataDatasetSeed; 8] {
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

pub(super) fn metadata_entity_seeds() -> [MetadataEntitySeed; 2] {
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
