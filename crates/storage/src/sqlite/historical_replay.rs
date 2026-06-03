use chrono::NaiveDate;
use fc_domain::{HistoricalAssessmentPointRecord, HistoricalReplayRunRecord};

use crate::StorageError;

use super::{
    format_datetime, historical_assessment_point_id, map_historical_assessment_point_row,
    map_historical_replay_run_row, SqliteStore,
};

impl SqliteStore {
    pub async fn upsert_historical_replay_run(
        &self,
        run: &HistoricalReplayRunRecord,
    ) -> Result<(), StorageError> {
        sqlx::query(
            r#"
            INSERT INTO analytics_historical_replay_runs (
                replay_run_id,
                release_id,
                market_scope,
                from_date,
                to_date,
                history_cache_key,
                feature_set_version,
                label_version,
                point_in_time_mode,
                runtime_policy_version,
                action_playbook_version,
                protected_window_catalog_id,
                source_watermark,
                status,
                point_count,
                failure_reason,
                created_at
            )
            VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17
            )
            ON CONFLICT(replay_run_id) DO UPDATE SET
                release_id = excluded.release_id,
                market_scope = excluded.market_scope,
                from_date = excluded.from_date,
                to_date = excluded.to_date,
                history_cache_key = excluded.history_cache_key,
                feature_set_version = excluded.feature_set_version,
                label_version = excluded.label_version,
                point_in_time_mode = excluded.point_in_time_mode,
                runtime_policy_version = excluded.runtime_policy_version,
                action_playbook_version = excluded.action_playbook_version,
                protected_window_catalog_id = excluded.protected_window_catalog_id,
                source_watermark = excluded.source_watermark,
                status = excluded.status,
                point_count = excluded.point_count,
                failure_reason = excluded.failure_reason,
                created_at = excluded.created_at
            "#,
        )
        .bind(&run.replay_run_id)
        .bind(run.release_id.as_deref())
        .bind(&run.market_scope)
        .bind(run.from_date.to_string())
        .bind(run.to_date.to_string())
        .bind(&run.history_cache_key)
        .bind(&run.feature_set_version)
        .bind(&run.label_version)
        .bind(&run.point_in_time_mode)
        .bind(&run.runtime_policy_version)
        .bind(&run.action_playbook_version)
        .bind(&run.protected_window_catalog_id)
        .bind(&run.source_watermark)
        .bind(&run.status)
        .bind(run.point_count as i64)
        .bind(run.failure_reason.as_deref())
        .bind(format_datetime(run.created_at))
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn load_latest_historical_replay_run(
        &self,
        market_scope: &str,
        release_id: Option<&str>,
        history_cache_key: &str,
        from_date: NaiveDate,
        to_date: NaiveDate,
    ) -> Result<Option<HistoricalReplayRunRecord>, StorageError> {
        let mut query = String::from(
            r#"
            SELECT
                replay_run_id,
                release_id,
                market_scope,
                from_date,
                to_date,
                history_cache_key,
                feature_set_version,
                label_version,
                point_in_time_mode,
                runtime_policy_version,
                action_playbook_version,
                protected_window_catalog_id,
                source_watermark,
                status,
                point_count,
                failure_reason,
                created_at
            FROM analytics_historical_replay_runs
            WHERE market_scope = ?1
              AND history_cache_key = ?2
              AND from_date = ?3
              AND to_date = ?4
              AND status = 'success'
            "#,
        );
        if release_id.is_some() {
            query.push_str(" AND release_id = ?5");
        } else {
            query.push_str(" AND release_id IS NULL");
        }
        query.push_str(" ORDER BY created_at DESC LIMIT 1");

        let mut statement = sqlx::query(&query)
            .bind(market_scope)
            .bind(history_cache_key)
            .bind(from_date.to_string())
            .bind(to_date.to_string());
        if let Some(release_id) = release_id {
            statement = statement.bind(release_id);
        }

        let row = statement.fetch_optional(&self.pool).await?;
        row.map(map_historical_replay_run_row).transpose()
    }

