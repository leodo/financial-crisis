use std::collections::BTreeMap;

pub const FEATURE_OVERALL_SCORE: &str = "overall_score";
pub const FEATURE_EXTERNAL_SHOCK_SCORE: &str = "external_shock_score";
pub const FEATURE_HEURISTIC_P_5D: &str = "heuristic_p_5d";
pub const FEATURE_HEURISTIC_P_20D: &str = "heuristic_p_20d";
pub const FEATURE_HEURISTIC_P_60D: &str = "heuristic_p_60d";
pub const FEATURE_COVERAGE_SCORE: &str = "coverage_score";
pub const FEATURE_BUCKET_MONTHS_OR_HIGHER: &str = "bucket_months_or_higher";
pub const FEATURE_BUCKET_WEEKS_OR_HIGHER: &str = "bucket_weeks_or_higher";
pub const FEATURE_BUCKET_NOW: &str = "bucket_now";
pub const FEATURE_FRESHNESS_DELAYED_OR_WORSE: &str = "freshness_delayed_or_worse";
pub const FEATURE_FRESHNESS_STALE_OR_MISSING: &str = "freshness_stale_or_missing";
pub const FEATURE_STRUCTURAL_SCORE: &str = "structural_score";
pub const FEATURE_TRIGGER_SCORE: &str = "trigger_score";
pub const FEATURE_EXTERNAL_DIMENSION_SCORE: &str = "external_dimension_score";
pub const FEATURE_US_VIX_LEVEL: &str = "us_vix_level";
pub const FEATURE_US_VIX_CHANGE_5D: &str = "us_vix_change_5d";
pub const FEATURE_US_CURVE_10Y2Y_LEVEL: &str = "us_curve_10y2y_level";
pub const FEATURE_US_BAA_10Y_SPREAD_LEVEL: &str = "us_baa_10y_spread_level";
pub const FEATURE_US_FED_FUNDS_LEVEL: &str = "us_fed_funds_level";
pub const FEATURE_US_NFCI_LEVEL: &str = "us_nfci_level";
pub const FEATURE_US_STLFSI_LEVEL: &str = "us_stlfsi_level";
pub const FEATURE_US_UNEMPLOYMENT_LEVEL: &str = "us_unemployment_level";
pub const FEATURE_US_HOUSING_STARTS_LEVEL: &str = "us_housing_starts_level";
pub const FEATURE_US_USDJPY_LEVEL: &str = "us_usdjpy_level";
pub const FEATURE_US_USDJPY_CHANGE_20D: &str = "us_usdjpy_change_20d";
pub const PROBABILITY_MODEL_FAMILY_LINEAR_V1: &str = "linear_v1";
pub const PROBABILITY_MODEL_FAMILY_INTERACTION_TAIL_V1: &str = "interaction_tail_v1";
pub const PROBABILITY_MODEL_FAMILY_FAMILY_CONDITIONAL_V1: &str = "family_conditional_v1";
pub const PROBABILITY_MODEL_FAMILY_FAMILY_HYBRID_V1: &str = "family_hybrid_v1";
pub const PROBABILITY_FEATURE_TRANSFORM_IDENTITY_V1: &str = "identity_v1";
pub const PROBABILITY_FEATURE_TRANSFORM_INTERACTION_TAIL_V1: &str = "interaction_tail_v1";
pub const PROBABILITY_FEATURE_TRANSFORM_FAMILY_CONDITIONAL_V1: &str = "family_conditional_v1";
pub const PROBABILITY_FEATURE_TRANSFORM_FAMILY_HYBRID_V1: &str = "family_hybrid_v1";

