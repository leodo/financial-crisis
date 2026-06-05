use fc_domain::{
    ActiveModelPointer, FeatureSnapshotRecord, FormalDatasetManifest, FormalDatasetRecord,
    FormalDatasetRowRecord, HistoricalAssessmentPointRecord, HistoricalReplayRunRecord,
    ModelReleaseManifest, ModelReleaseRecord, PredictionSnapshotRecord,
};
use sqlx::{sqlite::SqliteRow, Row};

use crate::StorageError;

use super::{parse_date, parse_optional_date, parse_optional_datetime, parse_required_datetime};

pub(super) fn map_model_release_row(row: SqliteRow) -> Result<ModelReleaseRecord, StorageError> {
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

pub(super) fn map_active_pointer_row(row: SqliteRow) -> Result<ActiveModelPointer, StorageError> {
    Ok(ActiveModelPointer {
        market_scope: row.try_get("market_scope")?,
        release_id: row.try_get("release_id")?,
        updated_at: parse_required_datetime(row.try_get::<String, _>("updated_at")?.as_str())?,
        updated_by: row.try_get("updated_by")?,
    })
}

pub(super) fn map_prediction_snapshot_row(
    row: SqliteRow,
) -> Result<PredictionSnapshotRecord, StorageError> {
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

pub(super) fn map_feature_snapshot_row(
    row: SqliteRow,
) -> Result<FeatureSnapshotRecord, StorageError> {
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

pub(super) fn map_formal_dataset_row(row: SqliteRow) -> Result<FormalDatasetRecord, StorageError> {
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

pub(super) fn map_formal_dataset_row_record(
    row: SqliteRow,
) -> Result<FormalDatasetRowRecord, StorageError> {
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

pub(super) fn map_historical_replay_run_row(
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

pub(super) fn map_historical_assessment_point_row(
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
    let probability_diagnostics = serde_json::from_str(
        row.try_get::<String, _>("probability_diagnostics_json")?
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
        probability_diagnostics,
        posture_trigger_codes,
        posture_blocker_codes,
        coverage_score: row.try_get("coverage_score")?,
        freshness_status: row.try_get("freshness_status")?,
        generated_at: parse_required_datetime(row.try_get::<String, _>("generated_at")?.as_str())?,
    })
}