    pub async fn list_historical_replay_runs(
        &self,
        market_scope: Option<&str>,
        release_id: Option<&str>,
        from: Option<NaiveDate>,
        to: Option<NaiveDate>,
        limit: Option<usize>,
    ) -> Result<Vec<HistoricalReplayRunRecord>, StorageError> {
        let mut query = String::from(
            r#"
            SELECT
                replay_run_id,
                release_id,
                market_scope,
                from_date,
                to_date,
                history_cache_key,
                feature_set_version,
                label_version,
                point_in_time_mode,
                runtime_policy_version,
                action_playbook_version,
                protected_window_catalog_id,
                source_watermark,
                status,
                point_count,
                failure_reason,
                created_at
            FROM analytics_historical_replay_runs
            WHERE 1 = 1
            "#,
        );
        let mut param_index = 1;
        if market_scope.is_some() {
            query.push_str(&format!(" AND market_scope = ?{param_index}"));
            param_index += 1;
        }
        if release_id.is_some() {
            query.push_str(&format!(" AND release_id = ?{param_index}"));
            param_index += 1;
        }
        if from.is_some() {
            query.push_str(&format!(" AND to_date >= ?{param_index}"));
            param_index += 1;
        }
        if to.is_some() {
            query.push_str(&format!(" AND from_date <= ?{param_index}"));
            param_index += 1;
        }
        query.push_str(" ORDER BY created_at DESC");
        if limit.is_some() {
            query.push_str(&format!(" LIMIT ?{param_index}"));
        }

        let mut statement = sqlx::query(&query);
        if let Some(market_scope) = market_scope {
            statement = statement.bind(market_scope);
        }
        if let Some(release_id) = release_id {
            statement = statement.bind(release_id);
        }
        if let Some(from) = from {
            statement = statement.bind(from.to_string());
        }
        if let Some(to) = to {
            statement = statement.bind(to.to_string());
        }
        if let Some(limit) = limit {
            statement = statement.bind(limit as i64);
        }

        let rows = statement.fetch_all(&self.pool).await?;
        rows.into_iter()
            .map(map_historical_replay_run_row)
            .collect()
    }

    pub async fn replace_historical_assessment_points(
        &self,
        replay_run_id: &str,
        points: &[HistoricalAssessmentPointRecord],
    ) -> Result<(), StorageError> {
        let mut transaction = self.pool.begin().await?;
        sqlx::query(
            r#"
            DELETE FROM analytics_historical_assessment_points
            WHERE replay_run_id = ?1
            "#,
        )
        .bind(replay_run_id)
        .execute(&mut *transaction)
        .await?;

        for point in points {
            let replay_point_id = historical_assessment_point_id(
                &point.replay_run_id,
                &point.entity_id,
                point.as_of_date,
            );
            let posture_trigger_codes_json = serde_json::to_string(&point.posture_trigger_codes)
                .map_err(|error| StorageError::Database(sqlx::Error::Decode(Box::new(error))))?;
            let posture_blocker_codes_json = serde_json::to_string(&point.posture_blocker_codes)
                .map_err(|error| StorageError::Database(sqlx::Error::Decode(Box::new(error))))?;
            sqlx::query(
                r#"
                INSERT INTO analytics_historical_assessment_points (
                    replay_point_id,
                    replay_run_id,
                    entity_id,
                    market_scope,
                    release_id,
                    as_of_date,
                    feature_snapshot_id,
                    point_in_time_mode,
                    runtime_policy_version,
                    action_playbook_version,
                    overall_score,
                    structural_score,
                    trigger_score,
                    external_shock_score,
                    raw_p_5d,
                    raw_p_20d,
                    raw_p_60d,
                    calibrated_p_5d,
                    calibrated_p_20d,
                    calibrated_p_60d,
                    posture,
                    time_to_risk_bucket,
                    actionability_prepare,
                    actionability_hedge,
                    actionability_defend,
                    posture_trigger_codes_json,
                    posture_blocker_codes_json,
                    coverage_score,
                    freshness_status,
                    generated_at
                )
                VALUES (
                    ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16,
                    ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26, ?27, ?28, ?29, ?30
                )
                "#,
            )
            .bind(replay_point_id)
            .bind(&point.replay_run_id)
            .bind(&point.entity_id)
            .bind(&point.market_scope)
            .bind(point.release_id.as_deref())
            .bind(point.as_of_date.to_string())
            .bind(point.feature_snapshot_id.as_deref())
            .bind(&point.point_in_time_mode)
            .bind(&point.runtime_policy_version)
            .bind(&point.action_playbook_version)
            .bind(point.overall_score)
            .bind(point.structural_score)
            .bind(point.trigger_score)
            .bind(point.external_shock_score)
            .bind(point.raw_p_5d)
            .bind(point.raw_p_20d)
            .bind(point.raw_p_60d)
            .bind(point.calibrated_p_5d)
            .bind(point.calibrated_p_20d)
            .bind(point.calibrated_p_60d)
            .bind(&point.posture)
            .bind(&point.time_to_risk_bucket)
            .bind(point.actionability_prepare)
            .bind(point.actionability_hedge)
            .bind(point.actionability_defend)
            .bind(&posture_trigger_codes_json)
            .bind(&posture_blocker_codes_json)
            .bind(point.coverage_score)
            .bind(&point.freshness_status)
            .bind(format_datetime(point.generated_at))
            .execute(&mut *transaction)
            .await?;
        }

        transaction.commit().await?;
        Ok(())
    }

