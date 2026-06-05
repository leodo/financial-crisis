use fc_domain::RiskDirection;

use super::SecEventIndicatorSeed;

pub(super) fn sec_event_indicator_seeds() -> Vec<SecEventIndicatorSeed> {
    vec![
        SecEventIndicatorSeed {
            indicator_id: "us_event_bank_8k_count",
            display_name: "白名单银行 8-K 数量",
            description: "Daily count of 8-K filings from the SEC EDGAR bank watchlist.",
            unit: "count",
            risk_direction: RiskDirection::ManualRule,
        },
        SecEventIndicatorSeed {
            indicator_id: "us_event_risk_keyword_count",
            display_name: "SEC 风险关键词/规则命中数",
            description:
                "Daily count of SEC filing metadata keyword hits plus high-risk 8-K item rule matches.",
            unit: "count",
            risk_direction: RiskDirection::ManualRule,
        },
        SecEventIndicatorSeed {
            indicator_id: "us_banking_filing_stress_count",
            display_name: "银行 filing 压力计数",
            description:
                "Daily count of filings whose rule-based severity passes the stress threshold.",
            unit: "count",
            risk_direction: RiskDirection::ManualRule,
        },
        SecEventIndicatorSeed {
            indicator_id: "us_event_official_filing_severity",
            display_name: "SEC 官方公告严重度",
            description:
                "Daily severity index aggregated from SEC filing form types, items, and watchlist breadth.",
            unit: "score",
            risk_direction: RiskDirection::ManualRule,
        },
    ]
}
