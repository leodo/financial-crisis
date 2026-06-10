use crate::ProbabilityTargetLabelMode;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExpectedCoefficientSign {
    Positive,
    Negative,
}

#[derive(Debug, Clone, Copy)]
struct CoefficientBounds {
    min: Option<f64>,
    max: Option<f64>,
}

fn forward_crisis_expected_coefficient_sign(
    feature_name: &str,
    horizon_days: u32,
    label_mode: ProbabilityTargetLabelMode,
) -> Option<ExpectedCoefficientSign> {
    if label_mode != ProbabilityTargetLabelMode::ForwardCrisis || horizon_days < 20 {
        return None;
    }

    if feature_name.starts_with("family_proxy__") || feature_name.starts_with("family_context__") {
        return Some(ExpectedCoefficientSign::Positive);
    }

    if horizon_days == 20 {
        // The curve inversion tail is not a simple monotonic risk head on 20d.
        // Once inversion is already entrenched, forcing this tail nonnegative
        // removes a stabilizing offset and re-opens broad normal-window noise.
        if feature_name == "tail_neg__us_curve_10y2y_level__0" {
            return None;
        }

        if let Some(base_feature_name) = derived_tail_base_feature_name(feature_name, "tail_pos__")
        {
            if matches!(
                forward_crisis_expected_base_coefficient_sign(base_feature_name),
                Some(ExpectedCoefficientSign::Positive)
            ) {
                return Some(ExpectedCoefficientSign::Positive);
            }
        }

        if let Some(base_feature_name) = derived_tail_base_feature_name(feature_name, "tail_neg__")
        {
            if matches!(
                forward_crisis_expected_base_coefficient_sign(base_feature_name),
                Some(ExpectedCoefficientSign::Negative)
            ) {
                return Some(ExpectedCoefficientSign::Positive);
            }
        }
    }

    forward_crisis_expected_base_coefficient_sign(feature_name)
}

fn forward_crisis_expected_base_coefficient_sign(
    feature_name: &str,
) -> Option<ExpectedCoefficientSign> {
    match feature_name {
        "overall_score"
        | "structural_score"
        | "trigger_score"
        | "external_dimension_score"
        | "interaction__overall_score__us_vix_level"
        | "interaction__structural_score__trigger_score"
        | "interaction__trigger_score__us_vix_level"
        | "interaction__external_dimension_score__us_usdjpy_level"
        | "interaction__us_nfci_level__us_stlfsi_level"
        | "interaction__us_baa_10y_spread_level__us_vix_level"
        | "us_vix_level"
        | "us_vix_change_5d"
        | "us_baa_10y_spread_level"
        | "us_fed_funds_level"
        | "us_nfci_level"
        | "us_stlfsi_level"
        | "us_unemployment_level" => Some(ExpectedCoefficientSign::Positive),
        "us_curve_10y2y_level" | "us_housing_starts_level" => {
            Some(ExpectedCoefficientSign::Negative)
        }
        _ => None,
    }
}

fn derived_tail_base_feature_name<'a>(feature_name: &'a str, prefix: &str) -> Option<&'a str> {
    let rest = feature_name.strip_prefix(prefix)?;
    let (base_feature_name, _) = rest.rsplit_once("__")?;
    Some(base_feature_name)
}

fn forward_crisis_sign_constraint_strength(
    horizon_days: u32,
    label_mode: ProbabilityTargetLabelMode,
) -> f64 {
    if label_mode != ProbabilityTargetLabelMode::ForwardCrisis {
        return 0.0;
    }
    match horizon_days {
        20 => 0.55,
        60 => 0.70,
        _ => 0.0,
    }
}

