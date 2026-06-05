mod seeds;
mod upsert;

use crate::StorageError;

use super::super::{
    boj_indicator_seeds, fred_indicator_seeds, gdelt_indicator_seeds, sec_event_indicator_seeds,
    world_bank_indicator_seeds, SqliteStore, TREASURY_YIELD_DATASET_ID, WORLD_BANK_DATASET_ID,
};
use seeds::{metadata_dataset_seeds, metadata_entity_seeds, metadata_source_seeds};

impl SqliteStore {
    pub async fn seed_fred_metadata(&self) -> Result<(), StorageError> {
        self.seed_metadata_catalog().await?;
        self.seed_metadata_entities().await?;
        self.seed_indicator_catalog().await?;
        Ok(())
    }

    async fn seed_metadata_catalog(&self) -> Result<(), StorageError> {
        for seed in metadata_source_seeds() {
            self.upsert_metadata_source(seed).await?;
        }
        for seed in metadata_dataset_seeds() {
            self.upsert_metadata_dataset(seed).await?;
        }
        Ok(())
    }

    async fn seed_metadata_entities(&self) -> Result<(), StorageError> {
        for seed in metadata_entity_seeds() {
            self.upsert_metadata_entity(seed).await?;
        }
        Ok(())
    }

    async fn seed_indicator_catalog(&self) -> Result<(), StorageError> {
        for seed in fred_indicator_seeds() {
            let indicator = seed.indicator();
            self.upsert_indicator(&indicator).await?;
            self.upsert_fred_mapping(&indicator.indicator_id, seed.external_code, seed.priority)
                .await?;
        }

        for seed in boj_indicator_seeds() {
            let indicator = seed.indicator();
            self.upsert_indicator(&indicator).await?;
            self.upsert_external_mapping(
                &indicator.indicator_id,
                "boj",
                seed.dataset_id,
                seed.external_code,
                seed.priority,
            )
            .await?;
        }

        self.upsert_external_mapping(
            "us_rates_yield_curve_10y2y",
            "treasury",
            TREASURY_YIELD_DATASET_ID,
            "T10Y2Y",
            90,
        )
        .await?;

        for seed in world_bank_indicator_seeds() {
            let indicator = seed.indicator();
            self.upsert_indicator(&indicator).await?;
            self.upsert_external_mapping(
                &indicator.indicator_id,
                "world_bank",
                WORLD_BANK_DATASET_ID,
                seed.external_code,
                100,
            )
            .await?;
        }

        for seed in sec_event_indicator_seeds() {
            let indicator = seed.indicator();
            self.upsert_indicator(&indicator).await?;
        }

        for seed in gdelt_indicator_seeds() {
            let indicator = seed.indicator();
            self.upsert_indicator(&indicator).await?;
        }

        Ok(())
    }
}
