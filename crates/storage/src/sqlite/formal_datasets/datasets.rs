use fc_domain::FormalDatasetRecord;

use crate::sqlite::{formal_dataset_key, format_datetime, map_formal_dataset_row, SqliteStore};
use crate::StorageError;

impl SqliteStore {
    pub async fn upsert_formal_dataset(
        &self,
        dataset: &FormalDatasetRecord,
    ) -> Result<(), StorageError> {
        let dataset_key = formal_dataset_key(
            &dataset.manifest.dataset_id,
            &dataset.manifest.dataset_version,
        );

        let manifest_json = serde_json::to_string(&dataset.manifest)
            .map_err(|error| StorageError::Database(sqlx::Error::Decode(Box::new(error))))?;

        sqlx::query(
            r#"
            INSERT INTO analytics_formal_datasets (
                dataset_key,
                dataset_id,
                dataset_version,
                market_scope,
                feature_set_version,
                label_version,
                scenario_set_version,
                point_in_time_mode,
                from_date,
                to_date,
                train_end_date,
                calibration_end_date,
                evaluation_start_date,
                row_count,
                note,
                manifest_json,
                created_at
            )
            VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17
            )
            ON CONFLICT(dataset_key) DO UPDATE SET
                feature_set_version = excluded.feature_set_version,
                label_version = excluded.label_version,
                scenario_set_version = excluded.scenario_set_version,
                point_in_time_mode = excluded.point_in_time_mode,
                from_date = excluded.from_date,
                to_date = excluded.to_date,
                train_end_date = excluded.train_end_date,
                calibration_end_date = excluded.calibration_end_date,
                evaluation_start_date = excluded.evaluation_start_date,
                row_count = excluded.row_count,
                note = excluded.note,
                manifest_json = excluded.manifest_json,
                created_at = excluded.created_at
            "#,
        )
        .bind(&dataset_key)
        .bind(&dataset.manifest.dataset_id)
        .bind(&dataset.manifest.dataset_version)
        .bind(&dataset.manifest.market_scope)
        .bind(&dataset.manifest.feature_set_version)
        .bind(&dataset.manifest.label_version)
        .bind(&dataset.manifest.scenario_set_version)
        .bind(&dataset.manifest.point_in_time_mode)
        .bind(dataset.manifest.from_date.map(|value| value.to_string()))
        .bind(dataset.manifest.to_date.map(|value| value.to_string()))
        .bind(
            dataset
                .manifest
                .train_end_date
                .map(|value| value.to_string()),
        )
        .bind(
            dataset
                .manifest
                .calibration_end_date
                .map(|value| value.to_string()),
        )
        .bind(
            dataset
                .manifest
                .evaluation_start_date
                .map(|value| value.to_string()),
        )
        .bind(dataset.manifest.row_count as i64)
        .bind(&dataset.manifest.note)
        .bind(manifest_json)
        .bind(format_datetime(dataset.created_at))
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn load_formal_dataset(
        &self,
        dataset_key: &str,
    ) -> Result<Option<FormalDatasetRecord>, StorageError> {
        let row = sqlx::query(
            r#"
            SELECT
                dataset_id,
                dataset_version,
                market_scope,
                feature_set_version,
                label_version,
                scenario_set_version,
                point_in_time_mode,
                from_date,
                to_date,
                train_end_date,
                calibration_end_date,
                evaluation_start_date,
                row_count,
                note,
                manifest_json,
                created_at
            FROM analytics_formal_datasets
            WHERE dataset_key = ?1
            "#,
        )
        .bind(dataset_key)
        .fetch_optional(&self.pool)
        .await?;

        row.map(map_formal_dataset_row).transpose()
    }

    pub async fn list_formal_datasets(
        &self,
        market_scope: Option<&str>,
        dataset_id: Option<&str>,
        limit: Option<usize>,
    ) -> Result<Vec<FormalDatasetRecord>, StorageError> {
        let mut query = String::from(
            r#"
            SELECT
                dataset_id,
                dataset_version,
                market_scope,
                feature_set_version,
                label_version,
                scenario_set_version,
                point_in_time_mode,
                from_date,
                to_date,
                train_end_date,
                calibration_end_date,
                evaluation_start_date,
                row_count,
                note,
                manifest_json,
                created_at
            FROM analytics_formal_datasets
            WHERE 1 = 1
            "#,
        );

        let mut param_index = 1;

        if market_scope.is_some() {
            query.push_str(&format!(" AND market_scope = ?{param_index}"));
            param_index += 1;
        }

        if dataset_id.is_some() {
            query.push_str(&format!(" AND dataset_id = ?{param_index}"));
            param_index += 1;
        }

        query.push_str(" ORDER BY created_at DESC");

        if limit.is_some() {
            query.push_str(&format!(" LIMIT ?{param_index}"));
        }

        let mut statement = sqlx::query(&query);

        if let Some(scope) = market_scope {
            statement = statement.bind(scope);
        }

        if let Some(dataset_id) = dataset_id {
            statement = statement.bind(dataset_id);
        }

        if let Some(limit) = limit {
            statement = statement.bind(limit as i64);
        }

        let rows = statement.fetch_all(&self.pool).await?;

        rows.into_iter().map(map_formal_dataset_row).collect()
    }
}
