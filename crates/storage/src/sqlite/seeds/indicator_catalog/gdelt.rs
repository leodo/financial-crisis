use fc_domain::RiskDirection;

use super::GdeltIndicatorSeed;

pub(super) fn gdelt_indicator_seeds() -> Vec<GdeltIndicatorSeed> {
    vec![GdeltIndicatorSeed {
        indicator_id: "global_news_financial_stress_count",
        display_name: "金融压力新闻数量",
        description:
            "Daily GDELT DOC API count for banking, liquidity, funding, and credit-stress coverage.",
        unit: "count",
        risk_direction: RiskDirection::HigherIsRiskier,
    }]
}
