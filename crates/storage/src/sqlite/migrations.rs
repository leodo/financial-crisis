use std::collections::HashSet;

use sqlx::Row;

use crate::StorageError;

use super::{SqliteStore, SQLITE_INIT_SQL};

impl SqliteStore {
    pub async fn migrate(&self) -> Result<(), StorageError> {
        for statement in SQLITE_INIT_SQL.split(';') {
            let statement = statement.trim();
            if !statement.is_empty() {
                sqlx::query(statement).execute(&self.pool).await?;
            }
        }
        self.ensure_prediction_snapshot_clause_columns().await?;
        self.ensure_formal_dataset_regime_columns().await?;
        self.ensure_formal_dataset_action_label_columns().await?;
        self.ensure_formal_dataset_action_episode_columns().await?;
        self.ensure_historical_replay_probability_diagnostics_column()
            .await?;
        Ok(())
    }

    async fn ensure_prediction_snapshot_clause_columns(&self) -> Result<(), StorageError> {
        let columns = sqlx::query("PRAGMA table_info(analytics_prediction_snapshots)")
            .fetch_all(&self.pool)
            .await?;
        let column_names = columns
            .into_iter()
            .map(|row| row.try_get::<String, _>("name"))
            .collect::<Result<HashSet<_>, _>>()?;

        for (column_name, alter_sql) in [
            (
                "posture_trigger_codes_json",
                "ALTER TABLE analytics_prediction_snapshots ADD COLUMN posture_trigger_codes_json TEXT NOT NULL DEFAULT '[]'",
            ),
            (
                "posture_blocker_codes_json",
                "ALTER TABLE analytics_prediction_snapshots ADD COLUMN posture_blocker_codes_json TEXT NOT NULL DEFAULT '[]'",
            ),
        ] {
            if !column_names.contains(column_name) {
                sqlx::query(alter_sql).execute(&self.pool).await?;
            }
        }

        Ok(())
    }

    async fn ensure_historical_replay_probability_diagnostics_column(
        &self,
    ) -> Result<(), StorageError> {
        let columns = sqlx::query("PRAGMA table_info(analytics_historical_assessment_points)")
            .fetch_all(&self.pool)
            .await?;
        let column_names = columns
            .into_iter()
            .map(|row| row.try_get::<String, _>("name"))
            .collect::<Result<HashSet<_>, _>>()?;
        if !column_names.contains("probability_diagnostics_json") {
            sqlx::query(
                "ALTER TABLE analytics_historical_assessment_points ADD COLUMN probability_diagnostics_json TEXT NOT NULL DEFAULT '{\"horizon_overlays\":[]}'",
            )
            .execute(&self.pool)
            .await?;
        }
        Ok(())
    }

    async fn ensure_formal_dataset_regime_columns(&self) -> Result<(), StorageError> {
        let columns = sqlx::query("PRAGMA table_info(analytics_formal_dataset_rows)")
            .fetch_all(&self.pool)
            .await?;
        let column_names = columns
            .into_iter()
            .map(|row| row.try_get::<String, _>("name"))
            .collect::<Result<HashSet<_>, _>>()?;

        for (column_name, alter_sql) in [
            (
                "regime_5d",
                "ALTER TABLE analytics_formal_dataset_rows ADD COLUMN regime_5d TEXT NOT NULL DEFAULT 'normal'",
            ),
            (
                "regime_20d",
                "ALTER TABLE analytics_formal_dataset_rows ADD COLUMN regime_20d TEXT NOT NULL DEFAULT 'normal'",
            ),
            (
                "regime_60d",
                "ALTER TABLE analytics_formal_dataset_rows ADD COLUMN regime_60d TEXT NOT NULL DEFAULT 'normal'",
            ),
        ] {
            if !column_names.contains(column_name) {
                sqlx::query(alter_sql).execute(&self.pool).await?;
            }
        }

        Ok(())
    }

    async fn ensure_formal_dataset_action_label_columns(&self) -> Result<(), StorageError> {
        let columns = sqlx::query("PRAGMA table_info(analytics_formal_dataset_rows)")
            .fetch_all(&self.pool)
            .await?;
        let column_names = columns
            .into_iter()
            .map(|row| row.try_get::<String, _>("name"))
            .collect::<Result<HashSet<_>, _>>()?;

        for (column_name, alter_sql) in [
            (
                "action_label_5d",
                "ALTER TABLE analytics_formal_dataset_rows ADD COLUMN action_label_5d INTEGER NOT NULL DEFAULT 0",
            ),
            (
                "action_label_20d",
                "ALTER TABLE analytics_formal_dataset_rows ADD COLUMN action_label_20d INTEGER NOT NULL DEFAULT 0",
            ),
            (
                "action_label_60d",
                "ALTER TABLE analytics_formal_dataset_rows ADD COLUMN action_label_60d INTEGER NOT NULL DEFAULT 0",
            ),
        ] {
            if !column_names.contains(column_name) {
                sqlx::query(alter_sql).execute(&self.pool).await?;
            }
        }

        Ok(())
    }

    async fn ensure_formal_dataset_action_episode_columns(&self) -> Result<(), StorageError> {
        let columns = sqlx::query("PRAGMA table_info(analytics_formal_dataset_rows)")
            .fetch_all(&self.pool)
            .await?;
        let column_names = columns
            .into_iter()
            .map(|row| row.try_get::<String, _>("name"))
            .collect::<Result<HashSet<_>, _>>()?;

        for (column_name, alter_sql) in [
            (
                "scenario_training_role",
                "ALTER TABLE analytics_formal_dataset_rows ADD COLUMN scenario_training_role TEXT",
            ),
            (
                "prepare_episode_label",
                "ALTER TABLE analytics_formal_dataset_rows ADD COLUMN prepare_episode_label INTEGER NOT NULL DEFAULT 0",
            ),
            (
                "hedge_episode_label",
                "ALTER TABLE analytics_formal_dataset_rows ADD COLUMN hedge_episode_label INTEGER NOT NULL DEFAULT 0",
            ),
            (
                "defend_episode_label",
                "ALTER TABLE analytics_formal_dataset_rows ADD COLUMN defend_episode_label INTEGER NOT NULL DEFAULT 0",
            ),
            (
                "primary_action_level",
                "ALTER TABLE analytics_formal_dataset_rows ADD COLUMN primary_action_level TEXT",
            ),
            (
                "action_episode_id",
                "ALTER TABLE analytics_formal_dataset_rows ADD COLUMN action_episode_id TEXT",
            ),
            (
                "action_episode_phase",
                "ALTER TABLE analytics_formal_dataset_rows ADD COLUMN action_episode_phase TEXT NOT NULL DEFAULT 'outside'",
            ),
            (
                "protected_action_window",
                "ALTER TABLE analytics_formal_dataset_rows ADD COLUMN protected_action_window INTEGER NOT NULL DEFAULT 0",
            ),
        ] {
            if !column_names.contains(column_name) {
                sqlx::query(alter_sql).execute(&self.pool).await?;
            }
        }

        Ok(())
    }

    pub(super) async fn initialize_connection(&self) -> Result<(), StorageError> {
        sqlx::query("PRAGMA journal_mode = WAL")
            .execute(&self.pool)
            .await?;
        sqlx::query("PRAGMA foreign_keys = ON")
            .execute(&self.pool)
            .await?;
        sqlx::query("PRAGMA busy_timeout = 5000")
            .execute(&self.pool)
            .await?;
        sqlx::query("PRAGMA synchronous = NORMAL")
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}
