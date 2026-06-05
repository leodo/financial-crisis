use chrono::NaiveDate;
use fc_domain::HistoricalReplayRunRecord;

use crate::sqlite::{format_datetime, map_historical_replay_run_row, SqliteStore};
use crate::StorageError;

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
}
