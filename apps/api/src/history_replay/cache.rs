use std::collections::BTreeSet;

use chrono::NaiveDate;
use fc_domain::{HistoricalReplayRunRecord, Observation};
use fc_storage::SqliteStore;
use uuid::Uuid;

use crate::{
    assessment::{history_runtime_policy_version, ServingModelContext},
    demo::{FORMAL_MAIN_FEATURE_SET_VERSION, FORMAL_MAIN_LABEL_VERSION},
};

use super::{
    transform::historical_output_from_replay_points, HistoricalAssessmentOutput,
    HistoricalReplayPointDraft,
};

const PREDICTION_SNAPSHOT_CACHE_VERSION: &str = "history_cache_v4_20260606";

pub(crate) async fn persist_historical_replay_output(
    store: &SqliteStore,
    observations: &[Observation],
    serving_model: Option<&ServingModelContext>,
    output: &HistoricalAssessmentOutput,
) -> anyhow::Result<()> {
    let Some(first_point) = output.replay_point_drafts.first() else {
        return Ok(());
    };
    let Some(last_point) = output.replay_point_drafts.last() else {
        return Ok(());
    };

    let replay_run_id = Uuid::new_v4().to_string();
    let protected_stress_window_catalog = fc_domain::load_protected_stress_window_catalog();
    let created_at = output
        .replay_point_drafts
        .last()
        .map(|point| point.generated_at)
        .unwrap_or_else(chrono::Utc::now);
    let run = HistoricalReplayRunRecord {
        replay_run_id: replay_run_id.clone(),
        release_id: first_point.release_id.clone(),
        market_scope: first_point.market_scope.clone(),
        from_date: first_point.as_of_date,
        to_date: last_point.as_of_date,
        history_cache_key: expected_prediction_snapshot_method_version(serving_model),
        feature_set_version: first_point.feature_set_version.clone(),
        label_version: first_point.label_version.clone(),
        point_in_time_mode: first_point.point_in_time_mode.clone(),
        runtime_policy_version: first_point.runtime_policy_version.clone(),
        action_playbook_version: first_point.action_playbook_version.clone(),
        protected_window_catalog_id: protected_stress_window_catalog.catalog_id,
        source_watermark: historical_replay_source_watermark(observations),
        status: "success".to_string(),
        point_count: output.replay_point_drafts.len(),
        failure_reason: None,
        created_at,
    };
    let points = output
        .replay_point_drafts
        .iter()
        .cloned()
        .map(|point| historical_assessment_point_record(replay_run_id.clone(), point))
        .collect::<Vec<_>>();

    store.upsert_historical_replay_run(&run).await?;
    store
        .replace_historical_assessment_points(&replay_run_id, &points)
        .await?;
    Ok(())
}

fn historical_assessment_point_record(
    replay_run_id: String,
    point: HistoricalReplayPointDraft,
) -> fc_domain::HistoricalAssessmentPointRecord {
    fc_domain::HistoricalAssessmentPointRecord {
        replay_run_id,
        entity_id: point.entity_id,
        market_scope: point.market_scope,
        release_id: point.release_id,
        as_of_date: point.as_of_date,
        feature_snapshot_id: point.feature_snapshot_id,
        point_in_time_mode: point.point_in_time_mode,
        runtime_policy_version: point.runtime_policy_version,
        action_playbook_version: point.action_playbook_version,
        overall_score: point.overall_score,
        structural_score: point.structural_score,
        trigger_score: point.trigger_score,
        external_shock_score: point.external_shock_score,
        raw_p_5d: point.raw_p_5d,
        raw_p_20d: point.raw_p_20d,
        raw_p_60d: point.raw_p_60d,
        calibrated_p_5d: point.calibrated_p_5d,
        calibrated_p_20d: point.calibrated_p_20d,
        calibrated_p_60d: point.calibrated_p_60d,
        posture: point.posture,
        time_to_risk_bucket: point.time_to_risk_bucket,
        actionability_prepare: point.actionability_prepare,
        actionability_hedge: point.actionability_hedge,
        actionability_defend: point.actionability_defend,
        probability_diagnostics: point.probability_diagnostics,
        posture_trigger_codes: point.posture_trigger_codes,
        posture_blocker_codes: point.posture_blocker_codes,
        coverage_score: point.coverage_score,
        freshness_status: point.freshness_status,
        generated_at: point.generated_at,
    }
}

