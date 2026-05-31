CREATE EXTENSION IF NOT EXISTS pgcrypto;
CREATE EXTENSION IF NOT EXISTS timescaledb;

CREATE SCHEMA IF NOT EXISTS metadata;
CREATE SCHEMA IF NOT EXISTS ingest;
CREATE SCHEMA IF NOT EXISTS raw;
CREATE SCHEMA IF NOT EXISTS ts;
CREATE SCHEMA IF NOT EXISTS quality;
CREATE SCHEMA IF NOT EXISTS alerts;
CREATE SCHEMA IF NOT EXISTS audit;

CREATE TABLE IF NOT EXISTS metadata.sources (
    source_id TEXT PRIMARY KEY,
    display_name TEXT NOT NULL,
    source_type TEXT NOT NULL,
    official_url TEXT,
    documentation_url TEXT,
    access_method TEXT NOT NULL,
    auth_required BOOLEAN NOT NULL DEFAULT FALSE,
    auth_secret_ref TEXT,
    rate_limit_policy JSONB NOT NULL DEFAULT '{}'::jsonb,
    license_note TEXT NOT NULL DEFAULT '',
    commercial_use_status TEXT NOT NULL DEFAULT 'unknown',
    production_allowed BOOLEAN NOT NULL DEFAULT FALSE,
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS metadata.datasets (
    dataset_id TEXT PRIMARY KEY,
    source_id TEXT NOT NULL REFERENCES metadata.sources(source_id),
    display_name TEXT NOT NULL,
    frequency_set TEXT[] NOT NULL DEFAULT ARRAY[]::TEXT[],
    region_set TEXT[] NOT NULL DEFAULT ARRAY[]::TEXT[],
    supports_backfill BOOLEAN NOT NULL DEFAULT TRUE,
    supports_incremental BOOLEAN NOT NULL DEFAULT TRUE,
    supports_vintage BOOLEAN NOT NULL DEFAULT FALSE,
    expected_latency INTERVAL,
    config_version TEXT NOT NULL,
    enabled BOOLEAN NOT NULL DEFAULT TRUE
);

CREATE TABLE IF NOT EXISTS metadata.indicators (
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
    default_source_id TEXT REFERENCES metadata.sources(source_id),
    quality_tier TEXT NOT NULL DEFAULT 'core',
    enabled BOOLEAN NOT NULL DEFAULT TRUE
);

CREATE TABLE IF NOT EXISTS metadata.external_indicator_mappings (
    mapping_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    indicator_id TEXT NOT NULL REFERENCES metadata.indicators(indicator_id),
    source_id TEXT NOT NULL REFERENCES metadata.sources(source_id),
    dataset_id TEXT NOT NULL REFERENCES metadata.datasets(dataset_id),
    external_code TEXT NOT NULL,
    external_params JSONB NOT NULL DEFAULT '{}'::jsonb,
    valid_from DATE NOT NULL DEFAULT CURRENT_DATE,
    valid_to DATE,
    priority INTEGER NOT NULL DEFAULT 100
);

CREATE TABLE IF NOT EXISTS metadata.entities (
    entity_id TEXT PRIMARY KEY,
    entity_type TEXT NOT NULL,
    display_name TEXT NOT NULL,
    iso_country_code TEXT,
    currency TEXT,
    parent_entity_id TEXT REFERENCES metadata.entities(entity_id),
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb
);

CREATE TABLE IF NOT EXISTS metadata.calendars (
    calendar_id TEXT NOT NULL,
    region TEXT NOT NULL,
    calendar_type TEXT NOT NULL,
    calendar_date DATE NOT NULL,
    is_open BOOLEAN NOT NULL,
    note TEXT,
    PRIMARY KEY (calendar_id, calendar_date)
);

CREATE TABLE IF NOT EXISTS ingest.jobs (
    job_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    source_id TEXT NOT NULL REFERENCES metadata.sources(source_id),
    dataset_id TEXT NOT NULL REFERENCES metadata.datasets(dataset_id),
    target_id TEXT,
    run_mode TEXT NOT NULL,
    schedule TEXT NOT NULL,
    priority INTEGER NOT NULL DEFAULT 100,
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    next_run_at TIMESTAMPTZ,
    config_version TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS ingest.runs (
    run_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    job_id UUID REFERENCES ingest.jobs(job_id),
    source_id TEXT NOT NULL,
    dataset_id TEXT NOT NULL,
    target_id TEXT,
    run_mode TEXT NOT NULL,
    status TEXT NOT NULL,
    started_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    finished_at TIMESTAMPTZ,
    attempt INTEGER NOT NULL DEFAULT 1,
    watermark_before JSONB,
    watermark_after JSONB,
    records_read INTEGER NOT NULL DEFAULT 0,
    records_written INTEGER NOT NULL DEFAULT 0,
    error_type TEXT,
    error_message TEXT
);

CREATE TABLE IF NOT EXISTS ingest.watermarks (
    source_id TEXT NOT NULL,
    dataset_id TEXT NOT NULL,
    target_id TEXT NOT NULL,
    last_successful_period DATE,
    last_publication_time TIMESTAMPTZ,
    last_revision_time TIMESTAMPTZ,
    last_run_id UUID,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (source_id, dataset_id, target_id)
);

CREATE TABLE IF NOT EXISTS raw.objects (
    raw_payload_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    run_id UUID REFERENCES ingest.runs(run_id),
    source_id TEXT NOT NULL,
    dataset_id TEXT NOT NULL,
    request_url TEXT NOT NULL,
    request_params_hash TEXT,
    response_hash TEXT NOT NULL,
    content_type TEXT NOT NULL,
    content_length BIGINT,
    raw_object_uri TEXT NOT NULL,
    fetched_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS ts.indicator_observations (
    indicator_id TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    as_of_date DATE NOT NULL,
    period_start DATE,
    period_end DATE,
    frequency TEXT NOT NULL,
    value DOUBLE PRECISION NOT NULL,
    unit TEXT NOT NULL,
    currency TEXT,
    source_id TEXT NOT NULL,
    dataset_id TEXT NOT NULL,
    revision_time TIMESTAMPTZ,
    publication_time TIMESTAMPTZ,
    raw_payload_id UUID,
    quality_score DOUBLE PRECISION NOT NULL,
    quality_flags TEXT[] NOT NULL DEFAULT ARRAY[]::TEXT[],
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (indicator_id, entity_id, as_of_date, frequency, source_id, revision_time)
);

SELECT create_hypertable('ts.indicator_observations', 'as_of_date', if_not_exists => TRUE);

CREATE TABLE IF NOT EXISTS ts.feature_values (
    feature_id TEXT NOT NULL,
    indicator_id TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    as_of_date DATE NOT NULL,
    feature_name TEXT NOT NULL,
    feature_value DOUBLE PRECISION NOT NULL,
    lookback_window TEXT,
    method_version TEXT NOT NULL,
    quality_score DOUBLE PRECISION NOT NULL,
    PRIMARY KEY (feature_id, indicator_id, entity_id, as_of_date, method_version)
);

SELECT create_hypertable('ts.feature_values', 'as_of_date', if_not_exists => TRUE);

CREATE TABLE IF NOT EXISTS ts.risk_scores (
    score_id UUID NOT NULL DEFAULT gen_random_uuid(),
    score_scope TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    as_of_date DATE NOT NULL,
    dimension TEXT NOT NULL,
    score DOUBLE PRECISION NOT NULL,
    level TEXT NOT NULL,
    method_version TEXT NOT NULL,
    top_contributors JSONB NOT NULL DEFAULT '[]'::jsonb,
    explanation JSONB NOT NULL DEFAULT '{}'::jsonb,
    quality_score DOUBLE PRECISION NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (score_id, as_of_date)
);

SELECT create_hypertable('ts.risk_scores', 'as_of_date', if_not_exists => TRUE);

CREATE TABLE IF NOT EXISTS quality.rules (
    rule_id TEXT PRIMARY KEY,
    rule_name TEXT NOT NULL,
    scope_type TEXT NOT NULL,
    scope_id TEXT NOT NULL,
    severity TEXT NOT NULL,
    config JSONB NOT NULL DEFAULT '{}'::jsonb,
    enabled BOOLEAN NOT NULL DEFAULT TRUE
);

CREATE TABLE IF NOT EXISTS quality.check_results (
    check_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    run_id UUID REFERENCES ingest.runs(run_id),
    indicator_id TEXT,
    entity_id TEXT,
    as_of_date DATE,
    rule_id TEXT REFERENCES quality.rules(rule_id),
    status TEXT NOT NULL,
    severity TEXT NOT NULL,
    message TEXT NOT NULL,
    observed_value JSONB,
    expected_value JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS alerts.events (
    alert_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    entity_id TEXT NOT NULL,
    scope TEXT NOT NULL,
    dimension TEXT,
    level TEXT NOT NULL,
    status TEXT NOT NULL,
    triggered_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    triggered_as_of_date DATE NOT NULL,
    resolved_at TIMESTAMPTZ,
    score DOUBLE PRECISION NOT NULL,
    trigger_reason TEXT NOT NULL,
    contributors JSONB NOT NULL DEFAULT '[]'::jsonb,
    related_indicators TEXT[] NOT NULL DEFAULT ARRAY[]::TEXT[],
    method_version TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS alerts.event_history (
    history_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    alert_id UUID NOT NULL REFERENCES alerts.events(alert_id),
    event_type TEXT NOT NULL,
    from_status TEXT,
    to_status TEXT,
    actor TEXT NOT NULL DEFAULT 'system',
    note TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS audit.model_releases (
    release_id TEXT PRIMARY KEY,
    market_scope TEXT NOT NULL,
    status TEXT NOT NULL,
    probability_mode TEXT NOT NULL,
    serving_status TEXT NOT NULL,
    bundle_uri TEXT NOT NULL,
    manifest_json JSONB NOT NULL DEFAULT '{}'::jsonb,
    feature_set_version TEXT NOT NULL,
    label_version TEXT NOT NULL,
    prob_model_version TEXT NOT NULL,
    calibration_version TEXT NOT NULL,
    posture_policy_version TEXT NOT NULL,
    action_playbook_version TEXT NOT NULL,
    point_in_time_mode TEXT NOT NULL,
    training_range_start DATE,
    training_range_end DATE,
    calibration_range_start DATE,
    calibration_range_end DATE,
    evaluation_range_start DATE,
    evaluation_range_end DATE,
    brier_score DOUBLE PRECISION,
    log_loss DOUBLE PRECISION,
    ece DOUBLE PRECISION,
    note TEXT NOT NULL DEFAULT '',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    activated_at TIMESTAMPTZ,
    retired_at TIMESTAMPTZ
);

CREATE TABLE IF NOT EXISTS audit.active_model_pointers (
    market_scope TEXT PRIMARY KEY,
    release_id TEXT NOT NULL REFERENCES audit.model_releases(release_id) ON DELETE CASCADE,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_by TEXT NOT NULL DEFAULT 'system'
);

CREATE TABLE IF NOT EXISTS audit.prediction_snapshots (
    snapshot_id TEXT PRIMARY KEY,
    entity_id TEXT NOT NULL,
    market_scope TEXT NOT NULL,
    as_of_date DATE NOT NULL,
    release_id TEXT REFERENCES audit.model_releases(release_id) ON DELETE SET NULL,
    probability_mode TEXT NOT NULL,
    release_status TEXT NOT NULL,
    point_in_time_mode TEXT NOT NULL,
    overall_score DOUBLE PRECISION NOT NULL,
    external_shock_score DOUBLE PRECISION NOT NULL,
    raw_p_5d DOUBLE PRECISION NOT NULL,
    raw_p_20d DOUBLE PRECISION NOT NULL,
    raw_p_60d DOUBLE PRECISION NOT NULL,
    calibrated_p_5d DOUBLE PRECISION NOT NULL,
    calibrated_p_20d DOUBLE PRECISION NOT NULL,
    calibrated_p_60d DOUBLE PRECISION NOT NULL,
    posture TEXT NOT NULL,
    time_to_risk_bucket TEXT NOT NULL,
    feature_set_version TEXT NOT NULL,
    label_version TEXT NOT NULL,
    coverage_score DOUBLE PRECISION NOT NULL,
    freshness_status TEXT NOT NULL,
    method_version TEXT NOT NULL,
    recorded_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS audit.feature_snapshots (
    snapshot_id TEXT PRIMARY KEY,
    entity_id TEXT NOT NULL,
    market_scope TEXT NOT NULL,
    as_of_date DATE NOT NULL,
    feature_set_version TEXT NOT NULL,
    point_in_time_mode TEXT NOT NULL,
    visibility_status TEXT NOT NULL,
    latest_visible_at TIMESTAMPTZ,
    coverage_score DOUBLE PRECISION NOT NULL,
    core_feature_coverage DOUBLE PRECISION NOT NULL,
    trigger_feature_coverage DOUBLE PRECISION NOT NULL,
    external_feature_coverage DOUBLE PRECISION NOT NULL,
    feature_count INTEGER NOT NULL,
    features_json JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS audit.formal_datasets (
    dataset_key TEXT PRIMARY KEY,
    dataset_id TEXT NOT NULL,
    dataset_version TEXT NOT NULL,
    market_scope TEXT NOT NULL,
    feature_set_version TEXT NOT NULL,
    label_version TEXT NOT NULL,
    scenario_set_version TEXT NOT NULL,
    point_in_time_mode TEXT NOT NULL,
    from_date DATE,
    to_date DATE,
    train_end_date DATE,
    calibration_end_date DATE,
    evaluation_start_date DATE,
    row_count INTEGER NOT NULL,
    note TEXT NOT NULL DEFAULT '',
    manifest_json JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS audit.formal_dataset_rows (
    row_id TEXT PRIMARY KEY,
    dataset_key TEXT NOT NULL REFERENCES audit.formal_datasets(dataset_key) ON DELETE CASCADE,
    split_name TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    market_scope TEXT NOT NULL,
    as_of_date DATE NOT NULL,
    point_in_time_mode TEXT NOT NULL,
    latest_visible_at TIMESTAMPTZ,
    coverage_score DOUBLE PRECISION NOT NULL,
    core_feature_coverage DOUBLE PRECISION NOT NULL,
    trigger_feature_coverage DOUBLE PRECISION NOT NULL,
    external_feature_coverage DOUBLE PRECISION NOT NULL,
    sample_quality_grade TEXT NOT NULL,
    primary_scenario_id TEXT,
    scenario_family TEXT,
    label_5d SMALLINT NOT NULL,
    label_20d SMALLINT NOT NULL,
    label_60d SMALLINT NOT NULL,
    features_json JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS audit.config_changes (
    change_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    object_type TEXT NOT NULL,
    object_id TEXT NOT NULL,
    before_value JSONB,
    after_value JSONB,
    actor TEXT NOT NULL,
    reason TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_indicator_observations_lookup
    ON ts.indicator_observations (indicator_id, entity_id, as_of_date DESC);

CREATE INDEX IF NOT EXISTS idx_indicator_observations_source
    ON ts.indicator_observations (source_id, dataset_id, as_of_date DESC);

CREATE INDEX IF NOT EXISTS idx_risk_scores_lookup
    ON ts.risk_scores (entity_id, score_scope, as_of_date DESC);

CREATE INDEX IF NOT EXISTS idx_ingest_runs_status
    ON ingest.runs (source_id, dataset_id, status, started_at DESC);

CREATE INDEX IF NOT EXISTS idx_alerts_events_status
    ON alerts.events (status, level, triggered_at DESC);

CREATE INDEX IF NOT EXISTS idx_model_releases_scope_status
    ON audit.model_releases (market_scope, status, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_prediction_snapshots_scope_date
    ON audit.prediction_snapshots (market_scope, as_of_date DESC);

CREATE INDEX IF NOT EXISTS idx_prediction_snapshots_release_date
    ON audit.prediction_snapshots (release_id, as_of_date DESC);

CREATE INDEX IF NOT EXISTS idx_feature_snapshots_scope_version_date
    ON audit.feature_snapshots (market_scope, feature_set_version, as_of_date DESC);

CREATE INDEX IF NOT EXISTS idx_formal_datasets_scope_version
    ON audit.formal_datasets (market_scope, dataset_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_formal_dataset_rows_dataset_split_date
    ON audit.formal_dataset_rows (dataset_key, split_name, as_of_date DESC);
