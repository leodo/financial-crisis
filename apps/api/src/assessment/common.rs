use fc_domain::DecisionPosture;

pub(super) fn scaled_pressure(score: f64, center: f64, width: f64) -> f64 {
    ((score - center) / width).clamp(0.0, 1.0)
}

pub(super) fn clamp_probability(value: f64) -> f64 {
    value.clamp(0.0, 0.93)
}

pub(super) fn posture_label(posture: DecisionPosture) -> &'static str {
    match posture {
        DecisionPosture::Normal => "normal",
        DecisionPosture::Prepare => "prepare",
        DecisionPosture::Hedge => "hedge",
        DecisionPosture::Defend => "defend",
    }
}

pub(super) fn round1(value: f64) -> f64 {
    (value * 10.0).round() / 10.0
}

pub(super) fn round3(value: f64) -> f64 {
    (value * 1000.0).round() / 1000.0
}

pub(super) fn format_probability_threshold(value: f64) -> String {
    format!("{value:.2}")
}

pub(super) fn round6(value: f64) -> f64 {
    (value * 1_000_000.0).round() / 1_000_000.0
}

pub(super) fn round_option(value: Option<f64>, decimals: i32) -> Option<f64> {
    let scale = 10_f64.powi(decimals);
    value.map(|value| (value * scale).round() / scale)
}
