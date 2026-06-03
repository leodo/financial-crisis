use chrono::Utc;
use fc_domain::{ActiveModelPointer, ModelReleaseRecord};
use sqlx::Row;

use crate::StorageError;

use super::{format_datetime, map_active_pointer_row, map_model_release_row, SqliteStore};

impl SqliteStore {
    pub async fn upsert_model_release(
        &self,

        release: &ModelReleaseRecord,
    ) -> Result<(), StorageError> {
        let manifest_json =
            serde_json::to_string(&release.manifest).unwrap_or_else(|_| "{}".to_string());

        sqlx::query(
            r#"

            INSERT INTO analytics_model_releases (

                release_id,

                market_scope,

                status,

                probability_mode,

                serving_status,

                bundle_uri,

                manifest_json,

                feature_set_version,

                label_version,

                prob_model_version,

                calibration_version,

                posture_policy_version,

                action_playbook_version,

                point_in_time_mode,

                training_range_start,

                training_range_end,

                calibration_range_start,

                calibration_range_end,

                evaluation_range_start,

                evaluation_range_end,

                brier_score,

                log_loss,

                ece,

                note,

                created_at,

                activated_at,

                retired_at

            )

            VALUES (

                ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18,

                ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26, ?27

            )

            ON CONFLICT(release_id) DO UPDATE SET

                market_scope = excluded.market_scope,

                status = excluded.status,

                probability_mode = excluded.probability_mode,

                serving_status = excluded.serving_status,

                bundle_uri = excluded.bundle_uri,

                manifest_json = excluded.manifest_json,

                feature_set_version = excluded.feature_set_version,

                label_version = excluded.label_version,

                prob_model_version = excluded.prob_model_version,

                calibration_version = excluded.calibration_version,

                posture_policy_version = excluded.posture_policy_version,

                action_playbook_version = excluded.action_playbook_version,

                point_in_time_mode = excluded.point_in_time_mode,

                training_range_start = excluded.training_range_start,

                training_range_end = excluded.training_range_end,

                calibration_range_start = excluded.calibration_range_start,

                calibration_range_end = excluded.calibration_range_end,

                evaluation_range_start = excluded.evaluation_range_start,

                evaluation_range_end = excluded.evaluation_range_end,

                brier_score = excluded.brier_score,

                log_loss = excluded.log_loss,

                ece = excluded.ece,

                note = excluded.note,

                activated_at = excluded.activated_at,

                retired_at = excluded.retired_at

            "#,
        )
        .bind(&release.manifest.release_id)
        .bind(&release.manifest.market_scope)
        .bind(&release.manifest.status)
        .bind(&release.manifest.probability_mode)
        .bind(&release.manifest.serving_status)
        .bind(&release.manifest.bundle_uri)
        .bind(manifest_json)
        .bind(&release.manifest.feature_set_version)
        .bind(&release.manifest.label_version)
        .bind(&release.manifest.prob_model_version)
        .bind(&release.manifest.calibration_version)
        .bind(&release.manifest.posture_policy_version)
        .bind(&release.manifest.action_playbook_version)
        .bind(&release.manifest.point_in_time_mode)
        .bind(
            release
                .manifest
                .training_range_start
                .map(|date| date.to_string()),
        )
        .bind(
            release
                .manifest
                .training_range_end
                .map(|date| date.to_string()),
        )
        .bind(
            release
                .manifest
                .calibration_range_start
                .map(|date| date.to_string()),
        )
        .bind(
            release
                .manifest
                .calibration_range_end
                .map(|date| date.to_string()),
        )
        .bind(
            release
                .manifest
                .evaluation_range_start
                .map(|date| date.to_string()),
        )
        .bind(
            release
                .manifest
                .evaluation_range_end
                .map(|date| date.to_string()),
        )
        .bind(release.manifest.brier_score)
        .bind(release.manifest.log_loss)
        .bind(release.manifest.ece)
        .bind(&release.manifest.note)
        .bind(format_datetime(release.created_at))
        .bind(release.activated_at.map(format_datetime))
        .bind(release.retired_at.map(format_datetime))
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn list_model_releases(
        &self,

        market_scope: Option<&str>,
    ) -> Result<Vec<ModelReleaseRecord>, StorageError> {
        let rows = if let Some(market_scope) = market_scope {
            sqlx::query(
                r#"

                SELECT *

                FROM analytics_model_releases

                WHERE market_scope = ?1

                ORDER BY created_at DESC, release_id DESC

                "#,
            )
            .bind(market_scope)
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query(
                r#"

                SELECT *

                FROM analytics_model_releases

                ORDER BY created_at DESC, release_id DESC

                "#,
            )
            .fetch_all(&self.pool)
            .await?
        };

        rows.into_iter().map(map_model_release_row).collect()
    }

    pub async fn load_model_release(
        &self,

        release_id: &str,
    ) -> Result<Option<ModelReleaseRecord>, StorageError> {
        let row = sqlx::query(
            r#"

            SELECT *

            FROM analytics_model_releases

            WHERE release_id = ?1

            "#,
        )
        .bind(release_id)
        .fetch_optional(&self.pool)
        .await?;

        row.map(map_model_release_row).transpose()
    }

    pub async fn load_active_model_pointer(
        &self,

        market_scope: &str,
    ) -> Result<Option<ActiveModelPointer>, StorageError> {
        let row = sqlx::query(
            r#"

            SELECT market_scope, release_id, updated_at, updated_by

            FROM analytics_active_model_pointers

            WHERE market_scope = ?1

            "#,
        )
        .bind(market_scope)
        .fetch_optional(&self.pool)
        .await?;

        row.map(map_active_pointer_row).transpose()
    }

    pub async fn load_active_model_release(
        &self,

        market_scope: &str,
    ) -> Result<Option<ModelReleaseRecord>, StorageError> {
        let row = sqlx::query(
            r#"

            SELECT r.*

            FROM analytics_active_model_pointers p

            JOIN analytics_model_releases r ON r.release_id = p.release_id

            WHERE p.market_scope = ?1

            "#,
        )
        .bind(market_scope)
        .fetch_optional(&self.pool)
        .await?;

        row.map(map_model_release_row).transpose()
    }

    pub async fn activate_model_release(
        &self,

        market_scope: &str,

        release_id: &str,

        actor: &str,
    ) -> Result<ModelReleaseRecord, StorageError> {
        let now = Utc::now();

        let mut transaction = self.pool.begin().await?;

        let current_active = sqlx::query(
            r#"

            SELECT release_id

            FROM analytics_active_model_pointers

            WHERE market_scope = ?1

            "#,
        )
        .bind(market_scope)
        .fetch_optional(&mut *transaction)
        .await?;

        if let Some(current_active) = current_active {
            let current_release_id: String = current_active.try_get("release_id")?;

            if current_release_id != release_id {
                sqlx::query(
                    r#"

                    UPDATE analytics_model_releases

                    SET status = 'retired',

                        retired_at = ?2

                    WHERE release_id = ?1

                    "#,
                )
                .bind(current_release_id)
                .bind(format_datetime(now))
                .execute(&mut *transaction)
                .await?;
            }
        }

        let updated = sqlx::query(
            r#"

            UPDATE analytics_model_releases

            SET status = 'active',

                activated_at = ?2,

                retired_at = NULL

            WHERE release_id = ?1

              AND market_scope = ?3

            "#,
        )
        .bind(release_id)
        .bind(format_datetime(now))
        .bind(market_scope)
        .execute(&mut *transaction)
        .await?;

        if updated.rows_affected() == 0 {
            return Err(StorageError::Database(sqlx::Error::RowNotFound));
        }

        sqlx::query(
            r#"

            INSERT INTO analytics_active_model_pointers (

                market_scope, release_id, updated_at, updated_by

            )

            VALUES (?1, ?2, ?3, ?4)

            ON CONFLICT(market_scope) DO UPDATE SET

                release_id = excluded.release_id,

                updated_at = excluded.updated_at,

                updated_by = excluded.updated_by

            "#,
        )
        .bind(market_scope)
        .bind(release_id)
        .bind(format_datetime(now))
        .bind(actor)
        .execute(&mut *transaction)
        .await?;

        transaction.commit().await?;

        self.load_model_release(release_id)
            .await?
            .ok_or(StorageError::Database(sqlx::Error::RowNotFound))
    }

    pub async fn rollback_model_release(
        &self,

        market_scope: &str,

        to_release_id: &str,

        actor: &str,
    ) -> Result<ModelReleaseRecord, StorageError> {
        let now = Utc::now();

        let mut transaction = self.pool.begin().await?;

        let current_active = sqlx::query(
            r#"

            SELECT release_id

            FROM analytics_active_model_pointers

            WHERE market_scope = ?1

            "#,
        )
        .bind(market_scope)
        .fetch_optional(&mut *transaction)
        .await?;

        if let Some(current_active) = current_active {
            let current_release_id: String = current_active.try_get("release_id")?;

            if current_release_id != to_release_id {
                sqlx::query(
                    r#"

                    UPDATE analytics_model_releases

                    SET status = 'rolled_back',

                        retired_at = ?2

                    WHERE release_id = ?1

                    "#,
                )
                .bind(current_release_id)
                .bind(format_datetime(now))
                .execute(&mut *transaction)
                .await?;
            }
        }

        let updated = sqlx::query(
            r#"

            UPDATE analytics_model_releases

            SET status = 'active',

                activated_at = ?2,

                retired_at = NULL

            WHERE release_id = ?1

              AND market_scope = ?3

            "#,
        )
        .bind(to_release_id)
        .bind(format_datetime(now))
        .bind(market_scope)
        .execute(&mut *transaction)
        .await?;

        if updated.rows_affected() == 0 {
            return Err(StorageError::Database(sqlx::Error::RowNotFound));
        }

        sqlx::query(
            r#"

            INSERT INTO analytics_active_model_pointers (

                market_scope, release_id, updated_at, updated_by

            )

            VALUES (?1, ?2, ?3, ?4)

            ON CONFLICT(market_scope) DO UPDATE SET

                release_id = excluded.release_id,

                updated_at = excluded.updated_at,

                updated_by = excluded.updated_by

            "#,
        )
        .bind(market_scope)
        .bind(to_release_id)
        .bind(format_datetime(now))
        .bind(actor)
        .execute(&mut *transaction)
        .await?;

        transaction.commit().await?;

        self.load_model_release(to_release_id)
            .await?
            .ok_or(StorageError::Database(sqlx::Error::RowNotFound))
    }
}
