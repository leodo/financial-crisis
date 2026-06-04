use anyhow::{bail, Result};

pub(crate) async fn handle_research_command(
    area: &str,
    action: &str,
    rest: &[String],
) -> Result<()> {
    match area {
        "release" => handle_release_command(action, rest).await,
        "snapshot" => handle_snapshot_command(action, rest).await,
        "feature" => handle_feature_command(action, rest).await,
        "dataset" => handle_dataset_command(action, rest).await,
        "pipeline" => handle_pipeline_command(action, rest).await,
        _ => {
            super::print_help();
            bail!("unknown research area: {area}")
        }
    }
}

async fn handle_release_command(action: &str, rest: &[String]) -> Result<()> {
    match action {
        "publish" => super::release::research_release_publish(rest).await,
        "list" => super::release::research_release_list(rest).await,
        "show" => super::release::research_release_show(rest).await,
        "activate" => super::release::research_release_activate(rest).await,
        "rollback" => super::release::research_release_rollback(rest).await,
        "review" => super::release::research_release_review(rest).await,
        "probability-slice" => super::release::research_release_probability_slice(rest).await,
        "formal-probability-slice" => {
            super::release::research_release_formal_probability_slice(rest).await
        }
        "formal-probability-compare" => {
            super::release::research_release_formal_probability_compare(rest).await
        }
        _ => {
            super::print_help();
            bail!("unknown research release command: {action}")
        }
    }
}

async fn handle_snapshot_command(action: &str, rest: &[String]) -> Result<()> {
    match action {
        "list" => super::snapshot::research_prediction_snapshot_list(rest).await,
        "export" => super::snapshot::research_prediction_snapshot_export(rest).await,
        "dataset" => super::snapshot::research_prediction_snapshot_dataset(rest).await,
        _ => {
            super::print_help();
            bail!("unknown research snapshot command: {action}")
        }
    }
}

async fn handle_feature_command(action: &str, rest: &[String]) -> Result<()> {
    match action {
        "build" => super::feature::research_feature_snapshot_build(rest).await,
        "list" => super::feature::research_feature_snapshot_list(rest).await,
        _ => {
            super::print_help();
            bail!("unknown research feature command: {action}")
        }
    }
}

async fn handle_dataset_command(action: &str, rest: &[String]) -> Result<()> {
    match action {
        "build-main" => super::dataset::research_formal_dataset_build_main(rest).await,
        "list-main" => super::dataset::research_formal_dataset_list_main(rest).await,
        "summarize-main" => super::dataset::research_formal_dataset_summarize_main(rest).await,
        "slice-main" => super::dataset::research_formal_dataset_slice_main(rest).await,
        _ => {
            super::print_help();
            bail!("unknown research dataset command: {action}")
        }
    }
}

async fn handle_pipeline_command(action: &str, rest: &[String]) -> Result<()> {
    match action {
        "train-probability" => super::pipeline::research_pipeline_train_probability(rest).await,
        "bootstrap-formal-release" => {
            super::pipeline::research_pipeline_bootstrap_formal_release(rest).await
        }
        _ => {
            super::print_help();
            bail!("unknown research pipeline command: {action}")
        }
    }
}
