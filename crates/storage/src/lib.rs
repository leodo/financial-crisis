use chrono::NaiveDate;
use fc_domain::{Frequency, Indicator, Observation, RiskDimension, RiskDirection};
use sqlx::{postgres::PgPoolOptions, PgPool, Row};
use thiserror::Error;

mod sqlite;

pub use sqlite::{
    ExternalIndicatorMapping, RawResponseRecord, SqliteStore, FRED_DATASET_ID,
    TREASURY_YIELD_DATASET_ID, WORLD_BANK_DATASET_ID,
};

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("unknown risk dimension: {0}")]
    UnknownDimension(String),
    #[error("unknown frequency: {0}")]
    UnknownFrequency(String),
    #[error("unknown risk direction: {0}")]
    UnknownRiskDirection(String),
}

#[async_trait::async_trait]
pub trait RiskStore: Send + Sync {
    async fn load_indicators(&self) -> Result<Vec<Indicator>, StorageError>;

    async fn load_observations(
        &self,
        entity_id: &str,
        as_of_date: NaiveDate,
    ) -> Result<Vec<Observation>, StorageError>;

    async fn upsert_indicator(&self, indicator: &Indicator) -> Result<(), StorageError>;

    async fn insert_observations(&self, observations: &[Observation]) -> Result<(), StorageError>;
}

#[derive(Debug, Clone)]
pub struct PostgresStore {
    pool: PgPool,
}

impl PostgresStore {
    pub async fn connect(database_url: &str) -> Result<Self, StorageError> {
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(database_url)
            .await?;
        Ok(Self { pool })
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    pub async fn load_indicators(&self) -> Result<Vec<Indicator>, StorageError> {
        let rows = sqlx::query(
            r#"
            SELECT
                indicator_id,
                display_name,
                dimension,
                description,
                unit,
                frequency,
                risk_direction,
                default_source_id,
                quality_tier
            FROM metadata.indicators
            WHERE enabled = TRUE
            ORDER BY dimension, indicator_id
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|row| {
                Ok(Indicator {
                    indicator_id: row.try_get("indicator_id")?,
                    display_name: row.try_get("display_name")?,
                    dimension: parse_dimension(row.try_get::<String, _>("dimension")?.as_str())?,
                    description: row.try_get("description")?,
                    unit: row.try_get("unit")?,
                    frequency: parse_frequency(row.try_get::<String, _>("frequency")?.as_str())?,
                    risk_direction: parse_risk_direction(
                        row.try_get::<String, _>("risk_direction")?.as_str(),
                    )?,
                    default_source_id: row.try_get("default_source_id")?,
                    quality_tier: row.try_get("quality_tier")?,
                })
            })
            .collect()
    }

    pub async fn load_observations(
        &self,
        entity_id: &str,
        as_of_date: NaiveDate,
    ) -> Result<Vec<Observation>, StorageError> {
        let rows = sqlx::query(
            r#"
            SELECT
                indicator_id,
                entity_id,
                as_of_date,
                period_start,
                period_end,
                frequency,
                value,
                unit,
                source_id,
                dataset_id,
                revision_time,
                publication_time,
                quality_score,
                quality_flags
            FROM ts.indicator_observations
            WHERE entity_id = $1
              AND as_of_date <= $2
            ORDER BY indicator_id, as_of_date
            "#,
        )
        .bind(entity_id)
        .bind(as_of_date)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|row| {
                Ok(Observation {
                    indicator_id: row.try_get("indicator_id")?,
                    entity_id: row.try_get("entity_id")?,
                    as_of_date: row.try_get("as_of_date")?,
                    period_start: row.try_get("period_start")?,
                    period_end: row.try_get("period_end")?,
                    frequency: parse_frequency(row.try_get::<String, _>("frequency")?.as_str())?,
                    value: row.try_get("value")?,
                    unit: row.try_get("unit")?,
                    source_id: row.try_get("source_id")?,
                    dataset_id: row.try_get("dataset_id")?,
                    revision_time: row.try_get("revision_time")?,
                    publication_time: row.try_get("publication_time")?,
                    quality_score: row.try_get("quality_score")?,
                    quality_flags: row.try_get("quality_flags")?,
                })
            })
            .collect()
    }

