use super::*;

#[test]
fn time_to_risk_bucket_requires_confirmation_for_months_bucket() {
    let bucket = build_time_to_risk_bucket(
        &ProbabilityBlock {
            p_5d: 0.004,
            p_20d: 0.018,
            p_60d: 0.14,
        },
        None,
        None,
        None,
        44.0,
        59.0,
        40.0,
        44.0,
        32.0,
        &quiet_jpy_carry(20.0),
        ProbabilityActionThresholds {
            prepare_p60d: 0.12,
            hedge_p20d: 0.06,
            defend_p5d: 0.05,
        },
    );

    assert_eq!(bucket, TimeToRiskBucket::Normal);
}

#[test]
fn time_to_risk_bucket_allows_months_when_probability_and_context_align() {
    let bucket = build_time_to_risk_bucket(
        &ProbabilityBlock {
            p_5d: 0.004,
            p_20d: 0.05,
            p_60d: 0.14,
        },
        None,
        None,
        None,
        56.0,
        59.0,
        47.0,
        52.0,
        38.0,
        &quiet_jpy_carry(20.0),
        ProbabilityActionThresholds {
            prepare_p60d: 0.12,
            hedge_p20d: 0.06,
            defend_p5d: 0.05,
        },
    );

    assert_eq!(bucket, TimeToRiskBucket::Months);
}

#[test]
fn time_to_risk_bucket_ignores_monotonic_only_prepare_crossing() {
    let bucket = build_time_to_risk_bucket(
        &ProbabilityBlock {
            p_5d: 0.004,
            p_20d: 0.09,
            p_60d: 0.14,
        },
        Some(0.09),
        None,
        None,
        50.0,
        60.0,
        46.0,
        44.0,
        36.0,
        &quiet_jpy_carry(20.0),
        ProbabilityActionThresholds {
            prepare_p60d: 0.12,
            hedge_p20d: 0.06,
            defend_p5d: 0.05,
        },
    );

    assert_eq!(bucket, TimeToRiskBucket::Normal);
}

#[test]
fn time_to_risk_bucket_keeps_months_for_long_window_prepare_continuity() {
    let bucket = build_time_to_risk_bucket(
        &ProbabilityBlock {
            p_5d: 0.004,
            p_20d: 0.93,
            p_60d: 0.99,
        },
        Some(0.93),
        Some(&ActionabilityBlock {
            prepare: 0.24,
            hedge: 0.02,
            defend: 0.0,
        }),
        Some(&ActionabilityBlock {
            prepare: 0.24,
            hedge: 0.02,
            defend: 0.0,
        }),
        53.5,
        62.6,
        42.3,
        43.8,
        40.0,
        &quiet_jpy_carry(20.0),
        ProbabilityActionThresholds {
            prepare_p60d: 0.12,
            hedge_p20d: 0.06,
            defend_p5d: 0.05,
        },
    );

    assert_eq!(bucket, TimeToRiskBucket::Months);
}

#[test]
fn time_to_risk_bucket_does_not_bridge_without_prepare_context_support() {
    let bucket = build_time_to_risk_bucket(
        &ProbabilityBlock {
            p_5d: 0.004,
            p_20d: 0.44,
            p_60d: 0.99,
        },
        Some(0.93),
        Some(&ActionabilityBlock {
            prepare: 0.16,
            hedge: 0.02,
            defend: 0.0,
        }),
        Some(&ActionabilityBlock {
            prepare: 0.16,
            hedge: 0.02,
            defend: 0.0,
        }),
        52.0,
        62.6,
        38.0,
        40.0,
        34.0,
        &quiet_jpy_carry(20.0),
        ProbabilityActionThresholds {
            prepare_p60d: 0.12,
            hedge_p20d: 0.06,
            defend_p5d: 0.05,
        },
    );

    assert_eq!(bucket, TimeToRiskBucket::Normal);
}

#[test]
fn time_to_risk_bucket_uses_support_actionability_for_continuity_without_trigger_head() {
    let bucket = build_time_to_risk_bucket(
        &ProbabilityBlock {
            p_5d: 0.004,
            p_20d: 0.93,
            p_60d: 0.99,
        },
        Some(0.93),
        None,
        Some(&ActionabilityBlock {
            prepare: 0.245,
            hedge: 0.02,
            defend: 0.0,
        }),
        53.5,
        62.6,
        42.3,
        43.8,
        40.0,
        &quiet_jpy_carry(20.0),
        ProbabilityActionThresholds {
            prepare_p60d: 0.12,
            hedge_p20d: 0.06,
            defend_p5d: 0.05,
        },
    );

    assert_eq!(bucket, TimeToRiskBucket::Months);
}

