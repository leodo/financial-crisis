use fc_domain::{
    AlertEvent, AssessmentHistoryPoint, AssessmentSnapshot, BacktestScenarioSummary,
    BacktestWindowPoint, DataMode, DataSource, IndicatorRisk, PostureGuidance,
    ProtectedStressWindowCatalog, RiskSnapshot, UserRiskPreferences,
};
use tokio::sync::RwLock;

use crate::{
    assessment::RuntimeThresholdDiagnostics,
    demo::{self, AppDataSource},
};

#[derive(Debug, Clone)]
pub struct AppData {
    pub data_mode: DataMode,
    pub user_preferences: UserRiskPreferences,
    pub overview: RiskSnapshot,
    pub indicators: Vec<IndicatorRisk>,
    pub alerts: Vec<AlertEvent>,
    pub sources: Vec<DataSource>,
    pub backtests: Vec<BacktestScenarioSummary>,
    pub backtest_timeline: Vec<BacktestWindowPoint>,
    pub assessment: AssessmentSnapshot,
    pub assessment_history: Vec<AssessmentHistoryPoint>,
    pub posture_guidance: PostureGuidance,
    pub protected_stress_window_catalog: ProtectedStressWindowCatalog,
    pub runtime_thresholds: RuntimeThresholdDiagnostics,
}

#[derive(Debug)]
pub struct AppState {
    data: RwLock<AppData>,
    source: AppDataSource,
    default_history_points: usize,
    max_history_points: usize,
}

impl AppState {
    pub fn new(
        data: AppData,
        source: AppDataSource,
        default_history_points: usize,
        max_history_points: usize,
    ) -> Self {
        Self {
            data: RwLock::new(data),
            source,
            default_history_points,
            max_history_points,
        }
    }

    pub async fn data(&self) -> AppData {
        self.data.read().await.clone()
    }

    pub fn source(&self) -> &AppDataSource {
        &self.source
    }

    pub fn default_history_points(&self) -> usize {
        self.default_history_points
    }

    pub async fn reload(&self) -> anyhow::Result<AppData> {
        let data = demo::load_app_data(&self.source, self.max_history_points).await?;
        *self.data.write().await = data.clone();
        Ok(data)
    }
}
