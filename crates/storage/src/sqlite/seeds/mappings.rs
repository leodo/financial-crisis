use uuid::Uuid;

use crate::StorageError;

use super::super::{SqliteStore, FRED_DATASET_ID};

impl SqliteStore {
    pub(in super::super::super) async fn upsert_fred_mapping(
        &self,
        indicator_id: &str,
        external_code: &str,
        priority: i64,
    ) -> Result<(), StorageError> {
        self.upsert_external_mapping(
            indicator_id,
            "fred",
            FRED_DATASET_ID,
            external_code,
            priority,
        )
        .await
    }

    pub(in super::super::super) async fn upsert_external_mapping(
        &self,
        indicator_id: &str,
        source_id: &str,
        dataset_id: &str,
        external_code: &str,
        priority: i64,
    ) -> Result<(), StorageError> {
        sqlx::query(
            r#"
            INSERT INTO metadata_external_indicator_mappings (
                mapping_id,
                indicator_id,
                source_id,
                dataset_id,
                external_code,
                external_params_json,
                priority
            )
            VALUES (?1, ?2, ?3, ?4, ?5, '{}', ?6)
            ON CONFLICT(indicator_id, source_id, dataset_id, external_code) DO UPDATE SET
                external_params_json = excluded.external_params_json,
                priority = excluded.priority
            "#,
        )
        .bind(Uuid::new_v4().to_string())
        .bind(indicator_id)
        .bind(source_id)
        .bind(dataset_id)
        .bind(external_code)
        .bind(priority)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}
