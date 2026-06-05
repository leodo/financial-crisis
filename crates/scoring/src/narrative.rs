use fc_domain::{IndicatorRisk, RiskContributor, RiskLevel};

pub(crate) fn build_level_reason(level: RiskLevel, contributors: &[RiskContributor]) -> String {
    let headline = format!("{} {}", level.code(), level.label());
    match contributors.first() {
        Some(top) => format!(
            "{headline} 由 {}、{} 等指标驱动，主要集中在 {}。",
            top.display_name,
            contributors
                .get(1)
                .map(|contributor| contributor.display_name.as_str())
                .unwrap_or("其他风险信号"),
            top.dimension.label()
        ),
        None => format!("{headline}，暂无足够指标形成明确解释。"),
    }
}

pub(crate) fn explain_indicator(risk: &IndicatorRisk) -> String {
    match (
        risk.score_input_value,
        risk.score_input_unit.as_deref(),
        risk.percentile,
    ) {
        (Some(value), Some(unit), Some(percentile)) => format!(
            "{} 按{}评分，当前信号 {}，历史分位 {:.1}，风险分 {:.1}。",
            risk.indicator.display_name,
            risk.score_basis,
            format_signal_value(value, unit),
            percentile,
            risk.score
        ),
        (Some(value), Some(unit), None) => format!(
            "{} 按{}评分，当前信号 {}，风险分 {:.1}。",
            risk.indicator.display_name,
            risk.score_basis,
            format_signal_value(value, unit),
            risk.score
        ),
        _ => format!(
            "{} 当前风险分为 {:.1}，评分口径为 {}。",
            risk.indicator.display_name, risk.score, risk.score_basis
        ),
    }
}

fn format_signal_value(value: f64, unit: &str) -> String {
    match unit {
        "%" | "percent" => format!("{value:.2}%"),
        "index" | "jpy_per_usd" => format!("{value:.2}"),
        "count" => format!("{value:.0}"),
        "score" => format!("{value:.1}"),
        "billions" | "thousands" => format!("{value:.1} {unit}"),
        _ => format!("{value:.2} {unit}"),
    }
}

pub(crate) fn round1(value: f64) -> f64 {
    (value * 10.0).round() / 10.0
}
