const TEXT_KEYWORDS: &[&str] = &[
    "liquidity",
    "funding",
    "deposit",
    "capital",
    "downgrade",
    "restructuring",
    "bankruptcy",
    "supervisory",
    "material weakness",
    "going concern",
];

const RISKY_ITEM_CODES: &[(&str, f64)] = &[
    ("1.03", 28.0),
    ("2.03", 18.0),
    ("2.04", 24.0),
    ("2.05", 12.0),
    ("2.06", 14.0),
    ("3.01", 15.0),
    ("3.03", 6.0),
    ("4.02", 22.0),
    ("8.01", 4.0),
];

pub(super) fn relevant_form_bucket(form: &str) -> Option<&'static str> {
    let upper = form.to_ascii_uppercase();
    if upper.starts_with("8-K") {
        Some("8-K")
    } else if upper.starts_with("10-Q") {
        Some("10-Q")
    } else if upper.starts_with("10-K") {
        Some("10-K")
    } else {
        None
    }
}

pub(super) fn filing_severity(
    importance: u8,
    form_bucket: &str,
    keyword_hits: &[String],
    rule_hits: &[String],
) -> f64 {
    let base = match form_bucket {
        "8-K" => 12.0,
        "10-Q" => 6.0,
        "10-K" => 5.0,
        _ => 0.0,
    };
    let importance_boost = match importance {
        3 => 8.0,
        2 => 5.0,
        _ => 3.0,
    };
    let keyword_boost = if keyword_hits.is_empty() {
        0.0
    } else {
        (12.0 + (keyword_hits.len().saturating_sub(1) as f64 * 4.0)).min(20.0)
    };
    let rule_boost = rule_hits
        .iter()
        .filter_map(|label| {
            label
                .strip_prefix("item_")
                .and_then(|code| {
                    RISKY_ITEM_CODES
                        .iter()
                        .find(|(candidate, _)| *candidate == code)
                })
                .map(|(_, boost)| *boost)
        })
        .fold(0.0_f64, f64::max);

    round1((base + importance_boost + keyword_boost + rule_boost).clamp(0.0, 100.0))
}

pub(super) fn keyword_hits(text: &str) -> Vec<String> {
    TEXT_KEYWORDS
        .iter()
        .filter(|keyword| text.contains(**keyword))
        .map(|keyword| (*keyword).to_string())
        .collect()
}

pub(super) fn item_rule_hits(items: &[String]) -> Vec<String> {
    items
        .iter()
        .filter_map(|item| {
            RISKY_ITEM_CODES
                .iter()
                .find(|(code, _)| *code == item.as_str())
                .map(|(code, _)| format!("item_{code}"))
        })
        .collect()
}

pub(super) fn split_codes(raw: &str) -> Vec<String> {
    raw.split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(|item| item.to_string())
        .collect()
}

pub(super) fn round1(value: f64) -> f64 {
    (value * 10.0).round() / 10.0
}