pub(crate) async fn load_cached_historical_replay_output(
    store: &SqliteStore,
    serving_model: Option<&ServingModelContext>,
    observations: &[Observation],
    target_dates: &BTreeSet<NaiveDate>,
) -> anyhow::Result<Option<HistoricalAssessmentOutput>> {
    let Some(from_date) = target_dates.first().copied() else {
        return Ok(None);
    };
    let Some(to_date) = target_dates.last().copied() else {
        return Ok(None);
    };
    let release_filter = serving_model.map(|context| context.release.manifest.release_id.as_str());
    let history_cache_key = expected_prediction_snapshot_method_version(serving_model);
    let Some(run) = store
        .load_latest_historical_replay_run(
            "financial_system",
            release_filter,
            &history_cache_key,
            from_date,
            to_date,
        )
        .await?
    else {
        return Ok(None);
    };

    let expected_source_watermark = historical_replay_source_watermark(observations);
    if !historical_replay_run_has_expected_source_watermark(&run, &expected_source_watermark) {
        tracing::warn!(
            replay_run_id = run.replay_run_id,
            cached_source_watermark = %run.source_watermark,
            expected_source_watermark = %expected_source_watermark,
            "cached historical replay run source watermark is stale; skipping replay cache reuse"
        );
        return Ok(None);
    }

    let points = store
        .list_historical_assessment_points(
            Some(&run.replay_run_id),
            Some("financial_system"),
            release_filter,
            Some(from_date),
            Some(to_date),
            None,
        )
        .await?;
    let available_dates = points
        .iter()
        .map(|point| point.as_of_date)
        .collect::<BTreeSet<_>>();
    if available_dates != *target_dates {
        tracing::warn!(
            replay_run_id = run.replay_run_id,
            expected_dates = target_dates.len(),
            available_dates = available_dates.len(),
            "cached historical replay run does not fully cover target dates; skipping replay cache reuse"
        );
        return Ok(None);
    }

    Ok(Some(historical_output_from_replay_points(points)))
}

pub(crate) fn historical_replay_source_watermark(observations: &[Observation]) -> String {
    observations
        .iter()
        .filter(|observation| observation.entity_id == "us")
        .map(|observation| observation.as_of_date)
        .max()
        .map(|date| format!("us_observations={date}"))
        .unwrap_or_else(|| "us_observations=missing".to_string())
}

fn historical_replay_run_has_expected_source_watermark(
    run: &HistoricalReplayRunRecord,
    expected_source_watermark: &str,
) -> bool {
    run.source_watermark == expected_source_watermark
}

pub(crate) fn expected_prediction_snapshot_method_version(
    serving_model: Option<&ServingModelContext>,
) -> String {
    let Some(serving_model) = serving_model else {
        return history_cache_key(
            None,
            "heuristic_mvp",
            "feature_v2_20260531",
            "label_v1_20260530",
            "prob_v1_20260531",
            "calib_v1_20260531",
            "posture_v1_20260530",
            "action_playbook_v1_20260531",
            "best_effort",
            &history_runtime_policy_version(None),
        );
    };

    history_cache_key(
        Some(serving_model.release.manifest.release_id.as_str()),
        &serving_model.runtime_probability_mode,
        &serving_model.release.manifest.feature_set_version,
        &serving_model.release.manifest.label_version,
        &serving_model.release.manifest.prob_model_version,
        &serving_model.release.manifest.calibration_version,
        &serving_model.release.manifest.posture_policy_version,
        &serving_model.release.manifest.action_playbook_version,
        &serving_model.release.manifest.point_in_time_mode,
        &history_runtime_policy_version(Some(serving_model)),
    )
}

#[allow(clippy::too_many_arguments)]
fn history_cache_key(
    release_id: Option<&str>,
    probability_mode: &str,
    feature_set_version: &str,
    label_version: &str,
    prob_model_version: &str,
    calibration_version: &str,
    posture_policy_version: &str,
    action_playbook_version: &str,
    point_in_time_mode: &str,
    runtime_history_policy_version: &str,
) -> String {
    format!(
        "{PREDICTION_SNAPSHOT_CACHE_VERSION}|release={}|probability_mode={probability_mode}|feature={feature_set_version}|label={label_version}|prob={prob_model_version}|calib={calibration_version}|posture={posture_policy_version}|action={action_playbook_version}|pit={point_in_time_mode}|runtime_policy={runtime_history_policy_version}",
        release_id.unwrap_or("heuristic")
    )
}

