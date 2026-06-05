use crate::StorageError;

use super::super::super::SqliteStore;
use super::seeds::{MetadataDatasetSeed, MetadataEntitySeed, MetadataSourceSeed};

const fn sqlite_flag(value: bool) -> i64 {
    if value {
        1
    } else {
        0
    }
}

impl SqliteStore {
    pub(super) async fn upsert_metadata_source(
        &self,
        seed: MetadataSourceSeed,
    ) -> Result<(), StorageError> {
        sqlx::query(
            r#"
            INSERT INTO metadata_sources (
                source_id,
                display_name,
                source_type,
                official_url,
                documentation_url,
                access_method,
                auth_required,
                auth_secret_ref,
                rate_limit_policy_json,
                license_note,
                commercial_use_status,
                production_allowed,
                enabled
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
            ON CONFLICT(source_id) DO UPDATE SET
                display_name = excluded.display_name,
                documentation_url = excluded.documentation_url,
                access_method = excluded.access_method,
                auth_required = excluded.auth_required,
                auth_secret_ref = excluded.auth_secret_ref,
                rate_limit_policy_json = excluded.rate_limit_policy_json,
                license_note = excluded.license_note,
                production_allowed = excluded.production_allowed,
                enabled = excluded.enabled,
                updated_at = CURRENT_TIMESTAMP
            "#,
        )
        .bind(seed.source_id)
        .bind(seed.display_name)
        .bind(seed.source_type)
        .bind(seed.official_url)
        .bind(seed.documentation_url)
        .bind(seed.access_method)
        .bind(sqlite_flag(seed.auth_required))
        .bind(seed.auth_secret_ref)
        .bind(seed.rate_limit_policy_json)
        .bind(seed.license_note)
        .bind(seed.commercial_use_status)
        .bind(sqlite_flag(seed.production_allowed))
        .bind(sqlite_flag(seed.enabled))
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub(super) async fn upsert_metadata_dataset(
        &self,
        seed: MetadataDatasetSeed,
    ) -> Result<(), StorageError> {
        sqlx::query(
            r#"
            INSERT INTO metadata_datasets (
                dataset_id,
                source_id,
                display_name,
                frequency_set_json,
                region_set_json,
                supports_backfill,
                supports_incremental,
                supports_vintage,
                expected_latency_seconds,
                config_version,
                enabled
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
            ON CONFLICT(dataset_id) DO UPDATE SET
                display_name = excluded.display_name,
                frequency_set_json = excluded.frequency_set_json,
                region_set_json = excluded.region_set_json,
                supports_backfill = excluded.supports_backfill,
                supports_incremental = excluded.supports_incremental,
                supports_vintage = excluded.supports_vintage,
                config_version = excluded.config_version,
                enabled = excluded.enabled
            "#,
        )
        .bind(seed.dataset_id)
        .bind(seed.source_id)
        .bind(seed.display_name)
        .bind(seed.frequency_set_json)
        .bind(seed.region_set_json)
        .bind(sqlite_flag(seed.supports_backfill))
        .bind(sqlite_flag(seed.supports_incremental))
        .bind(sqlite_flag(seed.supports_vintage))
        .bind(seed.expected_latency_seconds)
        .bind(seed.config_version)
        .bind(sqlite_flag(seed.enabled))
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub(super) async fn upsert_metadata_entity(
        &self,
        seed: MetadataEntitySeed,
    ) -> Result<(), StorageError> {
        sqlx::query(
            r#"
            INSERT INTO metadata_entities (
                entity_id,
                entity_type,
                display_name,
                iso_country_code,
                currency,
                metadata_json
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            ON CONFLICT(entity_id) DO UPDATE SET
                display_name = excluded.display_name,
                iso_country_code = excluded.iso_country_code,
                currency = excluded.currency
            "#,
        )
        .bind(seed.entity_id)
        .bind(seed.entity_type)
        .bind(seed.display_name)
        .bind(seed.iso_country_code)
        .bind(seed.currency)
        .bind(seed.metadata_json)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}
