use chrono::NaiveDate;
use fc_domain::FeatureSnapshotRecord;

use crate::StorageError;

use super::{feature_snapshot_id, format_datetime, map_feature_snapshot_row, SqliteStore};

impl SqliteStore {
    pub async fn upsert_feature_snapshots(
        &self,

        snapshots: &[FeatureSnapshotRecord],
    ) -> Result<(), StorageError> {
        let mut transaction = self.pool.begin().await?;

        for snapshot in snapshots {
            let snapshot_id = feature_snapshot_id(
                &snapshot.entity_id,
                &snapshot.market_scope,
                snapshot.as_of_date,
                &snapshot.feature_set_version,
                &snapshot.point_in_time_mode,
            );

            let features_json = serde_json::to_string(&snapshot.features)
                .map_err(|error| StorageError::Database(sqlx::Error::Decode(Box::new(error))))?;

            sqlx::query(
                r#"

                INSERT INTO analytics_feature_snapshots (

                    snapshot_id,

                    entity_id,

                    market_scope,

                    as_of_date,

                    feature_set_version,

                    point_in_time_mode,

                    visibility_status,

                    latest_visible_at,

                    coverage_score,

                    core_feature_coverage,

                    trigger_feature_coverage,

                    external_feature_coverage,

                    feature_count,

                    features_json,

                    created_at

                )

                VALUES (

                    ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15

                )

                ON CONFLICT(snapshot_id) DO UPDATE SET

                    visibility_status = excluded.visibility_status,

                    latest_visible_at = excluded.latest_visible_at,

                    coverage_score = excluded.coverage_score,

                    core_feature_coverage = excluded.core_feature_coverage,

                    trigger_feature_coverage = excluded.trigger_feature_coverage,

                    external_feature_coverage = excluded.external_feature_coverage,

                    feature_count = excluded.feature_count,

                    features_json = excluded.features_json,

                    created_at = excluded.created_at

                "#,
            )
            .bind(snapshot_id)
            .bind(&snapshot.entity_id)
            .bind(&snapshot.market_scope)
            .bind(snapshot.as_of_date.to_string())
            .bind(&snapshot.feature_set_version)
            .bind(&snapshot.point_in_time_mode)
            .bind(&snapshot.visibility_status)
            .bind(snapshot.latest_visible_at.map(format_datetime))
            .bind(snapshot.coverage_score)
            .bind(snapshot.core_feature_coverage)
            .bind(snapshot.trigger_feature_coverage)
            .bind(snapshot.external_feature_coverage)
            .bind(snapshot.feature_count as i64)
            .bind(features_json)
            .bind(format_datetime(snapshot.created_at))
            .execute(&mut *transaction)
            .await?;
        }

        transaction.commit().await?;

        Ok(())
    }

    pub async fn list_feature_snapshots(
        &self,

        market_scope: Option<&str>,

        feature_set_version: Option<&str>,

        from: Option<NaiveDate>,

        to: Option<NaiveDate>,

        limit: Option<usize>,
    ) -> Result<Vec<FeatureSnapshotRecord>, StorageError> {
        let mut query = String::from(
            r#"

            SELECT

                entity_id,

                market_scope,

                as_of_date,

                feature_set_version,

                point_in_time_mode,

                visibility_status,

                latest_visible_at,

                coverage_score,

                core_feature_coverage,

                trigger_feature_coverage,

                external_feature_coverage,

                feature_count,

                features_json,

                created_at

            FROM analytics_feature_snapshots

            WHERE 1 = 1

            "#,
        );

        let mut param_index = 1;

        if market_scope.is_some() {
            query.push_str(&format!(" AND market_scope = ?{param_index}"));

            param_index += 1;
        }

        if feature_set_version.is_some() {
            query.push_str(&format!(" AND feature_set_version = ?{param_index}"));

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

        query.push_str(" ORDER BY as_of_date DESC, created_at DESC");

        if limit.is_some() {
            query.push_str(&format!(" LIMIT ?{param_index}"));
        }

        let mut statement = sqlx::query(&query);

        if let Some(scope) = market_scope {
            statement = statement.bind(scope);
        }

        if let Some(version) = feature_set_version {
            statement = statement.bind(version);
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

        rows.into_iter().map(map_feature_snapshot_row).collect()
    }

    pub async fn list_feature_snapshots_for_mode(
        &self,

        market_scope: &str,

        feature_set_version: &str,

        point_in_time_mode: &str,

        from: Option<NaiveDate>,

        to: Option<NaiveDate>,
    ) -> Result<Vec<FeatureSnapshotRecord>, StorageError> {
        let mut query = String::from(
            r#"

            SELECT

                entity_id,

                market_scope,

                as_of_date,

                feature_set_version,

                point_in_time_mode,

                visibility_status,

                latest_visible_at,

                coverage_score,

                core_feature_coverage,

                trigger_feature_coverage,

                external_feature_coverage,

                feature_count,

                features_json,

                created_at

            FROM analytics_feature_snapshots

            WHERE market_scope = ?1

              AND feature_set_version = ?2

              AND point_in_time_mode = ?3

            "#,
        );

        let mut param_index = 4;

        if from.is_some() {
            query.push_str(&format!(" AND as_of_date >= ?{param_index}"));

            param_index += 1;
        }

        if to.is_some() {
            query.push_str(&format!(" AND as_of_date <= ?{param_index}"));
        }

        query.push_str(" ORDER BY as_of_date ASC, created_at DESC");

        let mut statement = sqlx::query(&query)
            .bind(market_scope)
            .bind(feature_set_version)
            .bind(point_in_time_mode);

        if let Some(start_date) = from {
            statement = statement.bind(start_date.to_string());
        }

        if let Some(end_date) = to {
            statement = statement.bind(end_date.to_string());
        }

        let rows = statement.fetch_all(&self.pool).await?;

        rows.into_iter().map(map_feature_snapshot_row).collect()
    }
}