    pub async fn upsert_indicator(&self, indicator: &Indicator) -> Result<(), StorageError> {
        sqlx::query(
            r#"
            INSERT INTO metadata.indicators (
                indicator_id,
                display_name,
                dimension,
                description,
                unit,
                frequency,
                risk_direction,
                default_source_id,
                quality_tier
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            ON CONFLICT (indicator_id) DO UPDATE SET
                display_name = EXCLUDED.display_name,
                dimension = EXCLUDED.dimension,
                description = EXCLUDED.description,
                unit = EXCLUDED.unit,
                frequency = EXCLUDED.frequency,
                risk_direction = EXCLUDED.risk_direction,
                default_source_id = EXCLUDED.default_source_id,
                quality_tier = EXCLUDED.quality_tier,
                enabled = TRUE
            "#,
        )
        .bind(&indicator.indicator_id)
        .bind(&indicator.display_name)
        .bind(format_dimension(indicator.dimension))
        .bind(&indicator.description)
        .bind(&indicator.unit)
        .bind(format_frequency(indicator.frequency))
        .bind(format_risk_direction(indicator.risk_direction))
        .bind(&indicator.default_source_id)
        .bind(&indicator.quality_tier)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn insert_observations(
        &self,
        observations: &[Observation],
    ) -> Result<(), StorageError> {
        let mut transaction = self.pool.begin().await?;
        for observation in observations {
            sqlx::query(
                r#"
                INSERT INTO ts.indicator_observations (
                    indicator_id,
                    entity_id,
                    as_of_date,
                    period_start,
                    period_end,
                    frequency,
                    value,
                    unit,
                    source_id,
                    dataset_id,
                    revision_time,
                    publication_time,
                    quality_score,
                    quality_flags
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
                ON CONFLICT (indicator_id, entity_id, as_of_date, frequency, source_id, revision_time)
                DO UPDATE SET
                    value = EXCLUDED.value,
                    unit = EXCLUDED.unit,
                    dataset_id = EXCLUDED.dataset_id,
                    publication_time = EXCLUDED.publication_time,
                    quality_score = EXCLUDED.quality_score,
                    quality_flags = EXCLUDED.quality_flags
                "#,
            )
            .bind(&observation.indicator_id)
            .bind(&observation.entity_id)
            .bind(observation.as_of_date)
            .bind(observation.period_start)
            .bind(observation.period_end)
            .bind(format_frequency(observation.frequency))
            .bind(observation.value)
            .bind(&observation.unit)
            .bind(&observation.source_id)
            .bind(&observation.dataset_id)
            .bind(observation.revision_time)
            .bind(observation.publication_time)
            .bind(observation.quality_score)
            .bind(&observation.quality_flags)
            .execute(&mut *transaction)
            .await?;
        }
        transaction.commit().await?;
        Ok(())
    }
}

#[async_trait::async_trait]
impl RiskStore for PostgresStore {
    async fn load_indicators(&self) -> Result<Vec<Indicator>, StorageError> {
        self.load_indicators().await
    }

    async fn load_observations(
        &self,
        entity_id: &str,
        as_of_date: NaiveDate,
    ) -> Result<Vec<Observation>, StorageError> {
        self.load_observations(entity_id, as_of_date).await
    }

    async fn upsert_indicator(&self, indicator: &Indicator) -> Result<(), StorageError> {
        self.upsert_indicator(indicator).await
    }

    async fn insert_observations(&self, observations: &[Observation]) -> Result<(), StorageError> {
        self.insert_observations(observations).await
    }
}

pub(crate) fn parse_dimension(value: &str) -> Result<RiskDimension, StorageError> {
    match value {
        "macro_fragility" => Ok(RiskDimension::MacroFragility),
        "leverage_credit" => Ok(RiskDimension::LeverageCredit),
        "market_stress" => Ok(RiskDimension::MarketStress),
        "liquidity_funding" => Ok(RiskDimension::LiquidityFunding),
        "banking_system" => Ok(RiskDimension::BankingSystem),
        "real_estate" => Ok(RiskDimension::RealEstate),
        "external_sector" => Ok(RiskDimension::ExternalSector),
        "events_sentiment" => Ok(RiskDimension::EventsSentiment),
        other => Err(StorageError::UnknownDimension(other.to_string())),
    }
}

pub(crate) fn format_dimension(value: RiskDimension) -> &'static str {
    match value {
        RiskDimension::MacroFragility => "macro_fragility",
        RiskDimension::LeverageCredit => "leverage_credit",
        RiskDimension::MarketStress => "market_stress",
        RiskDimension::LiquidityFunding => "liquidity_funding",
        RiskDimension::BankingSystem => "banking_system",
        RiskDimension::RealEstate => "real_estate",
        RiskDimension::ExternalSector => "external_sector",
        RiskDimension::EventsSentiment => "events_sentiment",
    }
}

pub(crate) fn parse_frequency(value: &str) -> Result<Frequency, StorageError> {
    match value {
        "daily" => Ok(Frequency::Daily),
        "weekly" => Ok(Frequency::Weekly),
        "monthly" => Ok(Frequency::Monthly),
        "quarterly" => Ok(Frequency::Quarterly),
        "annual" => Ok(Frequency::Annual),
        "event" => Ok(Frequency::Event),
        other => Err(StorageError::UnknownFrequency(other.to_string())),
    }
}

pub(crate) fn format_frequency(value: Frequency) -> &'static str {
    match value {
        Frequency::Daily => "daily",
        Frequency::Weekly => "weekly",
        Frequency::Monthly => "monthly",
        Frequency::Quarterly => "quarterly",
        Frequency::Annual => "annual",
        Frequency::Event => "event",
    }
}

pub(crate) fn parse_risk_direction(value: &str) -> Result<RiskDirection, StorageError> {
    match value {
        "higher_is_riskier" => Ok(RiskDirection::HigherIsRiskier),
        "lower_is_riskier" => Ok(RiskDirection::LowerIsRiskier),
        "two_sided" => Ok(RiskDirection::TwoSided),
        "falling_fast_is_riskier" => Ok(RiskDirection::FallingFastIsRiskier),
        "rising_fast_is_riskier" => Ok(RiskDirection::RisingFastIsRiskier),
        "manual_rule" => Ok(RiskDirection::ManualRule),
        other => Err(StorageError::UnknownRiskDirection(other.to_string())),
    }
}

pub(crate) fn format_risk_direction(value: RiskDirection) -> &'static str {
    match value {
        RiskDirection::HigherIsRiskier => "higher_is_riskier",
        RiskDirection::LowerIsRiskier => "lower_is_riskier",
        RiskDirection::TwoSided => "two_sided",
        RiskDirection::FallingFastIsRiskier => "falling_fast_is_riskier",
        RiskDirection::RisingFastIsRiskier => "rising_fast_is_riskier",
        RiskDirection::ManualRule => "manual_rule",
    }
}

#[cfg(test)]
mod tests {
    use fc_domain::{Frequency, RiskDimension, RiskDirection};

    use crate::{format_dimension, format_frequency, format_risk_direction};

    #[test]
    fn formats_enum_values_as_storage_strings() {
        assert_eq!(
            format_dimension(RiskDimension::MarketStress),
            "market_stress"
        );
        assert_eq!(format_frequency(Frequency::Weekly), "weekly");
        assert_eq!(
            format_risk_direction(RiskDirection::LowerIsRiskier),
            "lower_is_riskier"
        );
    }
}