    pub async fn list_historical_assessment_points(
        &self,
        replay_run_id: Option<&str>,
        market_scope: Option<&str>,
        release_id: Option<&str>,
        from: Option<NaiveDate>,
        to: Option<NaiveDate>,
        limit: Option<usize>,
    ) -> Result<Vec<HistoricalAssessmentPointRecord>, StorageError> {
        let mut query = String::from(
            r#"
            SELECT
                replay_run_id,
                entity_id,
                market_scope,
                release_id,
                as_of_date,
                feature_snapshot_id,
                point_in_time_mode,
                runtime_policy_version,
                action_playbook_version,
                overall_score,
                structural_score,
                trigger_score,
                external_shock_score,
                raw_p_5d,
                raw_p_20d,
                raw_p_60d,
                calibrated_p_5d,
                calibrated_p_20d,
                calibrated_p_60d,
                posture,
                time_to_risk_bucket,
                actionability_prepare,
                actionability_hedge,
                actionability_defend,
                posture_trigger_codes_json,
                posture_blocker_codes_json,
                coverage_score,
                freshness_status,
                generated_at
            FROM analytics_historical_assessment_points
            WHERE 1 = 1
            "#,
        );
        let mut param_index = 1;
        if replay_run_id.is_some() {
            query.push_str(&format!(" AND replay_run_id = ?{param_index}"));
            param_index += 1;
        }
        if market_scope.is_some() {
            query.push_str(&format!(" AND market_scope = ?{param_index}"));
            param_index += 1;
        }
        if release_id.is_some() {
            query.push_str(&format!(" AND release_id = ?{param_index}"));
            param_index += 1;
        }
        if from.is_some() {
            query.push_str(&format!(" AND as_of_date >= ?{param_index}"));
            param_index += 1;
        }
        if to.is_some() {
            query.push_str(&format!(" AND as_of_date <= ?{param_index}"));
            param_index += 1;
        }
        query.push_str(" ORDER BY as_of_date ASC, generated_at DESC");
        if limit.is_some() {
            query.push_str(&format!(" LIMIT ?{param_index}"));
        }

        let mut statement = sqlx::query(&query);
        if let Some(replay_run_id) = replay_run_id {
            statement = statement.bind(replay_run_id);
        }
        if let Some(market_scope) = market_scope {
            statement = statement.bind(market_scope);
        }
        if let Some(release_id) = release_id {
            statement = statement.bind(release_id);
        }
        if let Some(from) = from {
            statement = statement.bind(from.to_string());
        }
        if let Some(to) = to {
            statement = statement.bind(to.to_string());
        }
        if let Some(limit) = limit {
            statement = statement.bind(limit as i64);
        }

        let rows = statement.fetch_all(&self.pool).await?;
        rows.into_iter()
            .map(map_historical_assessment_point_row)
            .collect()
    }
}
