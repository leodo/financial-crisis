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
