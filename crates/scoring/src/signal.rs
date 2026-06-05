use chrono::NaiveDate;
use fc_domain::{Frequency, Indicator, Observation, RiskDirection};

use crate::YOY_DAYS;

#[derive(Debug, Clone)]
pub(crate) struct SignalComputation {
    pub(crate) score: f64,
    pub(crate) percentile: Option<f64>,
    pub(crate) score_basis: String,
    pub(crate) score_input_value: Option<f64>,
    pub(crate) score_input_unit: Option<String>,
}

#[derive(Debug, Clone, Copy)]
enum SignalTransform {
    Level,
    ChangeDays { days: i64 },
    PctChangeDays { days: i64, absolute: bool },
}

#[derive(Debug, Clone, Copy)]
struct SignalSpec {
    transform: SignalTransform,
    scoring_direction: RiskDirection,
    activation: SignalActivation,
    basis_label: &'static str,
    unit_override: Option<&'static str>,
}

#[derive(Debug, Clone, Copy)]
enum SignalActivation {
    Any,
    PositiveOnly,
    NegativeOnly,
}

pub(crate) fn compute_signal(
    indicator: &Indicator,
    history: &[&Observation],
    latest: Option<&Observation>,
) -> SignalComputation {
    let Some(latest) = latest else {
        return SignalComputation {
            score: 0.0,
            percentile: None,
            score_basis: "缺少观测".to_string(),
            score_input_value: None,
            score_input_unit: None,
        };
    };

    if matches!(indicator.risk_direction, RiskDirection::ManualRule) {
        return SignalComputation {
            score: score_manual_rule(&indicator.indicator_id, latest.value),
            percentile: None,
            score_basis: "人工规则".to_string(),
            score_input_value: Some(latest.value),
            score_input_unit: Some(indicator.unit.clone()),
        };
    }

    let spec = signal_spec(indicator);
    let signal_series = build_signal_series(history, spec.transform);
    let signal_values = signal_series
        .iter()
        .filter_map(|(_, value)| value.is_finite().then_some(*value))
        .collect::<Vec<_>>();
    let current_signal_value = signal_series
        .iter()
        .rev()
        .find(|(date, _)| *date == latest.as_of_date)
        .map(|(_, value)| *value);
    let percentile = current_signal_value.map(|value| percentile_rank(&signal_values, value));
    let score = current_signal_value
        .map(|value| {
            if !signal_is_active(value, spec.activation) {
                0.0
            } else {
                score_value(&signal_values, value, spec.scoring_direction).0
            }
        })
        .unwrap_or(0.0);

    SignalComputation {
        score,
        percentile,
        score_basis: spec.basis_label.to_string(),
        score_input_value: current_signal_value,
        score_input_unit: Some(
            spec.unit_override
                .unwrap_or(indicator.unit.as_str())
                .to_string(),
        ),
    }
}

fn signal_spec(indicator: &Indicator) -> SignalSpec {
    match indicator.indicator_id.as_str() {
        "us_real_estate_home_price" => SignalSpec {
            transform: SignalTransform::PctChangeDays {
                days: YOY_DAYS,
                absolute: false,
            },
            scoring_direction: RiskDirection::TwoSided,
            activation: SignalActivation::Any,
            basis_label: "12m同比",
            unit_override: Some("%"),
        },
        "us_real_estate_housing_starts" => SignalSpec {
            transform: SignalTransform::PctChangeDays {
                days: YOY_DAYS,
                absolute: false,
            },
            scoring_direction: RiskDirection::LowerIsRiskier,
            activation: SignalActivation::NegativeOnly,
            basis_label: "12m同比",
            unit_override: Some("%"),
        },
        "us_liquidity_money_supply_m2" => SignalSpec {
            transform: SignalTransform::PctChangeDays {
                days: YOY_DAYS,
                absolute: false,
            },
            scoring_direction: RiskDirection::LowerIsRiskier,
            activation: SignalActivation::NegativeOnly,
            basis_label: "12m同比",
            unit_override: Some("%"),
        },
        "us_external_usdjpy_level" => SignalSpec {
            transform: SignalTransform::PctChangeDays {
                days: 20,
                absolute: true,
            },
            scoring_direction: RiskDirection::HigherIsRiskier,
            activation: SignalActivation::Any,
            basis_label: "20d振幅",
            unit_override: Some("%"),
        },
        _ => match indicator.risk_direction {
            RiskDirection::RisingFastIsRiskier => SignalSpec {
                transform: SignalTransform::ChangeDays {
                    days: default_change_window_days(indicator.frequency),
                },
                scoring_direction: RiskDirection::HigherIsRiskier,
                activation: SignalActivation::PositiveOnly,
                basis_label: "变化幅度",
                unit_override: None,
            },
            RiskDirection::FallingFastIsRiskier => SignalSpec {
                transform: SignalTransform::ChangeDays {
                    days: default_change_window_days(indicator.frequency),
                },
                scoring_direction: RiskDirection::LowerIsRiskier,
                activation: SignalActivation::NegativeOnly,
                basis_label: "变化幅度",
                unit_override: None,
            },
            other => SignalSpec {
                transform: SignalTransform::Level,
                scoring_direction: other,
                activation: SignalActivation::Any,
                basis_label: "原始水平",
                unit_override: None,
            },
        },
    }
}

