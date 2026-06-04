use std::{env, fs};

use anyhow::Context;
use chrono::{Duration, Utc};
use fc_domain::{DataMode, ModelReleaseRecord, ProbabilityBundle};
use fc_storage::{PostgresStore, SqliteStore};

use crate::{
    assessment::ServingModelContext,
    demo::{build_app_data_from_inputs, build_demo_data, load_user_preferences, BuiltAppData},
    history_builder::{
        build_assessment_history, load_sqlite_assessment_history, HistoryQueryWindow,
    },
    AppData,
};

const EVENT_LOOKBACK_DAYS: i64 = 30;

#[derive(Debug, Clone)]
pub enum AppDataSource {
    Demo,
    Sqlite { path: String },
    Postgres { database_url: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssessmentHistoryBuildMode {
    Default,
    StrictRebuild,
}

pub fn source_from_env() -> anyhow::Result<AppDataSource> {
    match env::var("FC_DATA_MODE").ok().as_deref() {
        Some("postgres") => {
            let database_url = env::var("DATABASE_URL").context("DATABASE_URL is required")?;
            Ok(AppDataSource::Postgres { database_url })
        }
        Some("sqlite") => Ok(AppDataSource::Sqlite {
            path: env::var("FC_SQLITE_PATH").unwrap_or_else(|_| "data/fc-local.sqlite".to_string()),
        }),
        _ => Ok(AppDataSource::Demo),
    }
}

pub async fn load_app_data(
    source: &AppDataSource,
    max_history_points: usize,
) -> anyhow::Result<AppData> {
    load_app_data_with_history_mode(
        source,
        max_history_points,
        AssessmentHistoryBuildMode::Default,
    )
    .await
}

pub async fn load_app_data_with_history_mode(
    source: &AppDataSource,
    max_history_points: usize,
    history_build_mode: AssessmentHistoryBuildMode,
) -> anyhow::Result<AppData> {
    match source {
        AppDataSource::Demo => Ok(build_demo_data(max_history_points)),
        AppDataSource::Sqlite { path } => {
            load_sqlite_app_data(path, max_history_points, history_build_mode).await
        }
        AppDataSource::Postgres { database_url } => {
            load_postgres_app_data(database_url, max_history_points).await
        }
    }
}

async fn load_postgres_app_data(
    database_url: &str,
    _max_history_points: usize,
) -> anyhow::Result<AppData> {
    let as_of_date = Utc::now().date_naive();
    let store = PostgresStore::connect(database_url).await?;
    let indicators = store.load_indicators().await?;
    if indicators.is_empty() {
        anyhow::bail!("metadata.indicators is empty");
    }
    let observations = store
        .load_observations_for_entities(&["us", "jp"], as_of_date)
        .await?;
    if observations.is_empty() {
        anyhow::bail!("ts.indicator_observations has no rows for entity us");
    }
    let user_preferences = load_user_preferences();
    let historical = build_assessment_history(
        DataMode::Postgres,
        &fc_scoring::ScoringEngine::default(),
        &indicators,
        &observations,
        Some(&[]),
        None,
        &user_preferences,
        HistoryQueryWindow {
            from: None,
            to: None,
            limit: None,
        },
    );
    Ok(build_app_data_from_inputs(
        DataMode::Postgres,
        indicators,
        observations,
        Some(Vec::new()),
        None,
        as_of_date,
        historical.history_points,
        user_preferences,
    )
    .app_data)
}

async fn load_sqlite_app_data(
    sqlite_path: &str,
    max_history_points: usize,
    history_build_mode: AssessmentHistoryBuildMode,
) -> anyhow::Result<AppData> {
    let as_of_date = Utc::now().date_naive();
    let store = SqliteStore::connect(sqlite_path).await?;
    store.migrate().await?;
    let indicators = store.load_indicators().await?;
    if indicators.is_empty() {
        anyhow::bail!("metadata_indicators is empty; run `just db-seed` first");
    }
    let observations = store
        .load_observations_for_entities(&["us", "jp"], as_of_date)
        .await?;
    if observations.is_empty() {
        anyhow::bail!(
            "ts_indicator_observations has no rows for entity us; run at least one backfill such as `just backfill-fred`, `just backfill-treasury-yield`, or `just backfill-world-bank` first"
        );
    }
    let alerts = store
        .load_alerts_recent(as_of_date - Duration::days(EVENT_LOOKBACK_DAYS), as_of_date)
        .await?;
    let serving_model = store
        .load_active_model_release("financial_system")
        .await?
        .map(build_serving_model_context);
    let user_preferences = load_user_preferences();
    let assessment_history = load_sqlite_assessment_history(
        &store,
        &indicators,
        &observations,
        &alerts,
        serving_model.as_ref(),
        &user_preferences,
        as_of_date,
        max_history_points,
        history_build_mode,
    )
    .await?;
    let built: BuiltAppData = build_app_data_from_inputs(
        DataMode::Sqlite,
        indicators,
        observations,
        Some(alerts),
        serving_model,
        as_of_date,
        assessment_history,
        user_preferences,
    );
    if let Err(error) = store
        .upsert_prediction_snapshots(&built.prediction_snapshots)
        .await
    {
        tracing::warn!(
            sqlite_path = sqlite_path,
            error = %format!("{error:#}"),
            "failed to persist assessment prediction snapshots"
        );
    }
    Ok(built.app_data)
}

fn build_serving_model_context(release: ModelReleaseRecord) -> ServingModelContext {
    if release.manifest.probability_mode == "heuristic_mvp" {
        return ServingModelContext {
            runtime_probability_mode: release.manifest.probability_mode.clone(),
            runtime_release_status: release.manifest.serving_status.clone(),
            probability_bundle: None,
            release,
        };
    }

    match load_probability_bundle(&release.manifest.bundle_uri) {
        Ok(bundle)
            if bundle.market_scope == release.manifest.market_scope
                && bundle
                    .horizons
                    .iter()
                    .any(|horizon| horizon.horizon_days == 5)
                && bundle
                    .horizons
                    .iter()
                    .any(|horizon| horizon.horizon_days == 20)
                && bundle
                    .horizons
                    .iter()
                    .any(|horizon| horizon.horizon_days == 60) =>
        {
            ServingModelContext {
                runtime_probability_mode: bundle.probability_mode.clone(),
                runtime_release_status: release.manifest.serving_status.clone(),
                probability_bundle: Some(bundle),
                release,
            }
        }
        Ok(_) => {
            tracing::warn!(
                release_id = %release.manifest.release_id,
                bundle_uri = %release.manifest.bundle_uri,
                "active release bundle is missing required 5d/20d/60d horizons; falling back to heuristic probabilities"
            );
            ServingModelContext {
                runtime_probability_mode: "heuristic_mvp".to_string(),
                runtime_release_status: "degraded".to_string(),
                probability_bundle: None,
                release,
            }
        }
        Err(error) => {
            tracing::warn!(
                release_id = %release.manifest.release_id,
                bundle_uri = %release.manifest.bundle_uri,
                error = %error,
                "failed to load active release bundle; falling back to heuristic probabilities"
            );
            ServingModelContext {
                runtime_probability_mode: "heuristic_mvp".to_string(),
                runtime_release_status: "degraded".to_string(),
                probability_bundle: None,
                release,
            }
        }
    }
}

fn load_probability_bundle(bundle_uri: &str) -> anyhow::Result<ProbabilityBundle> {
    let raw = fs::read_to_string(bundle_uri)
        .with_context(|| format!("failed to read probability bundle from {bundle_uri}"))?;
    serde_json::from_str(&raw)
        .with_context(|| format!("failed to parse probability bundle at {bundle_uri}"))
}
