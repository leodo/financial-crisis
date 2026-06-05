mod features;
mod formal;
mod snapshot;

use fc_storage::SqliteStore;

use super::{PipelineDatasetSource, PipelineTrainOptions};

pub(crate) use features::transitional_feature_names;
pub(crate) use formal::resolve_formal_dataset_key;
pub(crate) use snapshot::build_pipeline_dataset_rows;

pub(crate) async fn load_probability_training_input(
    store: &SqliteStore,
    options: &PipelineTrainOptions,
) -> anyhow::Result<crate::ProbabilityTrainingInput> {
    match options.dataset_source {
        PipelineDatasetSource::Formal => formal::load_formal_training_dataset(store, options).await,
        PipelineDatasetSource::Snapshot => {
            snapshot::load_snapshot_training_dataset(store, options).await
        }
    }
}
