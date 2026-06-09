use std::path::PathBuf;

use anyhow::{bail, Context};

use super::super::snapshot::PredictionSnapshotQueryOptions;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PipelineDatasetSource {
    Formal,
    Snapshot,
}

impl PipelineDatasetSource {
    pub(crate) fn parse(value: &str) -> anyhow::Result<Self> {
        match value {
            "formal" => Ok(Self::Formal),
            "snapshot" => Ok(Self::Snapshot),
            other => bail!("unsupported --dataset-source value: {other}"),
        }
    }

    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Formal => "formal",
            Self::Snapshot => "snapshot",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProbabilityModelShape {
    LinearV1,
    InteractionTailV1,
    FamilyConditionalV1,
    FamilyHybridV1,
}

impl ProbabilityModelShape {
    pub(crate) fn parse(value: &str) -> anyhow::Result<Self> {
        match value {
            "linear_v1" => Ok(Self::LinearV1),
            "interaction_tail_v1" => Ok(Self::InteractionTailV1),
            "family_conditional_v1" => Ok(Self::FamilyConditionalV1),
            "family_hybrid_v1" => Ok(Self::FamilyHybridV1),
            other => bail!("unsupported --model-shape value: {other}"),
        }
    }

    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::LinearV1 => crate::PROBABILITY_MODEL_FAMILY_LINEAR_V1,
            Self::InteractionTailV1 => crate::PROBABILITY_MODEL_FAMILY_INTERACTION_TAIL_V1,
            Self::FamilyConditionalV1 => crate::PROBABILITY_MODEL_FAMILY_FAMILY_CONDITIONAL_V1,
            Self::FamilyHybridV1 => crate::PROBABILITY_MODEL_FAMILY_FAMILY_HYBRID_V1,
        }
    }

    pub(crate) fn feature_transform(self) -> &'static str {
        match self {
            Self::LinearV1 => crate::PROBABILITY_FEATURE_TRANSFORM_IDENTITY_V1,
            Self::InteractionTailV1 => crate::PROBABILITY_FEATURE_TRANSFORM_INTERACTION_TAIL_V1,
            Self::FamilyConditionalV1 => crate::PROBABILITY_FEATURE_TRANSFORM_FAMILY_CONDITIONAL_V1,
            Self::FamilyHybridV1 => crate::PROBABILITY_FEATURE_TRANSFORM_FAMILY_HYBRID_V1,
        }
    }

    pub(crate) fn base_feature_transform_for_horizon(self, horizon_days: u32) -> &'static str {
        match self {
            Self::FamilyHybridV1 if horizon_days == 60 => {
                crate::PROBABILITY_FEATURE_TRANSFORM_INTERACTION_TAIL_V1
            }
            Self::FamilyHybridV1 => crate::PROBABILITY_FEATURE_TRANSFORM_FAMILY_CONDITIONAL_V1,
            _ => self.feature_transform(),
        }
    }

    pub(crate) fn overlay_feature_transform_for_horizon(self, horizon_days: u32) -> &'static str {
        match self {
            Self::FamilyHybridV1 if horizon_days == 60 => {
                crate::PROBABILITY_FEATURE_TRANSFORM_INTERACTION_TAIL_V1
            }
            Self::FamilyHybridV1 => crate::PROBABILITY_FEATURE_TRANSFORM_FAMILY_CONDITIONAL_V1,
            _ => self.feature_transform(),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct PipelineTrainOptions {
    pub(crate) dataset_source: PipelineDatasetSource,
    pub(crate) model_shape: ProbabilityModelShape,
    pub(crate) dry_run: bool,
    pub(crate) dataset_id: String,
    pub(crate) dataset_version: Option<String>,
    pub(crate) dataset_key: Option<String>,
    pub(crate) aux_dataset_keys: Vec<String>,
    pub(crate) query: PredictionSnapshotQueryOptions,
    pub(crate) output_dir: PathBuf,
    pub(crate) manifest_dir: PathBuf,
    pub(crate) release_prefix: String,
}

impl PipelineTrainOptions {
    pub(crate) fn parse(args: &[String]) -> anyhow::Result<Self> {
        let mut output_dir = PathBuf::from(crate::DEFAULT_PIPELINE_BUNDLE_OUTPUT_DIR);
        let mut manifest_dir = PathBuf::from(crate::DEFAULT_PIPELINE_MANIFEST_OUTPUT_DIR);
        let mut release_prefix = None;
        let mut dataset_source = PipelineDatasetSource::Formal;
        let mut model_shape = ProbabilityModelShape::LinearV1;
        let mut dry_run = false;
        let mut dataset_id = crate::DEFAULT_FORMAL_DATASET_ID.to_string();
        let mut dataset_version = None;
        let mut dataset_key = None;
        let mut aux_dataset_keys = Vec::new();
        let mut query_args = Vec::new();
        let mut index = 0;
        while index < args.len() {
            match args[index].as_str() {
                "--dry-run" => dry_run = true,
                "--dataset-source" => {
                    index += 1;
                    dataset_source = PipelineDatasetSource::parse(
                        args.get(index)
                            .with_context(|| "--dataset-source requires a value")?,
                    )?;
                }
                "--model-shape" => {
                    index += 1;
                    model_shape = ProbabilityModelShape::parse(
                        args.get(index)
                            .with_context(|| "--model-shape requires a value")?,
                    )?;
                }
                "--dataset-id" => {
                    index += 1;
                    dataset_id = args
                        .get(index)
                        .with_context(|| "--dataset-id requires a value")?
                        .clone();
                }
                "--dataset-version" => {
                    index += 1;
                    dataset_version = Some(
                        args.get(index)
                            .with_context(|| "--dataset-version requires a value")?
                            .clone(),
                    );
                }
                "--dataset-key" => {
                    index += 1;
                    dataset_key = Some(
                        args.get(index)
                            .with_context(|| "--dataset-key requires a value")?
                            .clone(),
                    );
                }
                "--aux-dataset-key" => {
                    index += 1;
                    aux_dataset_keys.push(
                        args.get(index)
                            .with_context(|| "--aux-dataset-key requires a value")?
                            .clone(),
                    );
                }
                "--output-dir" => {
                    index += 1;
                    output_dir = PathBuf::from(
                        args.get(index)
                            .with_context(|| "--output-dir requires a path")?,
                    );
                }
                "--manifest-dir" => {
                    index += 1;
                    manifest_dir = PathBuf::from(
                        args.get(index)
                            .with_context(|| "--manifest-dir requires a path")?,
                    );
                }
                "--release-prefix" => {
                    index += 1;
                    release_prefix = Some(
                        args.get(index)
                            .with_context(|| "--release-prefix requires a value")?
                            .clone(),
                    );
                }
                other => query_args.push(other.to_string()),
            }
            index += 1;
        }

        Ok(Self {
            dataset_source,
            model_shape,
            dry_run,
            dataset_id,
            dataset_version,
            dataset_key,
            aux_dataset_keys,
            query: PredictionSnapshotQueryOptions::parse_with_default_limit(&query_args, None)?,
            output_dir,
            manifest_dir,
            release_prefix: release_prefix
                .unwrap_or_else(|| default_release_prefix(dataset_source, model_shape)),
        })
    }
}

#[derive(Debug, Clone)]
pub(super) struct PipelineBootstrapOptions {
    pub(super) train: PipelineTrainOptions,
    pub(super) activate: bool,
    pub(super) reload_api: bool,
    pub(super) api_reload_url: String,
    pub(super) skip_operational_guard: bool,
    pub(super) updated_by: String,
}

impl PipelineBootstrapOptions {
    pub(super) fn parse(args: &[String]) -> anyhow::Result<Self> {
        let mut activate = true;
        let mut reload_api = true;
        let mut api_reload_url = crate::DEFAULT_API_RELOAD_URL.to_string();
        let mut skip_operational_guard = false;
        let mut updated_by = "fc-worker".to_string();
        let mut train_args = Vec::new();
        let mut index = 0;
        while index < args.len() {
            match args[index].as_str() {
                "--no-activate" => activate = false,
                "--no-reload-api" => reload_api = false,
                "--skip-operational-guard" => skip_operational_guard = true,
                "--api-reload-url" => {
                    index += 1;
                    api_reload_url = args
                        .get(index)
                        .with_context(|| "--api-reload-url requires a URL")?
                        .clone();
                }
                "--updated-by" => {
                    index += 1;
                    updated_by = args
                        .get(index)
                        .with_context(|| "--updated-by requires a value")?
                        .clone();
                }
                other => train_args.push(other.to_string()),
            }
            index += 1;
        }

        let train = PipelineTrainOptions::parse(&train_args)?;
        if !matches!(train.dataset_source, PipelineDatasetSource::Formal) {
            bail!(
                "bootstrap-formal-release only supports --dataset-source formal; snapshot is transitional research only and cannot be published as a formal release"
            );
        }

        Ok(Self {
            train,
            activate,
            reload_api,
            api_reload_url,
            skip_operational_guard,
            updated_by,
        })
    }
}

fn default_release_prefix(
    dataset_source: PipelineDatasetSource,
    model_shape: ProbabilityModelShape,
) -> String {
    match (dataset_source, model_shape) {
        (PipelineDatasetSource::Formal, ProbabilityModelShape::LinearV1) => {
            "us_formal_main".to_string()
        }
        (PipelineDatasetSource::Formal, ProbabilityModelShape::InteractionTailV1) => {
            "us_formal_interaction_tail".to_string()
        }
        (PipelineDatasetSource::Formal, ProbabilityModelShape::FamilyConditionalV1) => {
            "us_formal_family_conditional".to_string()
        }
        (PipelineDatasetSource::Formal, ProbabilityModelShape::FamilyHybridV1) => {
            "us_formal_family_hybrid".to_string()
        }
        (PipelineDatasetSource::Snapshot, ProbabilityModelShape::LinearV1) => {
            "us_formal_transitional".to_string()
        }
        (PipelineDatasetSource::Snapshot, ProbabilityModelShape::InteractionTailV1) => {
            "us_formal_transitional_interaction_tail".to_string()
        }
        (PipelineDatasetSource::Snapshot, ProbabilityModelShape::FamilyConditionalV1) => {
            "us_formal_transitional_family_conditional".to_string()
        }
        (PipelineDatasetSource::Snapshot, ProbabilityModelShape::FamilyHybridV1) => {
            "us_formal_transitional_family_hybrid".to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{PipelineBootstrapOptions, PipelineDatasetSource};

    #[test]
    fn bootstrap_formal_release_rejects_snapshot_dataset_source() {
        let error = PipelineBootstrapOptions::parse(&[
            "--dataset-source".to_string(),
            "snapshot".to_string(),
        ])
        .unwrap_err();

        assert!(
            error
                .to_string()
                .contains("bootstrap-formal-release only supports --dataset-source formal"),
            "unexpected error: {error:#}"
        );
    }

    #[test]
    fn bootstrap_formal_release_accepts_formal_dataset_source() {
        let options = PipelineBootstrapOptions::parse(&[
            "--dataset-source".to_string(),
            "formal".to_string(),
            "--updated-by".to_string(),
            "tester".to_string(),
        ])
        .unwrap();

        assert_eq!(options.train.dataset_source, PipelineDatasetSource::Formal);
        assert_eq!(options.updated_by, "tester");
        assert!(options.activate);
        assert!(options.reload_api);
    }
}