pub const TRANSITIONAL_PROBABILITY_BUNDLE_FEATURES: &[&str] = &[
    FEATURE_OVERALL_SCORE,
    FEATURE_EXTERNAL_SHOCK_SCORE,
    FEATURE_HEURISTIC_P_5D,
    FEATURE_HEURISTIC_P_20D,
    FEATURE_HEURISTIC_P_60D,
    FEATURE_COVERAGE_SCORE,
    FEATURE_BUCKET_MONTHS_OR_HIGHER,
    FEATURE_BUCKET_WEEKS_OR_HIGHER,
    FEATURE_BUCKET_NOW,
    FEATURE_FRESHNESS_DELAYED_OR_WORSE,
    FEATURE_FRESHNESS_STALE_OR_MISSING,
];

pub const FORMAL_PROBABILITY_BUNDLE_FEATURES: &[&str] = &[
    FEATURE_OVERALL_SCORE,
    FEATURE_STRUCTURAL_SCORE,
    FEATURE_TRIGGER_SCORE,
    FEATURE_EXTERNAL_DIMENSION_SCORE,
    FEATURE_COVERAGE_SCORE,
    FEATURE_US_VIX_LEVEL,
    FEATURE_US_VIX_CHANGE_5D,
    FEATURE_US_CURVE_10Y2Y_LEVEL,
    FEATURE_US_BAA_10Y_SPREAD_LEVEL,
    FEATURE_US_FED_FUNDS_LEVEL,
    FEATURE_US_NFCI_LEVEL,
    FEATURE_US_STLFSI_LEVEL,
    FEATURE_US_UNEMPLOYMENT_LEVEL,
    FEATURE_US_HOUSING_STARTS_LEVEL,
    FEATURE_US_USDJPY_LEVEL,
    FEATURE_US_USDJPY_CHANGE_20D,
];

pub const PROBABILITY_BUNDLE_FEATURES: &[&str] = TRANSITIONAL_PROBABILITY_BUNDLE_FEATURES;
pub const INTERACTION_TAIL_DERIVED_FEATURES: &[&str] = &[
    "interaction__overall_score__us_vix_level",
    "interaction__structural_score__trigger_score",
    "interaction__trigger_score__us_vix_level",
    "interaction__trigger_score__us_usdjpy_change_20d",
    "interaction__external_dimension_score__us_usdjpy_level",
    "interaction__us_curve_10y2y_level__us_fed_funds_level",
    "interaction__us_nfci_level__us_stlfsi_level",
    "interaction__us_baa_10y_spread_level__us_vix_level",
    "tail_pos__us_vix_level__24",
    "tail_pos__us_vix_level__32",
    "tail_pos__us_baa_10y_spread_level__2",
    "tail_pos__us_stlfsi_level__1",
    "tail_pos__us_usdjpy_level__145",
    "tail_abs_pos__us_usdjpy_change_20d__4",
    "tail_neg__us_curve_10y2y_level__0",
    "tail_pos__overall_score__55",
    "tail_pos__structural_score__52",
    "tail_pos__trigger_score__50",
    "tail_pos__external_dimension_score__50",
];

pub const FAMILY_CONDITIONAL_DERIVED_FEATURES: &[&str] = &[
    "family_proxy__systemic_credit",
    "family_proxy__mixed_systemic",
    "family_proxy__rate_shock",
    "family_proxy__acute_liquidity",
    "family_proxy__jpy_carry",
    "family_context__systemic_credit__structural_score",
    "family_context__mixed_systemic__trigger_score",
    "family_context__rate_shock__external_dimension_score",
    "family_context__acute_liquidity__trigger_score",
    "family_context__jpy_carry__external_dimension_score",
];

pub fn probability_feature_names_for_transform(
    base_feature_names: &[String],
    feature_transform: &str,
) -> Vec<String> {
    let mut names = base_feature_names.to_vec();
    if feature_transform == PROBABILITY_FEATURE_TRANSFORM_INTERACTION_TAIL_V1
        || feature_transform == PROBABILITY_FEATURE_TRANSFORM_FAMILY_CONDITIONAL_V1
        || feature_transform == PROBABILITY_FEATURE_TRANSFORM_FAMILY_HYBRID_V1
    {
        for feature_name in INTERACTION_TAIL_DERIVED_FEATURES {
            if !names.iter().any(|existing| existing == feature_name) {
                names.push((*feature_name).to_string());
            }
        }
    }
    if feature_transform == PROBABILITY_FEATURE_TRANSFORM_FAMILY_CONDITIONAL_V1
        || feature_transform == PROBABILITY_FEATURE_TRANSFORM_FAMILY_HYBRID_V1
    {
        for feature_name in FAMILY_CONDITIONAL_DERIVED_FEATURES {
            if !names.iter().any(|existing| existing == feature_name) {
                names.push((*feature_name).to_string());
            }
        }
    }
    names
}