#[test]
fn time_to_risk_bucket_uses_runtime_derived_prepare_continuity_floors() {
    let bucket = build_time_to_risk_bucket(
        &ProbabilityBlock {
            p_5d: 0.004,
            p_20d: 0.181,
            p_60d: 0.22,
        },
        Some(0.23),
        Some(&ActionabilityBlock {
            prepare: 0.245,
            hedge: 0.02,
            defend: 0.0,
        }),
        Some(&ActionabilityBlock {
            prepare: 0.245,
            hedge: 0.02,
            defend: 0.0,
        }),
        53.5,
        62.6,
        42.3,
        39.0,
        30.0,
        &quiet_jpy_carry(20.0),
        ProbabilityActionThresholds {
            prepare_p60d: 0.12,
            hedge_p20d: 0.06,
            defend_p5d: 0.05,
        },
    );

    assert_eq!(bucket, TimeToRiskBucket::Months);
}

#[test]
fn time_to_risk_bucket_promotes_months_for_trigger_dominant_plateau() {
    let bucket = build_time_to_risk_bucket(
        &ProbabilityBlock {
            p_5d: 0.004,
            p_20d: 0.239,
            p_60d: 0.901,
        },
        Some(0.901),
        None,
        None,
        48.1,
        43.2,
        54.2,
        32.8,
        30.0,
        &quiet_jpy_carry(20.0),
        ProbabilityActionThresholds {
            prepare_p60d: 0.203,
            hedge_p20d: 0.06,
            defend_p5d: 0.99,
        },
    );

    assert_eq!(bucket, TimeToRiskBucket::Months);
}

#[test]
fn time_to_risk_bucket_does_not_use_trigger_dominant_plateau_for_structural_context() {
    let bucket = build_time_to_risk_bucket(
        &ProbabilityBlock {
            p_5d: 0.004,
            p_20d: 0.39,
            p_60d: 0.93,
        },
        Some(0.93),
        None,
        None,
        54.1,
        54.8,
        53.2,
        36.1,
        30.0,
        &quiet_jpy_carry(20.0),
        ProbabilityActionThresholds {
            prepare_p60d: 0.203,
            hedge_p20d: 0.06,
            defend_p5d: 0.99,
        },
    );

    assert_eq!(bucket, TimeToRiskBucket::Normal);
}

#[test]
fn time_to_risk_bucket_blocks_saturated_weeks_without_strong_confirmation() {
    let bucket = build_time_to_risk_bucket(
        &ProbabilityBlock {
            p_5d: 0.01,
            p_20d: 0.765,
            p_60d: 0.93,
        },
        Some(0.93),
        None,
        None,
        58.5,
        63.6,
        52.2,
        39.1,
        42.0,
        &quiet_jpy_carry(20.0),
        ProbabilityActionThresholds {
            prepare_p60d: 0.12,
            hedge_p20d: 0.06,
            defend_p5d: 0.05,
        },
    );

    assert_eq!(bucket, TimeToRiskBucket::Normal);
}

#[test]
fn time_to_risk_bucket_keeps_saturated_weeks_with_external_confirmation() {
    let bucket = build_time_to_risk_bucket(
        &ProbabilityBlock {
            p_5d: 0.005,
            p_20d: 0.501,
            p_60d: 0.93,
        },
        Some(0.93),
        None,
        None,
        51.5,
        51.5,
        51.6,
        51.0,
        42.0,
        &quiet_jpy_carry(20.0),
        ProbabilityActionThresholds {
            prepare_p60d: 0.12,
            hedge_p20d: 0.06,
            defend_p5d: 0.05,
        },
    );

    assert_eq!(bucket, TimeToRiskBucket::Weeks);
}

#[test]
fn time_to_risk_bucket_blocks_saturated_months_without_strong_confirmation() {
    let bucket = build_time_to_risk_bucket(
        &ProbabilityBlock {
            p_5d: 0.009,
            p_20d: 0.62,
            p_60d: 0.93,
        },
        Some(0.93),
        Some(&ActionabilityBlock {
            prepare: 0.406,
            hedge: 0.16,
            defend: 0.0,
        }),
        Some(&ActionabilityBlock {
            prepare: 0.406,
            hedge: 0.16,
            defend: 0.0,
        }),
        55.4,
        63.4,
        45.6,
        48.3,
        40.0,
        &quiet_jpy_carry(20.0),
        ProbabilityActionThresholds {
            prepare_p60d: 0.12,
            hedge_p20d: 0.06,
            defend_p5d: 0.05,
        },
    );

    assert_eq!(bucket, TimeToRiskBucket::Normal);
}

#[test]
fn time_to_risk_bucket_promotes_months_for_probability_plateau_without_actionability_support() {
    let bucket = build_time_to_risk_bucket(
        &ProbabilityBlock {
            p_5d: 0.004,
            p_20d: 0.75,
            p_60d: 0.84,
        },
        Some(0.84),
        None,
        None,
        42.7,
        47.5,
        36.8,
        42.7,
        38.0,
        &quiet_jpy_carry(20.0),
        ProbabilityActionThresholds {
            prepare_p60d: 0.12,
            hedge_p20d: 0.06,
            defend_p5d: 0.05,
        },
    );

    assert_eq!(bucket, TimeToRiskBucket::Months);
}
