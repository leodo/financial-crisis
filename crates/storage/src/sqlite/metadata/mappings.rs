use sqlx::Row;

use crate::{parse_frequency, StorageError};

use super::super::{
    ExternalIndicatorMapping, SqliteStore, BOJ_FX_DATASET_ID, BOJ_MONEY_MARKET_DATASET_ID,
    FRED_DATASET_ID, TREASURY_YIELD_DATASET_ID, WORLD_BANK_DATASET_ID,
};

impl SqliteStore {
    pub async fn load_fred_mappings(&self) -> Result<Vec<ExternalIndicatorMapping>, StorageError> {
        self.load_external_mappings("fred", FRED_DATASET_ID).await
    }

    pub async fn load_treasury_yield_mappings(
        &self,
    ) -> Result<Vec<ExternalIndicatorMapping>, StorageError> {
        self.load_external_mappings("treasury", TREASURY_YIELD_DATASET_ID)
            .await
    }

    pub async fn load_world_bank_mappings(
        &self,
    ) -> Result<Vec<ExternalIndicatorMapping>, StorageError> {
        self.load_external_mappings("world_bank", WORLD_BANK_DATASET_ID)
            .await
    }

    pub async fn load_jpy_carry_mappings(
        &self,
    ) -> Result<Vec<ExternalIndicatorMapping>, StorageError> {
        let boj = self
            .load_external_mappings("boj", BOJ_FX_DATASET_ID)
            .await?
            .into_iter()
            .filter(|mapping| mapping.indicator_id == "us_external_usdjpy_level");

        let fred = self
            .load_external_mappings("fred", FRED_DATASET_ID)
            .await?
            .into_iter()
            .filter(|mapping| mapping.indicator_id == "us_external_usdjpy_level");

        Ok(boj.chain(fred).collect())
    }

    pub async fn load_boj_money_market_mappings(
        &self,
    ) -> Result<Vec<ExternalIndicatorMapping>, StorageError> {
        self.load_external_mappings("boj", BOJ_MONEY_MARKET_DATASET_ID)
            .await
    }

    pub async fn load_external_mappings(
        &self,
        source_id: &str,
        dataset_id: &str,
    ) -> Result<Vec<ExternalIndicatorMapping>, StorageError> {
        let rows = sqlx::query(
            r#"
            SELECT
                map.indicator_id,
                map.external_code,
                ind.frequency
            FROM metadata_external_indicator_mappings map
            JOIN metadata_indicators ind ON ind.indicator_id = map.indicator_id
            WHERE map.source_id = ?1
              AND map.dataset_id = ?2
              AND ind.enabled = 1
            ORDER BY map.priority, map.indicator_id
            "#,
        )
        .bind(source_id)
        .bind(dataset_id)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|row| {
                Ok(ExternalIndicatorMapping {
                    indicator_id: row.try_get("indicator_id")?,
                    external_code: row.try_get("external_code")?,
                    frequency: parse_frequency(row.try_get::<String, _>("frequency")?.as_str())?,
                })
            })
            .collect()
    }
}