pub fn resolve_probability_feature_value(
    feature_name: &str,
    features: &BTreeMap<String, f64>,
) -> Option<f64> {
    if let Some(value) = features.get(feature_name) {
        return Some(*value);
    }

    let parts = feature_name.split("__").collect::<Vec<_>>();
    if parts.first() == Some(&"interaction") && parts.len() > 2 {
        return resolve_interaction_feature_value(&parts[1..], features);
    }
    match parts.as_slice() {
        ["tail_pos", base, threshold] => Some(
            (resolve_probability_feature_value(base, features)? - threshold.parse::<f64>().ok()?)
                .max(0.0),
        ),
        ["tail_neg", base, threshold] => Some(
            (threshold.parse::<f64>().ok()? - resolve_probability_feature_value(base, features)?)
                .max(0.0),
        ),
        ["tail_abs_pos", base, threshold] => Some(
            (resolve_probability_feature_value(base, features)?.abs()
                - threshold.parse::<f64>().ok()?)
            .max(0.0),
        ),
        ["family_proxy", family] => resolve_family_proxy_value(family, features),
        ["family_context", family, base] => Some(
            resolve_family_proxy_value(family, features)?
                * resolve_probability_feature_value(base, features)?,
        ),
        _ => None,
    }
}

fn resolve_interaction_feature_value(
    parts: &[&str],
    features: &BTreeMap<String, f64>,
) -> Option<f64> {
    for split_index in 1..parts.len() {
        let left = parts[..split_index].join("__");
        let right = parts[split_index..].join("__");
        if let (Some(left_value), Some(right_value)) = (
            resolve_probability_feature_value(&left, features),
            resolve_probability_feature_value(&right, features),
        ) {
            return Some(left_value * right_value);
        }
    }
    None
}

