use fc_domain::{
    AlertEvent, AssessmentHistoryPoint, AssessmentSnapshot, BacktestScenarioSummary,
    BacktestWindowPoint, DataMode, DataSource, IndicatorRisk, PostureGuidance,
    ProtectedStressWindowCatalog, RiskSnapshot, ScenarioDataCoverageCatalog, UserRiskPreferences,
};
use tokio::sync::RwLock;

use crate::{
    assessment::RuntimeThresholdDiagnostics,
    data_source::{self, AppDataSource, AssessmentHistoryBuildMode, ServingRuntimePurpose},
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
    pub scenario_data_coverage_catalog: ScenarioDataCoverageCatalog,
    pub runtime_thresholds: RuntimeThresholdDiagnostics,
}

#[derive(Debug)]
pub struct AppState {
    data: RwLock<AppData>,
    source: AppDataSource,
    reload_options: RwLock<AppReloadOptions>,
    default_history_points: usize,
    max_history_points: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct AppReloadOptions {
    history_build_mode: AssessmentHistoryBuildMode,
    max_history_points: usize,
    runtime_purpose: ServingRuntimePurpose,
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
            reload_options: RwLock::new(AppReloadOptions {
                history_build_mode: AssessmentHistoryBuildMode::Default,
                max_history_points: default_history_points,
                runtime_purpose: ServingRuntimePurpose::Production,
            }),
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

    pub fn max_history_points(&self) -> usize {
        self.max_history_points
    }

    pub async fn reload(&self) -> anyhow::Result<AppData> {
        let options = *self.reload_options.read().await;
        self.reload_with_runtime_options(
            options.history_build_mode,
            options.max_history_points,
            options.runtime_purpose,
        )
        .await
    }

    pub async fn reload_with_history_mode(
        &self,
        history_build_mode: AssessmentHistoryBuildMode,
    ) -> anyhow::Result<AppData> {
        let options = *self.reload_options.read().await;
        self.reload_with_runtime_options(
            history_build_mode,
            options.max_history_points,
            options.runtime_purpose,
        )
        .await
    }

    pub async fn reload_with_history_mode_and_limit(
        &self,
        history_build_mode: AssessmentHistoryBuildMode,
        max_history_points: usize,
    ) -> anyhow::Result<AppData> {
        let options = *self.reload_options.read().await;
        self.reload_with_runtime_options(
            history_build_mode,
            max_history_points,
            options.runtime_purpose,
        )
        .await
    }

    pub async fn reload_with_runtime_options(
        &self,
        history_build_mode: AssessmentHistoryBuildMode,
        max_history_points: usize,
        runtime_purpose: ServingRuntimePurpose,
    ) -> anyhow::Result<AppData> {
        let data = data_source::load_app_data_with_runtime_options(
            &self.source,
            max_history_points,
            history_build_mode,
            runtime_purpose,
        )
        .await?;
        *self.data.write().await = data.clone();
        *self.reload_options.write().await = AppReloadOptions {
            history_build_mode,
            max_history_points,
            runtime_purpose,
        };
        Ok(data)
    }

    #[cfg(test)]
    pub async fn current_reload_config(
        &self,
    ) -> (AssessmentHistoryBuildMode, usize, ServingRuntimePurpose) {
        let options = *self.reload_options.read().await;
        (
            options.history_build_mode,
            options.max_history_points,
            options.runtime_purpose,
        )
    }
}
