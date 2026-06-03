mod audit;
mod backfill;
mod dataset;
mod db;
mod feature;
pub(crate) mod pipeline;
mod refresh;
pub(crate) mod release;
mod research;
pub(crate) mod snapshot;

use anyhow::{bail, Result};

pub(crate) use dataset::{
    build_formal_dataset_summary, collect_formal_dataset_scenario_ranges,
    formal_dataset_split_profile, load_formal_dataset_scenario_sets,
    load_label_set_crisis_scenarios, print_formal_dataset_summary,
    render_formal_dataset_summary_markdown, row_has_action_episode_label,
    scenario_count_for_split_range, FormalDatasetSplitProfile, FormalDatasetSummaryEnvelope,
    ScenarioRowRange,
};
#[cfg(test)]
pub(crate) use dataset::{
    formal_dataset_min_date, formal_dataset_snapshot_is_usable, formal_dataset_split_requirements,
    scenario_aware_formal_split_bounds, scenario_count_for_index_range, FormalDatasetBuildOptions,
    FormalDatasetSummaryOptions, FormalSplitLabelSupport,
};
pub(crate) use db::{db_check, db_init, db_seed};
pub(crate) use feature::{
    feature_quality_grade, has_extension_acute_core_features, has_main_dataset_core_features,
};
#[cfg(test)]
pub(crate) use feature::{
    observation_is_visible_for_date, FeatureSnapshotBuildOptions, PointInTimeMode,
};
#[cfg(test)]
pub(crate) use pipeline::ProbabilityModelShape;
pub(crate) use pipeline::{PipelineDatasetSource, PipelineTrainOptions};
#[cfg(test)]
pub(crate) use snapshot::PredictionSnapshotQueryOptions;

pub(crate) async fn run_from_args(args: Vec<String>) -> Result<()> {
    match args.as_slice() {
        [] => crate::run_demo_ingestion().await,
        [scope, action] if scope == "db" => db::handle_db_command(action).await,
        [scope, action, rest @ ..] if scope == "audit" => {
            audit::handle_audit_command(action, rest).await
        }
        [scope, area, action, rest @ ..] if scope == "research" => {
            research::handle_research_command(area, action, rest).await
        }
        [scope, action, rest @ ..] if scope == "refresh" => {
            refresh::handle_refresh_command(action, rest).await
        }
        [scope, source, rest @ ..] if scope == "backfill" => {
            backfill::handle_backfill_command(source, rest).await
        }
        [scope, ..] if matches!(scope.as_str(), "help" | "--help" | "-h") => {
            print_help();
            Ok(())
        }
        _ => unknown_command("unknown worker command"),
    }
}

fn unknown_command(message: &str) -> Result<()> {
    print_help();
    bail!("{message}")
}