fn forward_crisis_coefficient_bounds(
    feature_name: &str,
    horizon_days: u32,
    label_mode: ProbabilityTargetLabelMode,
    uses_family_context_features: bool,
) -> Option<CoefficientBounds> {
    if label_mode != ProbabilityTargetLabelMode::ForwardCrisis {
        return None;
    }

    match (horizon_days, feature_name) {
        // The 2026-06-04 joint audit showed that letting this tail drift negative
        // directly erodes regional-banks 20d continuity. Keep it nonnegative on
        // 20d and force any future refinement into more explicit protected-context
        // semantics instead of blunt raw suppression.
        (20, "tail_neg__us_curve_10y2y_level__0") => Some(CoefficientBounds {
            min: Some(0.0),
            max: Some(0.0),
        }),
        // rate_shock family features should stay as auxiliary context on 20d:
        // they helped recover regional-bank timing, but without a cap they also
        // over-lift non-crisis 2023-02 / 2023-07 windows.
        (20, "family_context__rate_shock__external_dimension_score") => Some(CoefficientBounds {
            min: Some(0.0),
            max: Some(0.12),
        }),
        (20, "family_proxy__rate_shock") => Some(CoefficientBounds {
            min: Some(0.0),
            max: Some(0.06),
        }),
        // jpy_carry is still proxy-only with no labeled primary scenarios in the current
        // formal dataset. Keep it as auxiliary context rather than a broad 20d driver.
        (20, "family_context__jpy_carry__external_dimension_score") => Some(CoefficientBounds {
            min: Some(0.0),
            max: Some(0.10),
        }),
        (20, "family_proxy__jpy_carry") => Some(CoefficientBounds {
            min: Some(0.0),
            max: Some(0.06),
        }),
        // The best current family-hybrid candidate keeps USDJPY level as a real
        // positive driver. The failed 064930 / 064040 branch only looked cleaner
        // because it pushed the base level down toward 0.22 while simultaneously
        // amplifying the external-dimension interaction, which then crushed true
        // positive continuity in regional-banks. Keep the base level in a narrower
        // positive band and prevent the interaction from expanding into a harsher
        // replacement for that base semantics.
        (20, "us_usdjpy_level") if uses_family_context_features => Some(CoefficientBounds {
            min: Some(0.30),
            max: Some(0.40),
        }),
        // High USDJPY is allowed to matter as carry-pressure context, but it must
        // not become a large negative suppressor that hides a possible unwind
        // setup. Keep the high-level tail nonnegative and auxiliary across base
        // and family-context model shapes.
        (5 | 20 | 60, "tail_pos__us_usdjpy_level__145") => Some(CoefficientBounds {
            min: Some(0.0),
            max: Some(match horizon_days {
                5 => 0.12,
                _ => 0.18,
            }),
        }),
        // USDJPY 20d change is a signed carry-speed feature: a positive move can
        // mean carry build-up, while a negative move can mean unwind. Do not let
        // this ambiguous signed feature become a strong suppressor or driver;
        // keep the directional semantics in the absolute-change tail and
        // jpy_carry family proxy instead.
        (5 | 20 | 60, "us_usdjpy_change_20d")
        | (5 | 20 | 60, "interaction__trigger_score__us_usdjpy_change_20d") => {
            Some(CoefficientBounds {
                min: Some(0.0),
                max: Some(0.0),
            })
        }
        (5 | 20 | 60, "tail_abs_pos__us_usdjpy_change_20d__4") => Some(CoefficientBounds {
            min: Some(0.0),
            max: Some(0.22),
        }),
        (20, "interaction__external_dimension_score__us_usdjpy_level")
            if uses_family_context_features =>
        {
            Some(CoefficientBounds {
                min: Some(0.0),
                max: Some(0.58),
            })
        }
        (20, "us_curve_10y2y_level") if uses_family_context_features => Some(CoefficientBounds {
            min: Some(-0.72),
            max: None,
        }),
        (20, "interaction__us_curve_10y2y_level__us_fed_funds_level")
            if uses_family_context_features =>
        {
            // Separation audit 2026-06-10 showed that letting this interaction
            // collapse to zero removes a stabilizing offset in high-rate curve
            // inversion windows: false-positive windows were lifted as much as
            // the 2023 regional-banks positive window. Keep a small positive
            // floor so the negative normalized interaction can still suppress
            // generic rate-shock noise, while retaining the existing cap.
            Some(CoefficientBounds {
                min: Some(0.18),
                max: Some(0.46),
            })
        }
        _ => None,
    }
}

