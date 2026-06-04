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

CREATE TABLE IF NOT EXISTS analytics_backtest_runs (
    run_id TEXT PRIMARY KEY,
    entity_id TEXT NOT NULL,
    market_scope TEXT NOT NULL,
    data_mode TEXT NOT NULL,
    point_in_time_mode TEXT NOT NULL,
    status TEXT NOT NULL,
    scenario_scope TEXT,
    from_date TEXT NOT NULL,
    to_date TEXT NOT NULL,
    history_points INTEGER NOT NULL,
    scenario_summary_count INTEGER NOT NULL DEFAULT 0,
    method_version TEXT NOT NULL,
    started_at TEXT NOT NULL,
    finished_at TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS analytics_backtest_daily_results (
    run_id TEXT NOT NULL REFERENCES analytics_backtest_runs(run_id) ON DELETE CASCADE,
    as_of_date TEXT NOT NULL,
    overall_score REAL NOT NULL,
    p_5d REAL NOT NULL,
    p_20d REAL NOT NULL,
    p_60d REAL NOT NULL,
    posture TEXT NOT NULL,
    crisis_window_open INTEGER NOT NULL,
    result_json TEXT NOT NULL,
    PRIMARY KEY (run_id, as_of_date)
);

CREATE TABLE IF NOT EXISTS analytics_backtest_scenario_summaries (
    run_id TEXT NOT NULL REFERENCES analytics_backtest_runs(run_id) ON DELETE CASCADE,
    scenario_id TEXT NOT NULL,
    summary_json TEXT NOT NULL,
    PRIMARY KEY (run_id, scenario_id)
);

CREATE TABLE IF NOT EXISTS analytics_model_releases (
    release_id TEXT PRIMARY KEY,
    market_scope TEXT NOT NULL,
    status TEXT NOT NULL,
    probability_mode TEXT NOT NULL,
    serving_status TEXT NOT NULL,
    bundle_uri TEXT NOT NULL,
    manifest_json TEXT NOT NULL,
    feature_set_version TEXT NOT NULL,
    label_version TEXT NOT NULL,
    prob_model_version TEXT NOT NULL,
    calibration_version TEXT NOT NULL,
    posture_policy_version TEXT NOT NULL,
    action_playbook_version TEXT NOT NULL,
    point_in_time_mode TEXT NOT NULL,
    training_range_start TEXT,
    training_range_end TEXT,
    calibration_range_start TEXT,
    calibration_range_end TEXT,
    evaluation_range_start TEXT,
    evaluation_range_end TEXT,
    brier_score REAL,
    log_loss REAL,
    ece REAL,
    note TEXT NOT NULL DEFAULT '',
    created_at TEXT NOT NULL,
    activated_at TEXT,
    retired_at TEXT
);

CREATE TABLE IF NOT EXISTS analytics_active_model_pointers (
    market_scope TEXT PRIMARY KEY,
    release_id TEXT NOT NULL REFERENCES analytics_model_releases(release_id) ON DELETE CASCADE,
    updated_at TEXT NOT NULL,
    updated_by TEXT NOT NULL DEFAULT 'system'
);

CREATE TABLE IF NOT EXISTS analytics_prediction_snapshots (
    snapshot_id TEXT PRIMARY KEY,
    entity_id TEXT NOT NULL,
    market_scope TEXT NOT NULL,
    as_of_date TEXT NOT NULL,
    release_id TEXT,
    probability_mode TEXT NOT NULL,
    release_status TEXT NOT NULL,
    point_in_time_mode TEXT NOT NULL,
    overall_score REAL NOT NULL,
    external_shock_score REAL NOT NULL,
    raw_p_5d REAL NOT NULL,
    raw_p_20d REAL NOT NULL,
    raw_p_60d REAL NOT NULL,
    calibrated_p_5d REAL NOT NULL,
    calibrated_p_20d REAL NOT NULL,
    calibrated_p_60d REAL NOT NULL,
    posture TEXT NOT NULL,
    time_to_risk_bucket TEXT NOT NULL,
    feature_set_version TEXT NOT NULL,
    label_version TEXT NOT NULL,
    coverage_score REAL NOT NULL,
    freshness_status TEXT NOT NULL,
    method_version TEXT NOT NULL,
    posture_trigger_codes_json TEXT NOT NULL DEFAULT '[]',
    posture_blocker_codes_json TEXT NOT NULL DEFAULT '[]',
    recorded_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS analytics_feature_snapshots (
    snapshot_id TEXT PRIMARY KEY,
    entity_id TEXT NOT NULL,
    market_scope TEXT NOT NULL,
    as_of_date TEXT NOT NULL,
    feature_set_version TEXT NOT NULL,
    point_in_time_mode TEXT NOT NULL,
    visibility_status TEXT NOT NULL,
    latest_visible_at TEXT,
    coverage_score REAL NOT NULL,
    core_feature_coverage REAL NOT NULL,
    trigger_feature_coverage REAL NOT NULL,
    external_feature_coverage REAL NOT NULL,
    feature_count INTEGER NOT NULL,
    features_json TEXT NOT NULL,
    created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS analytics_formal_datasets (
    dataset_key TEXT PRIMARY KEY,
    dataset_id TEXT NOT NULL,
    dataset_version TEXT NOT NULL,
    market_scope TEXT NOT NULL,
    feature_set_version TEXT NOT NULL,
    label_version TEXT NOT NULL,
    scenario_set_version TEXT NOT NULL,
    point_in_time_mode TEXT NOT NULL,
    from_date TEXT,
    to_date TEXT,
    train_end_date TEXT,
    calibration_end_date TEXT,
    evaluation_start_date TEXT,
    row_count INTEGER NOT NULL,
    note TEXT NOT NULL DEFAULT '',
    manifest_json TEXT NOT NULL,
    created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS analytics_formal_dataset_rows (
    row_id TEXT PRIMARY KEY,
    dataset_key TEXT NOT NULL REFERENCES analytics_formal_datasets(dataset_key) ON DELETE CASCADE,
    split_name TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    market_scope TEXT NOT NULL,
    as_of_date TEXT NOT NULL,
    point_in_time_mode TEXT NOT NULL,
    latest_visible_at TEXT,
    coverage_score REAL NOT NULL,
    core_feature_coverage REAL NOT NULL,
    trigger_feature_coverage REAL NOT NULL,
    external_feature_coverage REAL NOT NULL,
    sample_quality_grade TEXT NOT NULL,
    primary_scenario_id TEXT,
    scenario_family TEXT,
    scenario_training_role TEXT,
    label_5d INTEGER NOT NULL,
    label_20d INTEGER NOT NULL,
    label_60d INTEGER NOT NULL,
    regime_5d TEXT NOT NULL DEFAULT 'normal',
    regime_20d TEXT NOT NULL DEFAULT 'normal',
    regime_60d TEXT NOT NULL DEFAULT 'normal',
    action_label_5d INTEGER NOT NULL DEFAULT 0,
    action_label_20d INTEGER NOT NULL DEFAULT 0,
    action_label_60d INTEGER NOT NULL DEFAULT 0,
    prepare_episode_label INTEGER NOT NULL DEFAULT 0,
    hedge_episode_label INTEGER NOT NULL DEFAULT 0,
    defend_episode_label INTEGER NOT NULL DEFAULT 0,
    primary_action_level TEXT,
    action_episode_id TEXT,
    action_episode_phase TEXT NOT NULL DEFAULT 'outside',
    protected_action_window INTEGER NOT NULL DEFAULT 0,
    features_json TEXT NOT NULL,
    created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS analytics_historical_replay_runs (
    replay_run_id TEXT PRIMARY KEY,
    release_id TEXT REFERENCES analytics_model_releases(release_id) ON DELETE SET NULL,
    market_scope TEXT NOT NULL,
    from_date TEXT NOT NULL,
    to_date TEXT NOT NULL,
    history_cache_key TEXT NOT NULL,
    feature_set_version TEXT NOT NULL,
    label_version TEXT NOT NULL,
    point_in_time_mode TEXT NOT NULL,
    runtime_policy_version TEXT NOT NULL,
    action_playbook_version TEXT NOT NULL,
    protected_window_catalog_id TEXT NOT NULL,
    source_watermark TEXT NOT NULL,
    status TEXT NOT NULL,
    point_count INTEGER NOT NULL,
    failure_reason TEXT,
    created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS analytics_historical_assessment_points (
    replay_point_id TEXT PRIMARY KEY,
    replay_run_id TEXT NOT NULL REFERENCES analytics_historical_replay_runs(replay_run_id) ON DELETE CASCADE,
    entity_id TEXT NOT NULL,
    market_scope TEXT NOT NULL,
    release_id TEXT REFERENCES analytics_model_releases(release_id) ON DELETE SET NULL,
    as_of_date TEXT NOT NULL,
    feature_snapshot_id TEXT,
    point_in_time_mode TEXT NOT NULL,
    runtime_policy_version TEXT NOT NULL,
    action_playbook_version TEXT NOT NULL,
    overall_score REAL NOT NULL,
    structural_score REAL NOT NULL,
    trigger_score REAL NOT NULL,
    external_shock_score REAL NOT NULL,
    raw_p_5d REAL NOT NULL,
    raw_p_20d REAL NOT NULL,
    raw_p_60d REAL NOT NULL,
    calibrated_p_5d REAL NOT NULL,
    calibrated_p_20d REAL NOT NULL,
    calibrated_p_60d REAL NOT NULL,
    posture TEXT NOT NULL,
    time_to_risk_bucket TEXT NOT NULL,
    actionability_prepare REAL NOT NULL,
    actionability_hedge REAL NOT NULL,
    actionability_defend REAL NOT NULL,
    probability_diagnostics_json TEXT NOT NULL DEFAULT '{"horizon_overlays":[]}',
    posture_trigger_codes_json TEXT NOT NULL DEFAULT '[]',
    posture_blocker_codes_json TEXT NOT NULL DEFAULT '[]',
    coverage_score REAL NOT NULL,
    freshness_status TEXT NOT NULL,
    generated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS alerts_events (
    alert_id TEXT PRIMARY KEY,
    event_type TEXT NOT NULL,
    scope TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    dimension TEXT,
    level TEXT NOT NULL,
    status TEXT NOT NULL,
    triggered_at TEXT NOT NULL,
    triggered_as_of_date TEXT NOT NULL,
    resolved_at TEXT,
    score REAL NOT NULL,
    previous_score REAL,
    trigger_reason TEXT NOT NULL,
    top_contributors_json TEXT NOT NULL DEFAULT '[]',
    related_indicators_json TEXT NOT NULL DEFAULT '[]',
    method_version TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
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

CREATE INDEX IF NOT EXISTS idx_analytics_backtest_runs_finished
    ON analytics_backtest_runs(finished_at DESC);

CREATE INDEX IF NOT EXISTS idx_analytics_model_releases_scope_status
    ON analytics_model_releases(market_scope, status, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_analytics_prediction_snapshots_scope_date
    ON analytics_prediction_snapshots(market_scope, as_of_date DESC);

CREATE INDEX IF NOT EXISTS idx_analytics_prediction_snapshots_release_date
    ON analytics_prediction_snapshots(release_id, as_of_date DESC);

CREATE INDEX IF NOT EXISTS idx_analytics_feature_snapshots_scope_version_date
    ON analytics_feature_snapshots(market_scope, feature_set_version, as_of_date DESC);

CREATE INDEX IF NOT EXISTS idx_analytics_formal_datasets_scope_version
    ON analytics_formal_datasets(market_scope, dataset_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_analytics_formal_dataset_rows_dataset_split_date
    ON analytics_formal_dataset_rows(dataset_key, split_name, as_of_date DESC);

CREATE INDEX IF NOT EXISTS idx_analytics_historical_replay_runs_scope_release_created
    ON analytics_historical_replay_runs(market_scope, release_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_analytics_historical_replay_runs_cache
    ON analytics_historical_replay_runs(history_cache_key, from_date, to_date, status);

CREATE INDEX IF NOT EXISTS idx_analytics_historical_assessment_points_run_date
    ON analytics_historical_assessment_points(replay_run_id, as_of_date DESC);

CREATE INDEX IF NOT EXISTS idx_analytics_historical_assessment_points_scope_release_date
    ON analytics_historical_assessment_points(market_scope, release_id, as_of_date DESC);

CREATE INDEX IF NOT EXISTS idx_alerts_events_status
    ON alerts_events(status, level, triggered_as_of_date DESC);

CREATE INDEX IF NOT EXISTS idx_alerts_events_scope_date
    ON alerts_events(scope, triggered_as_of_date DESC);
