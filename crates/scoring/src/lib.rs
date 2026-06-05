mod aggregation;
mod engine;
mod narrative;
mod signal;
#[cfg(test)]
mod tests;

pub use engine::{ScoringEngine, ScoringOutput};
pub use signal::score_value;

pub(crate) use aggregation::{
    aggregate_dimension_group, build_dimension_scores, summarize_quality,
};
pub(crate) use narrative::{build_level_reason, explain_indicator, round1};
pub(crate) use signal::{change_since_days, compute_signal};

const METHOD_VERSION: &str = "scoring_v2_20260531";
const TAIL_WEIGHT: f64 = 0.2;
const YOY_DAYS: i64 = 365;