fn forward_crisis_coefficient_bound_strength(
    horizon_days: u32,
    label_mode: ProbabilityTargetLabelMode,
) -> f64 {
    if label_mode != ProbabilityTargetLabelMode::ForwardCrisis {
        return 0.0;
    }
    match horizon_days {
        5 => 0.30,
        20 => 0.40,
        60 => 0.35,
        _ => 0.0,
    }
}

pub(crate) fn apply_forward_crisis_sign_gradient(
    weight_gradients: &mut [f64],
    weights: &[f64],
    feature_names: &[String],
    sample_weight_sum: f64,
    horizon_days: u32,
    label_mode: ProbabilityTargetLabelMode,
) {
    let strength = forward_crisis_sign_constraint_strength(horizon_days, label_mode);
    if strength <= 0.0 {
        return;
    }

    for ((gradient, weight), feature_name) in weight_gradients
        .iter_mut()
        .zip(weights.iter())
        .zip(feature_names.iter())
    {
        let Some(expected_sign) =
            forward_crisis_expected_coefficient_sign(feature_name, horizon_days, label_mode)
        else {
            continue;
        };
        let violates_sign = match expected_sign {
            ExpectedCoefficientSign::Positive => *weight < 0.0,
            ExpectedCoefficientSign::Negative => *weight > 0.0,
        };
        if violates_sign {
            *gradient += *weight * sample_weight_sum * strength;
        }
    }
}

pub(crate) fn apply_forward_crisis_coefficient_bound_gradient(
    weight_gradients: &mut [f64],
    weights: &[f64],
    feature_names: &[String],
    sample_weight_sum: f64,
    horizon_days: u32,
    label_mode: ProbabilityTargetLabelMode,
) {
    let strength = forward_crisis_coefficient_bound_strength(horizon_days, label_mode);
    if strength <= 0.0 {
        return;
    }
    let uses_family_context_features = feature_names.iter().any(|feature_name| {
        feature_name.starts_with("family_proxy__") || feature_name.starts_with("family_context__")
    });

    for ((gradient, weight), feature_name) in weight_gradients
        .iter_mut()
        .zip(weights.iter())
        .zip(feature_names.iter())
    {
        let Some(bounds) = forward_crisis_coefficient_bounds(
            feature_name,
            horizon_days,
            label_mode,
            uses_family_context_features,
        ) else {
            continue;
        };

        if let Some(min) = bounds.min {
            if *weight < min {
                *gradient += (*weight - min) * sample_weight_sum * strength;
            }
        }
        if let Some(max) = bounds.max {
            if *weight > max {
                *gradient += (*weight - max) * sample_weight_sum * strength;
            }
        }
    }
}

pub(crate) fn project_forward_crisis_sign_constraints(
    weights: &mut [f64],
    feature_names: &[String],
    horizon_days: u32,
    label_mode: ProbabilityTargetLabelMode,
) {
    let sign_strength = forward_crisis_sign_constraint_strength(horizon_days, label_mode);
    if sign_strength <= 0.0 && label_mode != ProbabilityTargetLabelMode::ForwardCrisis {
        return;
    }
    let uses_family_context_features = feature_names.iter().any(|feature_name| {
        feature_name.starts_with("family_proxy__") || feature_name.starts_with("family_context__")
    });

    for (weight, feature_name) in weights.iter_mut().zip(feature_names.iter()) {
        if sign_strength > 0.0 {
            if let Some(expected_sign) =
                forward_crisis_expected_coefficient_sign(feature_name, horizon_days, label_mode)
            {
                match expected_sign {
                    ExpectedCoefficientSign::Positive if *weight < 0.0 => *weight = 0.0,
                    ExpectedCoefficientSign::Negative if *weight > 0.0 => *weight = 0.0,
                    _ => {}
                }
            }
        }

        if let Some(bounds) = forward_crisis_coefficient_bounds(
            feature_name,
            horizon_days,
            label_mode,
            uses_family_context_features,
        ) {
            if let Some(min) = bounds.min {
                if *weight < min {
                    *weight = min;
                }
            }
            if let Some(max) = bounds.max {
                if *weight > max {
                    *weight = max;
                }
            }
        }
    }
}
