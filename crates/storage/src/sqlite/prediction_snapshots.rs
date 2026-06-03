use chrono::NaiveDate;
use fc_domain::PredictionSnapshotRecord;

use crate::StorageError;

use super::{format_datetime, map_prediction_snapshot_row, prediction_snapshot_id, SqliteStore};

impl SqliteStore {
    pub async fn upsert_prediction_snapshots(
        &self,

        snapshots: &[PredictionSnapshotRecord],
    ) -> Result<(), StorageError> {
        let mut transaction = self.pool.begin().await?;

        for snapshot in snapshots {
            let posture_trigger_codes_json = serde_json::to_string(&snapshot.posture_trigger_codes)
                .map_err(|error| StorageError::Database(sqlx::Error::Decode(Box::new(error))))?;

            let posture_blocker_codes_json = serde_json::to_string(&snapshot.posture_blocker_codes)
                .map_err(|error| StorageError::Database(sqlx::Error::Decode(Box::new(error))))?;

            let snapshot_id = prediction_snapshot_id(
                &snapshot.entity_id,
                &snapshot.market_scope,
                snapshot.as_of_date,
                snapshot.release_id.as_deref(),
                &snapshot.point_in_time_mode,
            );

            sqlx::query(
                r#"

                INSERT INTO analytics_prediction_snapshots (

                    snapshot_id,

                    entity_id,

                    market_scope,

                    as_of_date,

                    release_id,

                    probability_mode,

                    release_status,

                    point_in_time_mode,

                    overall_score,

                    external_shock_score,

                    raw_p_5d,

                    raw_p_20d,

                    raw_p_60d,

                    calibrated_p_5d,

                    calibrated_p_20d,

                    calibrated_p_60d,

                    posture,

                    time_to_risk_bucket,

                    feature_set_version,

                    label_version,

                    coverage_score,

                    freshness_status,

                    method_version,

                    posture_trigger_codes_json,

                    posture_blocker_codes_json,

                    recorded_at

                )

                VALUES (

                    ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16,

                    ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26

                )

                ON CONFLICT(snapshot_id) DO UPDATE SET

                    release_id = excluded.release_id,

                    probability_mode = excluded.probability_mode,

                    release_status = excluded.release_status,

                    point_in_time_mode = excluded.point_in_time_mode,

                    overall_score = excluded.overall_score,

                    external_shock_score = excluded.external_shock_score,

                    raw_p_5d = excluded.raw_p_5d,

                    raw_p_20d = excluded.raw_p_20d,

                    raw_p_60d = excluded.raw_p_60d,

                    calibrated_p_5d = excluded.calibrated_p_5d,

                    calibrated_p_20d = excluded.calibrated_p_20d,

                    calibrated_p_60d = excluded.calibrated_p_60d,

                    posture = excluded.posture,

                    time_to_risk_bucket = excluded.time_to_risk_bucket,

                    feature_set_version = excluded.feature_set_version,

                    label_version = excluded.label_version,

                    coverage_score = excluded.coverage_score,

                    freshness_status = excluded.freshness_status,

                    method_version = excluded.method_version,

                    posture_trigger_codes_json = excluded.posture_trigger_codes_json,

                    posture_blocker_codes_json = excluded.posture_blocker_codes_json,

                    recorded_at = excluded.recorded_at

                "#,
            )
            .bind(snapshot_id)
            .bind(&snapshot.entity_id)
            .bind(&snapshot.market_scope)
            .bind(snapshot.as_of_date.to_string())
            .bind(snapshot.release_id.as_deref())
            .bind(&snapshot.probability_mode)
            .bind(&snapshot.release_status)
            .bind(&snapshot.point_in_time_mode)
            .bind(snapshot.overall_score)
            .bind(snapshot.external_shock_score)
            .bind(snapshot.raw_p_5d)
            .bind(snapshot.raw_p_20d)
            .bind(snapshot.raw_p_60d)
            .bind(snapshot.calibrated_p_5d)
            .bind(snapshot.calibrated_p_20d)
            .bind(snapshot.calibrated_p_60d)
            .bind(&snapshot.posture)
            .bind(&snapshot.time_to_risk_bucket)
            .bind(&snapshot.feature_set_version)
            .bind(&snapshot.label_version)
            .bind(snapshot.coverage_score)
            .bind(&snapshot.freshness_status)
            .bind(&snapshot.method_version)
            .bind(&posture_trigger_codes_json)
            .bind(&posture_blocker_codes_json)
            .bind(format_datetime(snapshot.recorded_at))
            .execute(&mut *transaction)
            .await?;
        }

        transaction.commit().await?;

        Ok(())
    }

    pub async fn list_prediction_snapshots(
        &self,

        market_scope: Option<&str>,

        release_id: Option<&str>,

        from: Option<NaiveDate>,

        to: Option<NaiveDate>,

        limit: Option<usize>,
    ) -> Result<Vec<PredictionSnapshotRecord>, StorageError> {
        let mut query = String::from(
            r#"

            SELECT

                entity_id,

                market_scope,

                as_of_date,

                release_id,

                probability_mode,

                release_status,

                point_in_time_mode,

                overall_score,

                external_shock_score,

                raw_p_5d,

                raw_p_20d,

                raw_p_60d,

                calibrated_p_5d,

                calibrated_p_20d,

                calibrated_p_60d,

                posture,

                time_to_risk_bucket,

                feature_set_version,

                label_version,

                coverage_score,

                freshness_status,

                method_version,

                posture_trigger_codes_json,

                posture_blocker_codes_json,

                recorded_at

            FROM analytics_prediction_snapshots

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
            query.push_str(&format!(" AND as_of_date >= ?{param_index}"));

            param_index += 1;
        }

        if to.is_some() {
            query.push_str(&format!(" AND as_of_date <= ?{param_index}"));

            param_index += 1;
        }

        query.push_str(" ORDER BY as_of_date DESC, recorded_at DESC");

        if limit.is_some() {
            query.push_str(&format!(" LIMIT ?{param_index}"));
        }

        let mut statement = sqlx::query(&query);

        if let Some(scope) = market_scope {
            statement = statement.bind(scope);
        }

        if let Some(release_id) = release_id {
            statement = statement.bind(release_id);
        }

        if let Some(start_date) = from {
            statement = statement.bind(start_date.to_string());
        }

        if let Some(end_date) = to {
            statement = statement.bind(end_date.to_string());
        }

        if let Some(limit) = limit {
            statement = statement.bind(limit as i64);
        }

        let rows = statement.fetch_all(&self.pool).await?;

        rows.into_iter().map(map_prediction_snapshot_row).collect()
    }
}
