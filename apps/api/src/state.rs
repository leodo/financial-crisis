use fc_domain::{AlertEvent, BacktestScenarioSummary, DataSource, IndicatorRisk, RiskSnapshot};

#[derive(Debug, Clone)]
pub struct AppData {
    pub overview: RiskSnapshot,
    pub indicators: Vec<IndicatorRisk>,
    pub alerts: Vec<AlertEvent>,
    pub sources: Vec<DataSource>,
    pub backtests: Vec<BacktestScenarioSummary>,
}

#[derive(Debug)]
pub struct AppState {
    data: AppData,
}

impl AppState {
    pub fn new(data: AppData) -> Self {
        Self { data }
    }

    pub fn data(&self) -> &AppData {
        &self.data
    }
}