fn print_help() {
    println!(
        r#"fc-worker commands:
  cargo run -p fc-worker
      Run the original mock ingestion demo.

  cargo run -p fc-worker -- db init
      Create or migrate the local SQLite database.

  cargo run -p fc-worker -- db seed
      Seed FRED, Treasury, entity, indicator, and mapping metadata.

  cargo run -p fc-worker -- db check
      Check whether key SQLite indicators are fresh enough for the dashboard.

  cargo run -p fc-worker -- audit export-current [--api-base-url URL] [--output-dir DIR]
      Fetch /api/assessment/current, /api/backtests, and /api/assessment/method from the running API, then export a JSON + Markdown rolling-audit report.

  cargo run -p fc-worker -- research release publish --manifest FILE [--activate] [--reload-api] [--skip-operational-guard] [--api-reload-url URL] [--updated-by NAME]
      Save a release manifest into SQLite, and optionally activate it and reload the API runtime. With --reload-api, worker compares timely-warning / actionable-precision guardrails and auto-rolls back on clear regression unless --skip-operational-guard is set.

  cargo run -p fc-worker -- research release list [--market-scope SCOPE]
      List model releases stored in SQLite.

  cargo run -p fc-worker -- research release show --release-id ID
      Print a stored model release as JSON.

  cargo run -p fc-worker -- research release activate --release-id ID [--market-scope SCOPE] [--reload-api] [--skip-operational-guard] [--api-reload-url URL] [--updated-by NAME]
      Mark a release active for the selected market scope and optionally reload the API runtime. With --reload-api, worker compares runtime backtest guardrails and auto-rolls back on clear regression unless --skip-operational-guard is set.

  cargo run -p fc-worker -- research release rollback --to-release-id ID [--market-scope SCOPE] [--reload-api] [--api-reload-url URL] [--updated-by NAME]
      Roll back the selected market scope to an earlier release and optionally reload the API runtime.

  cargo run -p fc-worker -- research release review --candidate-release-id ID [--baseline-release-id ID] [--market-scope SCOPE] [--api-reload-url URL] [--history-mode default|strict_rebuild] [--history-limit N] [--output-dir DIR] [--updated-by NAME]
      Temporarily switch the running API between baseline and candidate releases. Review reloads default to strict_rebuild raw history replay and history-limit=20000 before exporting JSON + Markdown, then restore the original active release. Use --history-mode default with a smaller --history-limit only for quick triage when strict rebuild is too slow.

  cargo run -p fc-worker -- research snapshot list [--market-scope SCOPE] [--release-id ID] [--from YYYY-MM-DD] [--to YYYY-MM-DD] [--limit N]
      List persisted prediction snapshots stored in SQLite for audit and release-review work.

  cargo run -p fc-worker -- research snapshot export [--market-scope SCOPE] [--release-id ID] [--from YYYY-MM-DD] [--to YYYY-MM-DD] [--limit N] [--format json|csv] [--output-path FILE]
      Export persisted prediction snapshots as JSON or CSV for external audit and release review.

  cargo run -p fc-worker -- research snapshot dataset [--market-scope SCOPE] [--release-id ID] [--from YYYY-MM-DD] [--to YYYY-MM-DD] [--format json|csv] [--output-path FILE]
      Build a point-in-time feature + forward-crisis-label dataset from persisted prediction snapshots.

  cargo run -p fc-worker -- research feature build [--market-scope SCOPE] [--from YYYY-MM-DD] [--to YYYY-MM-DD] [--feature-set-version VERSION] [--point-in-time-mode MODE] [--force-rebuild]
      Build raw-observation-backed feature snapshots for the formal model pipeline and persist them into SQLite. Existing snapshots with the same feature_set_version + PIT mode are reused unless --force-rebuild is passed.

  cargo run -p fc-worker -- research feature list [--market-scope SCOPE] [--feature-set-version VERSION] [--from YYYY-MM-DD] [--to YYYY-MM-DD] [--limit N]
      List persisted feature snapshots stored in SQLite.

  cargo run -p fc-worker -- research dataset build-main [--market-scope SCOPE] [--from YYYY-MM-DD] [--to YYYY-MM-DD] [--feature-set-version VERSION] [--point-in-time-mode MODE] [--force-rebuild] [--dataset-id ID] [--dataset-version VERSION] [--label-version VERSION] [--scenario-set-version VERSION]
      Build the formal_v1 main dataset from raw observations -> feature snapshots -> forward crisis labels, then persist the dataset manifest and rows into SQLite. Existing snapshots with the same feature_set_version + PIT mode are reused unless --force-rebuild is passed.

  cargo run -p fc-worker -- research dataset list-main [--market-scope SCOPE] [--dataset-id ID] [--limit N]
      List persisted formal dataset manifests stored in SQLite.

  cargo run -p fc-worker -- research dataset summarize-main [--market-scope SCOPE] [--dataset-id ID] [--dataset-version VERSION] [--dataset-key KEY] [--output-dir DIR]
      Summarize a persisted formal dataset, export JSON + Markdown stats, and show split/scenario/coverage diagnostics before training. Default output goes to ignored artifacts/research; pass --output-dir reports/formal-dataset to curate evidence into Git.

  cargo run -p fc-worker -- research pipeline train-probability [--dataset-source formal|snapshot] [--model-shape linear_v1|interaction_tail_v1|family_conditional_v1] [--dataset-id ID] [--dataset-version VERSION] [--dataset-key KEY] [--aux-dataset-key KEY ...] [--market-scope SCOPE] [--release-id ID] [--from YYYY-MM-DD] [--to YYYY-MM-DD] [--output-dir DIR] [--manifest-dir DIR] [--release-prefix PREFIX]
      Train a formal probability bundle. By default it uses the latest persisted formal dataset with model-shape=linear_v1 and writes generated artifacts to ignored artifacts/research directories; pass explicit output dirs only when curating evidence into Git.

  cargo run -p fc-worker -- research pipeline bootstrap-formal-release [--dataset-source formal|snapshot] [--model-shape linear_v1|interaction_tail_v1|family_conditional_v1] [--dataset-id ID] [--dataset-version VERSION] [--dataset-key KEY] [--aux-dataset-key KEY ...] [--market-scope SCOPE] [--release-id ID] [--from YYYY-MM-DD] [--to YYYY-MM-DD] [--output-dir DIR] [--manifest-dir DIR] [--release-prefix PREFIX] [--no-activate] [--no-reload-api] [--skip-operational-guard] [--api-reload-url URL] [--updated-by NAME]
      Train a formal bundle, publish it into SQLite as a model release, optionally activate it, and optionally reload the API runtime. Default source is the latest persisted formal dataset; generated files default to ignored artifacts/research directories.

  cargo run -p fc-worker -- refresh latest-free [--fast-lookback-days N] [--slow-lookback-years N] [--fred-chunk-days N] [--skip-world-bank] [--include-gdelt] [--no-reload-api] [--api-reload-url URL]
      Refresh the latest free-source data set for the dashboard, then optionally POST /api/system/reload.

  cargo run -p fc-worker -- backfill fred [--start YYYY-MM-DD] [--end YYYY-MM-DD] [--chunk-days N] [--indicator ID] [--external-code CODE]
      Fetch FRED public graph CSV observations into SQLite. No API key required. Graph CSV is chunked by default.

  cargo run -p fc-worker -- backfill fred --api [--start YYYY-MM-DD] [--end YYYY-MM-DD] [--indicator ID] [--external-code CODE]
      Fetch FRED official API observations into SQLite. Requires FRED_API_KEY.

  cargo run -p fc-worker -- backfill treasury-yield [--start YYYY-MM-DD] [--end YYYY-MM-DD] [--indicator ID] [--external-code CODE]
      Fetch U.S. Treasury yield curve observations into SQLite. No API key required.

  cargo run -p fc-worker -- backfill world-bank [--start YYYY-MM-DD] [--end YYYY-MM-DD] [--indicator ID] [--external-code CODE]
      Fetch World Bank annual macro indicators into SQLite. No API key required.

  cargo run -p fc-worker -- backfill sec-edgar [--start YYYY-MM-DD] [--end YYYY-MM-DD]
      Fetch SEC submissions metadata for the U.S. financial watchlist, aggregate filing-event features, and write alerts into SQLite. No API key required.

  cargo run -p fc-worker -- backfill gdelt [--start YYYY-MM-DD] [--end YYYY-MM-DD] [--watermark-overlap-days N]
      Fetch GDELT DOC timeline aggregates for banking/liquidity stress coverage, write raw payloads, observations, and prototype alerts into SQLite. No API key required.

  cargo run -p fc-worker -- backfill boj --dataset fx-daily [--start YYYY-MM-DD] [--end YYYY-MM-DD] [--indicator ID] [--external-code CODE]
      Fetch official BOJ USDJPY history into SQLite. No API key required.

  cargo run -p fc-worker -- backfill boj --dataset money-market [--start YYYY-MM-DD] [--end YYYY-MM-DD] [--indicator ID] [--external-code CODE]
      Fetch official BOJ uncollateralized overnight call rate history into SQLite. No API key required.

  cargo run -p fc-worker -- backfill jpy-carry [--start YYYY-MM-DD] [--end YYYY-MM-DD] [--indicator ID] [--external-code CODE]
      Fetch JPY carry USDJPY history. BOJ official FX is tried first, then FRED graph CSV is used as fallback.
"#
    );
}