#[cfg(test)]
pub(crate) fn should_refresh_full_release_history(
    serving_model: Option<&ServingModelContext>,
    persisted_rows: &[fc_domain::PredictionSnapshotRecord],
    has_missing_dates: bool,
) -> bool {
    if !uses_bundle_backed_history(serving_model) {
        return false;
    }

    if persisted_rows.is_empty() || has_missing_dates {
        return true;
    }

    let expected_method_version = expected_prediction_snapshot_method_version(serving_model);
    persisted_rows
        .iter()
        .any(|snapshot| snapshot.method_version != expected_method_version)
}

#[cfg(test)]
fn uses_bundle_backed_history(serving_model: Option<&ServingModelContext>) -> bool {
    serving_model.is_some_and(|context| context.probability_bundle.is_some())
}

pub(crate) fn is_formal_main_release(serving_model: Option<&ServingModelContext>) -> bool {
    serving_model.is_some_and(|context| {
        context.release.manifest.feature_set_version == FORMAL_MAIN_FEATURE_SET_VERSION
            && context.release.manifest.label_version == FORMAL_MAIN_LABEL_VERSION
            && context.probability_bundle.is_some()
    })
}

#[cfg(test)]
mod tests {
    use chrono::{NaiveDate, TimeZone, Utc};
    use fc_domain::{HistoricalReplayRunRecord, Observation};

    use super::{
        historical_replay_run_has_expected_source_watermark, historical_replay_source_watermark,
    };

    fn observation(entity_id: &str, as_of_date: NaiveDate) -> Observation {
        Observation {
            indicator_id: "indicator".to_string(),
            entity_id: entity_id.to_string(),
            as_of_date,
            period_start: None,
            period_end: None,
            frequency: fc_domain::Frequency::Daily,
            value: 1.0,
            unit: "unit".to_string(),
            source_id: "source".to_string(),
            dataset_id: "dataset".to_string(),
            revision_time: None,
            publication_time: None,
            quality_score: 100.0,
            quality_flags: Vec::new(),
        }
    }

    fn replay_run(source_watermark: &str) -> HistoricalReplayRunRecord {
        HistoricalReplayRunRecord {
            replay_run_id: "run".to_string(),
            release_id: Some("release".to_string()),
            market_scope: "financial_system".to_string(),
            from_date: NaiveDate::from_ymd_opt(2026, 5, 1).unwrap(),
            to_date: NaiveDate::from_ymd_opt(2026, 5, 2).unwrap(),
            history_cache_key: "cache".to_string(),
            feature_set_version: "feature".to_string(),
            label_version: "label".to_string(),
            point_in_time_mode: "best_effort".to_string(),
            runtime_policy_version: "runtime".to_string(),
            action_playbook_version: "action".to_string(),
            protected_window_catalog_id: "catalog".to_string(),
            source_watermark: source_watermark.to_string(),
            status: "success".to_string(),
            point_count: 2,
            failure_reason: None,
            created_at: Utc.with_ymd_and_hms(2026, 6, 4, 8, 0, 0).single().unwrap(),
        }
    }

    #[test]
    fn historical_replay_source_watermark_uses_latest_us_observation_date() {
        let observations = vec![
            observation("jp", NaiveDate::from_ymd_opt(2026, 5, 3).unwrap()),
            observation("us", NaiveDate::from_ymd_opt(2026, 5, 1).unwrap()),
            observation("us", NaiveDate::from_ymd_opt(2026, 5, 2).unwrap()),
        ];

        assert_eq!(
            historical_replay_source_watermark(&observations),
            "us_observations=2026-05-02"
        );
    }

    #[test]
    fn historical_replay_run_reuse_requires_matching_source_watermark() {
        let expected = "us_observations=2026-05-02";

        assert!(historical_replay_run_has_expected_source_watermark(
            &replay_run(expected),
            expected
        ));
        assert!(!historical_replay_run_has_expected_source_watermark(
            &replay_run("us_observations=2026-05-01"),
            expected
        ));
    }
}
