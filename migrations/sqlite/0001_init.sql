CREATE TABLE IF NOT EXISTS metadata_sources (
    source_id TEXT PRIMARY KEY,
    display_name TEXT NOT NULL,
    source_type TEXT NOT NULL,
    official_url TEXT,
    documentation_url TEXT,
    access_method TEXT NOT NULL,
    auth_required INTEGER NOT NULL DEFAULT 0,
    auth_secret_ref TEXT,
    rate_limit_policy_json TEXT NOT NULL DEFAULT '{}',
    license_note TEXT NOT NULL DEFAULT '',
    commercial_use_status TEXT NOT NULL DEFAULT 'unknown',
    production_allowed INTEGER NOT NULL DEFAULT 0,
    enabled INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS metadata_datasets (
    dataset_id TEXT PRIMARY KEY,
    source_id TEXT NOT NULL REFERENCES metadata_sources(source_id),
    display_name TEXT NOT NULL,
    frequency_set_json TEXT NOT NULL DEFAULT '[]',
    region_set_json TEXT NOT NULL DEFAULT '[]',
    supports_backfill INTEGER NOT NULL DEFAULT 1,
    supports_incremental INTEGER NOT NULL DEFAULT 1,
    supports_vintage INTEGER NOT NULL DEFAULT 0,
    expected_latency_seconds INTEGER,
    config_version TEXT NOT NULL,
    enabled INTEGER NOT NULL DEFAULT 1
);

CREATE TABLE IF NOT EXISTS metadata_indicators (
    indicator_id TEXT PRIMARY KEY,
    display_name TEXT NOT NULL,
    dimension TEXT NOT NULL,
    subdimension TEXT,
    description TEXT NOT NULL DEFAULT '',
    unit TEXT NOT NULL,
    currency TEXT,
    frequency TEXT NOT NULL,
    risk_direction TEXT NOT NULL,
    default_transform TEXT,
    default_source_id TEXT REFERENCES metadata_sources(source_id),
    quality_tier TEXT NOT NULL DEFAULT 'core',
    enabled INTEGER NOT NULL DEFAULT 1
);

CREATE TABLE IF NOT EXISTS metadata_external_indicator_mappings (
    mapping_id TEXT PRIMARY KEY,
    indicator_id TEXT NOT NULL REFERENCES metadata_indicators(indicator_id),
    source_id TEXT NOT NULL REFERENCES metadata_sources(source_id),
    dataset_id TEXT NOT NULL REFERENCES metadata_datasets(dataset_id),
    external_code TEXT NOT NULL,
    external_params_json TEXT NOT NULL DEFAULT '{}',
    valid_from TEXT NOT NULL DEFAULT CURRENT_DATE,
    valid_to TEXT,
    priority INTEGER NOT NULL DEFAULT 100,
    UNIQUE (indicator_id, source_id, dataset_id, external_code)
);

CREATE TABLE IF NOT EXISTS metadata_entities (
    entity_id TEXT PRIMARY KEY,
    entity_type TEXT NOT NULL,
    display_name TEXT NOT NULL,
    iso_country_code TEXT,
    currency TEXT,
    parent_entity_id TEXT REFERENCES metadata_entities(entity_id),
    metadata_json TEXT NOT NULL DEFAULT '{}'
);

CREATE TABLE IF NOT EXISTS ingest_runs (
    run_id TEXT PRIMARY KEY,
    job_id TEXT,
    source_id TEXT NOT NULL,
    dataset_id TEXT NOT NULL,
    target_id TEXT,
    run_mode TEXT NOT NULL,
    status TEXT NOT NULL,
    started_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    finished_at TEXT,
    attempt INTEGER NOT NULL DEFAULT 1,
    watermark_before_json TEXT,
    watermark_after_json TEXT,
    records_read INTEGER NOT NULL DEFAULT 0,
    records_written INTEGER NOT NULL DEFAULT 0,
    error_type TEXT,
    error_message TEXT
);

CREATE TABLE IF NOT EXISTS ingest_watermarks (
    source_id TEXT NOT NULL,
    dataset_id TEXT NOT NULL,
    target_id TEXT NOT NULL,
    last_successful_period TEXT,
    last_publication_time TEXT,
    last_revision_time TEXT,
    last_run_id TEXT,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (source_id, dataset_id, target_id)
);

CREATE TABLE IF NOT EXISTS raw_responses (
    raw_payload_id TEXT PRIMARY KEY,
    run_id TEXT REFERENCES ingest_runs(run_id),
    source_id TEXT NOT NULL,
    dataset_id TEXT NOT NULL,
    request_url TEXT NOT NULL,
    request_params_hash TEXT,
    response_hash TEXT NOT NULL,
    content_type TEXT NOT NULL,
    content_length INTEGER,
    raw_file_path TEXT NOT NULL,
    fetched_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS ts_indicator_observations (
    indicator_id TEXT NOT NULL REFERENCES metadata_indicators(indicator_id),
    entity_id TEXT NOT NULL REFERENCES metadata_entities(entity_id),
    as_of_date TEXT NOT NULL,
    period_start TEXT,
    period_end TEXT,
    frequency TEXT NOT NULL,
    value REAL NOT NULL,
    unit TEXT NOT NULL,
    currency TEXT,
    source_id TEXT NOT NULL,
    dataset_id TEXT NOT NULL,
    revision_time TEXT NOT NULL DEFAULT '',
    publication_time TEXT,
    vintage_date TEXT NOT NULL DEFAULT '',
    raw_payload_id TEXT REFERENCES raw_responses(raw_payload_id),
    quality_score REAL NOT NULL,
    quality_flags_json TEXT NOT NULL DEFAULT '[]',
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (indicator_id, entity_id, as_of_date, frequency, source_id, vintage_date)
);

CREATE TABLE IF NOT EXISTS analytics_risk_snapshots (
    snapshot_id TEXT PRIMARY KEY,
    entity_id TEXT NOT NULL,
    market_scope TEXT NOT NULL,
    as_of_date TEXT NOT NULL,
    overall_score REAL NOT NULL,
    overall_level TEXT NOT NULL,
    method_version TEXT NOT NULL,
    snapshot_json TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE (entity_id, market_scope, as_of_date, method_version)
);

CREATE INDEX IF NOT EXISTS idx_ts_indicator_entity_date
    ON ts_indicator_observations(indicator_id, entity_id, as_of_date);

CREATE INDEX IF NOT EXISTS idx_ts_entity_date
    ON ts_indicator_observations(entity_id, as_of_date);

CREATE INDEX IF NOT EXISTS idx_ingest_runs_source_started
    ON ingest_runs(source_id, dataset_id, started_at);

CREATE INDEX IF NOT EXISTS idx_ingest_watermarks_target
    ON ingest_watermarks(source_id, dataset_id, target_id);

CREATE INDEX IF NOT EXISTS idx_analytics_snapshots_entity_date
    ON analytics_risk_snapshots(entity_id, market_scope, as_of_date);
