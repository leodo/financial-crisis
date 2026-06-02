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
        "publish" => crate::research_release_publish(rest).await,
        "list" => crate::research_release_list(rest).await,
        "show" => crate::research_release_show(rest).await,
        "activate" => crate::research_release_activate(rest).await,
        "rollback" => crate::research_release_rollback(rest).await,
        "review" => crate::research_release_review(rest).await,
        _ => {
            super::print_help();
            bail!("unknown research release command: {action}")
        }
    }
}

async fn handle_snapshot_command(action: &str, rest: &[String]) -> Result<()> {
    match action {
        "list" => crate::research_prediction_snapshot_list(rest).await,
        "export" => crate::research_prediction_snapshot_export(rest).await,
        "dataset" => crate::research_prediction_snapshot_dataset(rest).await,
        _ => {
            super::print_help();
            bail!("unknown research snapshot command: {action}")
        }
    }
}

async fn handle_feature_command(action: &str, rest: &[String]) -> Result<()> {
    match action {
        "build" => crate::research_feature_snapshot_build(rest).await,
        "list" => crate::research_feature_snapshot_list(rest).await,
        _ => {
            super::print_help();
            bail!("unknown research feature command: {action}")
        }
    }
}

async fn handle_dataset_command(action: &str, rest: &[String]) -> Result<()> {
    match action {
        "build-main" => crate::research_formal_dataset_build_main(rest).await,
        "list-main" => crate::research_formal_dataset_list_main(rest).await,
        "summarize-main" => crate::research_formal_dataset_summarize_main(rest).await,
        _ => {
            super::print_help();
            bail!("unknown research dataset command: {action}")
        }
    }
}

async fn handle_pipeline_command(action: &str, rest: &[String]) -> Result<()> {
    match action {
        "train-probability" => crate::research_pipeline_train_probability(rest).await,
        "bootstrap-formal-release" => crate::research_pipeline_bootstrap_formal_release(rest).await,
        _ => {
            super::print_help();
            bail!("unknown research pipeline command: {action}")
        }
    }
}
