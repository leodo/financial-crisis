use std::collections::BTreeMap;

use fc_domain::{
    formal_feature_quality_grade, formal_has_extension_acute_core_features,
    formal_has_main_dataset_core_features,
};

pub(crate) fn has_main_dataset_core_features(features: &BTreeMap<String, f64>) -> bool {
    formal_has_main_dataset_core_features(features)
}

pub(crate) fn has_extension_acute_core_features(features: &BTreeMap<String, f64>) -> bool {
    formal_has_extension_acute_core_features(features)
}

pub(crate) fn feature_quality_grade(coverage_score: f64) -> &'static str {
    formal_feature_quality_grade(coverage_score)
}