fn signal_is_active(value: f64, activation: SignalActivation) -> bool {
    match activation {
        SignalActivation::Any => true,
        SignalActivation::PositiveOnly => value > 0.0,
        SignalActivation::NegativeOnly => value < 0.0,
    }
}

fn default_change_window_days(frequency: Frequency) -> i64 {
    match frequency {
        Frequency::Daily => 30,
        Frequency::Weekly => 84,
        Frequency::Monthly => YOY_DAYS,
        Frequency::Quarterly => YOY_DAYS * 2,
        Frequency::Annual => YOY_DAYS * 3,
        Frequency::Event => 30,
    }
}

fn build_signal_series(
    history: &[&Observation],
    transform: SignalTransform,
) -> Vec<(NaiveDate, f64)> {
    match transform {
        SignalTransform::Level => history
            .iter()
            .filter_map(|observation| {
                observation
                    .value
                    .is_finite()
                    .then_some((observation.as_of_date, observation.value))
            })
            .collect(),
        SignalTransform::ChangeDays { days } => difference_series(history, days, false, false),
        SignalTransform::PctChangeDays { days, absolute } => {
            difference_series(history, days, true, absolute)
        }
    }
}

fn difference_series(
    history: &[&Observation],
    lookback_days: i64,
    percent: bool,
    absolute: bool,
) -> Vec<(NaiveDate, f64)> {
    let mut derived = Vec::new();
    if history.is_empty() {
        return derived;
    }

    let mut previous_index = 0_usize;
    for current_index in 0..history.len() {
        let current = history[current_index];
        if !current.value.is_finite() {
            continue;
        }
        let cutoff = current.as_of_date - chrono::Duration::days(lookback_days);
        while previous_index + 1 < current_index && history[previous_index + 1].as_of_date <= cutoff
        {
            previous_index += 1;
        }
        let previous = history[previous_index];
        if previous_index >= current_index
            || previous.as_of_date > cutoff
            || !previous.value.is_finite()
        {
            continue;
        }

        let mut value = if percent {
            if previous.value.abs() <= f64::EPSILON {
                continue;
            }
            (current.value - previous.value) / previous.value.abs() * 100.0
        } else {
            current.value - previous.value
        };
        if absolute {
            value = value.abs();
        }
        if value.is_finite() {
            derived.push((current.as_of_date, value));
        }
    }

    derived
}

pub fn score_value(history: &[f64], value: f64, direction: RiskDirection) -> (f64, Option<f64>) {
    if history.is_empty() || !value.is_finite() {
        return (0.0, None);
    }
    let percentile = percentile_rank(history, value);
    let score = match direction {
        RiskDirection::HigherIsRiskier | RiskDirection::RisingFastIsRiskier => percentile,
        RiskDirection::LowerIsRiskier | RiskDirection::FallingFastIsRiskier => 100.0 - percentile,
        RiskDirection::TwoSided => percentile.max(100.0 - percentile),
        RiskDirection::ManualRule => percentile,
    };
    (score.clamp(0.0, 100.0), Some(percentile))
}

fn score_manual_rule(indicator_id: &str, value: f64) -> f64 {
    if !value.is_finite() || value <= 0.0 {
        return 0.0;
    }

    match indicator_id {
        "us_event_bank_8k_count" => {
            if value >= 5.0 {
                92.0
            } else if value >= 4.0 {
                82.0
            } else if value >= 3.0 {
                68.0
            } else if value >= 2.0 {
                48.0
            } else {
                24.0
            }
        }
        "us_event_risk_keyword_count" => {
            if value >= 6.0 {
                94.0
            } else if value >= 4.0 {
                82.0
            } else if value >= 3.0 {
                66.0
            } else if value >= 2.0 {
                48.0
            } else {
                28.0
            }
        }
        "us_banking_filing_stress_count" => {
            if value >= 4.0 {
                92.0
            } else if value >= 3.0 {
                78.0
            } else if value >= 2.0 {
                60.0
            } else {
                34.0
            }
        }
        "us_event_official_filing_severity" => value.clamp(0.0, 100.0),
        _ => value.clamp(0.0, 100.0),
    }
}

fn percentile_rank(history: &[f64], value: f64) -> f64 {
    let valid_values = history
        .iter()
        .copied()
        .filter(|candidate| candidate.is_finite())
        .collect::<Vec<_>>();
    if valid_values.is_empty() {
        return 0.0;
    }
    let below_or_equal = valid_values
        .iter()
        .filter(|candidate| **candidate <= value)
        .count();
    below_or_equal as f64 / valid_values.len() as f64 * 100.0
}

pub(crate) fn change_since_days(
    history: &[&Observation],
    latest_date: NaiveDate,
    days: i64,
) -> Option<f64> {
    let cutoff = latest_date - chrono::Duration::days(days);
    history
        .iter()
        .rev()
        .find(|observation| observation.as_of_date <= cutoff)
        .map(|previous| history.last().expect("history has latest").value - previous.value)
}
