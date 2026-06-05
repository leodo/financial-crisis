use fc_domain::{Frequency, Indicator, RiskDimension, RiskDirection};

mod boj;
mod fred;
mod gdelt;
mod sec_events;
mod world_bank;

#[derive(Debug, Clone, Copy)]
pub(in super::super::super) struct FredIndicatorSeed {
    pub(in super::super::super) indicator_id: &'static str,
    pub(in super::super::super) display_name: &'static str,
    pub(in super::super::super) dimension: RiskDimension,
    pub(in super::super::super) description: &'static str,
    pub(in super::super::super) unit: &'static str,
    pub(in super::super::super) frequency: Frequency,
    pub(in super::super::super) risk_direction: RiskDirection,
    pub(in super::super::super) external_code: &'static str,
    pub(in super::super::super) priority: i64,
}

#[derive(Debug, Clone, Copy)]
pub(in super::super::super) struct BojIndicatorSeed {
    pub(in super::super::super) indicator_id: &'static str,
    pub(in super::super::super) display_name: &'static str,
    pub(in super::super::super) dimension: RiskDimension,
    pub(in super::super::super) description: &'static str,
    pub(in super::super::super) unit: &'static str,
    pub(in super::super::super) frequency: Frequency,
    pub(in super::super::super) risk_direction: RiskDirection,
    pub(in super::super::super) dataset_id: &'static str,
    pub(in super::super::super) external_code: &'static str,
    pub(in super::super::super) default_source_id: &'static str,
    pub(in super::super::super) quality_tier: &'static str,
    pub(in super::super::super) priority: i64,
}

#[derive(Debug, Clone, Copy)]
pub(in super::super::super) struct WorldBankIndicatorSeed {
    pub(in super::super::super) indicator_id: &'static str,
    pub(in super::super::super) display_name: &'static str,
    pub(in super::super::super) dimension: RiskDimension,
    pub(in super::super::super) description: &'static str,
    pub(in super::super::super) unit: &'static str,
    pub(in super::super::super) frequency: Frequency,
    pub(in super::super::super) risk_direction: RiskDirection,
    pub(in super::super::super) external_code: &'static str,
}

#[derive(Debug, Clone, Copy)]
pub(in super::super::super) struct SecEventIndicatorSeed {
    pub(in super::super::super) indicator_id: &'static str,
    pub(in super::super::super) display_name: &'static str,
    pub(in super::super::super) description: &'static str,
    pub(in super::super::super) unit: &'static str,
    pub(in super::super::super) risk_direction: RiskDirection,
}

#[derive(Debug, Clone, Copy)]
pub(in super::super::super) struct GdeltIndicatorSeed {
    pub(in super::super::super) indicator_id: &'static str,
    pub(in super::super::super) display_name: &'static str,
    pub(in super::super::super) description: &'static str,
    pub(in super::super::super) unit: &'static str,
    pub(in super::super::super) risk_direction: RiskDirection,
}

impl FredIndicatorSeed {
    pub(in super::super::super) fn indicator(&self) -> Indicator {
        Indicator {
            indicator_id: self.indicator_id.to_string(),
            display_name: self.display_name.to_string(),
            dimension: self.dimension,
            description: self.description.to_string(),
            unit: self.unit.to_string(),
            frequency: self.frequency,
            risk_direction: self.risk_direction,
            default_source_id: "fred".to_string(),
            quality_tier: "core".to_string(),
        }
    }
}

impl BojIndicatorSeed {
    pub(in super::super::super) fn indicator(&self) -> Indicator {
        Indicator {
            indicator_id: self.indicator_id.to_string(),
            display_name: self.display_name.to_string(),
            dimension: self.dimension,
            description: self.description.to_string(),
            unit: self.unit.to_string(),
            frequency: self.frequency,
            risk_direction: self.risk_direction,
            default_source_id: self.default_source_id.to_string(),
            quality_tier: self.quality_tier.to_string(),
        }
    }
}

impl WorldBankIndicatorSeed {
    pub(in super::super::super) fn indicator(&self) -> Indicator {
        Indicator {
            indicator_id: self.indicator_id.to_string(),
            display_name: self.display_name.to_string(),
            dimension: self.dimension,
            description: self.description.to_string(),
            unit: self.unit.to_string(),
            frequency: self.frequency,
            risk_direction: self.risk_direction,
            default_source_id: "world_bank".to_string(),
            quality_tier: "core".to_string(),
        }
    }
}

impl SecEventIndicatorSeed {
    pub(in super::super::super) fn indicator(&self) -> Indicator {
        Indicator {
            indicator_id: self.indicator_id.to_string(),
            display_name: self.display_name.to_string(),
            dimension: RiskDimension::EventsSentiment,
            description: self.description.to_string(),
            unit: self.unit.to_string(),
            frequency: Frequency::Daily,
            risk_direction: self.risk_direction,
            default_source_id: "sec_edgar".to_string(),
            quality_tier: "supplemental".to_string(),
        }
    }
}

impl GdeltIndicatorSeed {
    pub(in super::super::super) fn indicator(&self) -> Indicator {
        Indicator {
            indicator_id: self.indicator_id.to_string(),
            display_name: self.display_name.to_string(),
            dimension: RiskDimension::EventsSentiment,
            description: self.description.to_string(),
            unit: self.unit.to_string(),
            frequency: Frequency::Daily,
            risk_direction: self.risk_direction,
            default_source_id: "gdelt".to_string(),
            quality_tier: "supplemental".to_string(),
        }
    }
}

pub(in super::super::super) fn boj_indicator_seeds() -> Vec<BojIndicatorSeed> {
    boj::boj_indicator_seeds()
}

pub(in super::super::super) fn sec_event_indicator_seeds() -> Vec<SecEventIndicatorSeed> {
    sec_events::sec_event_indicator_seeds()
}

pub(in super::super::super) fn gdelt_indicator_seeds() -> Vec<GdeltIndicatorSeed> {
    gdelt::gdelt_indicator_seeds()
}

pub(in super::super::super) fn fred_indicator_seeds() -> Vec<FredIndicatorSeed> {
    fred::fred_indicator_seeds()
}

pub(in super::super::super) fn world_bank_indicator_seeds() -> Vec<WorldBankIndicatorSeed> {
    world_bank::world_bank_indicator_seeds()
}