fn resolve_family_proxy_value(family: &str, features: &BTreeMap<String, f64>) -> Option<f64> {
    match family {
        "systemic_credit" => Some(
            0.35 * scaled_tail_pos(features, FEATURE_US_BAA_10Y_SPREAD_LEVEL, 2.0, 3.0)?
                + 0.25 * scaled_tail_pos(features, FEATURE_US_STLFSI_LEVEL, 1.0, 3.0)?
                + 0.20 * scaled_tail_pos(features, FEATURE_US_NFCI_LEVEL, 0.25, 1.5)?
                + 0.20 * scaled_tail_pos(features, FEATURE_STRUCTURAL_SCORE, 52.0, 28.0)?,
        ),
        "mixed_systemic" => {
            let trigger_tail = scaled_tail_pos(features, FEATURE_TRIGGER_SCORE, 42.0, 28.0)?;
            let overall_tail = scaled_tail_pos(features, FEATURE_OVERALL_SCORE, 48.0, 30.0)?;
            let external_tail =
                scaled_tail_pos(features, FEATURE_EXTERNAL_DIMENSION_SCORE, 45.0, 30.0)?;
            let vix_tail = scaled_tail_pos(features, FEATURE_US_VIX_LEVEL, 20.0, 18.0)?;
            let credit_tail = scaled_tail_pos(features, FEATURE_US_BAA_10Y_SPREAD_LEVEL, 1.4, 2.0)?;
            let curve_tail = scaled_tail_neg(features, FEATURE_US_CURVE_10Y2Y_LEVEL, 0.15, 1.6)?;
            let nfci_tail = scaled_tail_pos(features, FEATURE_US_NFCI_LEVEL, 0.10, 0.9)?;
            let usdjpy_change_tail =
                scaled_tail_abs(features, FEATURE_US_USDJPY_CHANGE_20D, 2.5, 6.0)?;
            let chronic_pressure = credit_tail.max(curve_tail).max(nfci_tail);
            let external_confirmation = external_tail.max(usdjpy_change_tail);
            let risk_confirmation = trigger_tail.max(vix_tail).max(external_confirmation);
            let chronic_confirmation = chronic_pressure * (0.30 + 0.70 * risk_confirmation);
            let broad_context = overall_tail * (0.25 + 0.75 * chronic_pressure);
            Some(
                (0.50 * chronic_confirmation
                    + 0.20 * chronic_pressure
                    + 0.15 * broad_context
                    + 0.10 * trigger_tail * chronic_pressure
                    + 0.05 * external_confirmation * chronic_pressure)
                    .clamp(0.0, 1.0),
            )
        }
        "rate_shock" => Some(
            0.35 * scaled_tail_pos(features, FEATURE_US_FED_FUNDS_LEVEL, 4.0, 3.0)?
                + 0.35 * scaled_tail_neg(features, FEATURE_US_CURVE_10Y2Y_LEVEL, 0.0, 2.0)?
                + 0.15 * scaled_tail_pos(features, FEATURE_EXTERNAL_DIMENSION_SCORE, 50.0, 35.0)?
                + 0.15 * scaled_tail_abs(features, FEATURE_US_USDJPY_CHANGE_20D, 4.0, 8.0)?,
        ),
        "acute_liquidity" => Some(
            0.55 * scaled_tail_pos(features, FEATURE_US_VIX_LEVEL, 32.0, 20.0)?
                + 0.45 * scaled_tail_pos(features, FEATURE_US_STLFSI_LEVEL, 1.0, 3.0)?,
        ),
        "jpy_carry" => {
            let level_tail = scaled_tail_pos(features, FEATURE_US_USDJPY_LEVEL, 145.0, 20.0)?;
            let change_tail = scaled_tail_abs(features, FEATURE_US_USDJPY_CHANGE_20D, 4.0, 8.0)?;
            let funding_tail = scaled_tail_pos(features, FEATURE_US_FED_FUNDS_LEVEL, 4.0, 3.0)?;
            let external_tail =
                scaled_tail_pos(features, FEATURE_EXTERNAL_DIMENSION_SCORE, 50.0, 35.0)?;
            let stress_confirmation = change_tail.max(external_tail);
            let confirmed_level = level_tail * (0.25 + 0.75 * stress_confirmation);
            Some(
                (0.45 * confirmed_level
                    + 0.25 * change_tail
                    + 0.15 * funding_tail * stress_confirmation
                    + 0.15 * external_tail)
                    .clamp(0.0, 1.0),
            )
        }
        _ => None,
    }
}

fn scaled_tail_pos(
    features: &BTreeMap<String, f64>,
    feature_name: &str,
    threshold: f64,
    scale: f64,
) -> Option<f64> {
    Some(
        ((resolve_probability_feature_value(feature_name, features)? - threshold)
            / scale.max(1e-6))
        .clamp(0.0, 1.0),
    )
}

fn scaled_tail_neg(
    features: &BTreeMap<String, f64>,
    feature_name: &str,
    threshold: f64,
    scale: f64,
) -> Option<f64> {
    Some(
        ((threshold - resolve_probability_feature_value(feature_name, features)?)
            / scale.max(1e-6))
        .clamp(0.0, 1.0),
    )
}

fn scaled_tail_abs(
    features: &BTreeMap<String, f64>,
    feature_name: &str,
    threshold: f64,
    scale: f64,
) -> Option<f64> {
    Some(
        ((resolve_probability_feature_value(feature_name, features)?.abs() - threshold)
            / scale.max(1e-6))
        .clamp(0.0, 1.0),
    )
}
