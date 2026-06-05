use super::super::ReleaseReviewRuntimeSeparationComparison;

pub(crate) fn release_review_runtime_separation_takeaways(
    rows: &[ReleaseReviewRuntimeSeparationComparison],
) -> Vec<String> {
    let mut takeaways = Vec::new();
    for row in rows {
        if !matches!(row.horizon_days, 20 | 60) {
            continue;
        }
        match row.candidate_diagnosis.as_str() {
            "separated_but_below_runtime_floor" => {
                takeaways.push(format!(
                    "{}d: candidate 的 {} 已经和 normal 拉开，但 early-warning 平均概率 {} 仍低于 runtime floor {}（floor gap {}）。这更像阈值 / runtime policy 瓶颈，不是完全没有信号。",
                    row.horizon_days,
                    row.candidate_early_warning_regime,
                    crate::format_optional_pct(row.candidate_early_warning_avg_probability),
                    crate::format_optional_pct(row.candidate_threshold),
                    crate::format_optional_pct(row.candidate_floor_gap),
                ));
            }
            "usable_early_warning_separation" => {
                if row.baseline_diagnosis == "separated_but_below_runtime_floor" {
                    takeaways.push(format!(
                        "{}d: candidate 已把 early-warning 平均概率 {} 推过 runtime floor {}（floor gap {}），说明这一窗的主瓶颈已不再是 runtime threshold 本身。",
                        row.horizon_days,
                        crate::format_optional_pct(row.candidate_early_warning_avg_probability),
                        crate::format_optional_pct(row.candidate_threshold),
                        crate::format_optional_pct(row.candidate_floor_gap),
                    ));
                }
            }
            "cooldown_bleed" => {
                takeaways.push(format!(
                    "{}d: candidate 仍有 cooldown bleed，说明 post-crisis cooldown 段概率抬得过高，容易把危机后的背景值误当成提前预警。",
                    row.horizon_days
                ));
            }
            "late_only_no_early_warning" => {
                takeaways.push(format!(
                    "{}d: candidate 只有晚到信号，没有形成可用的 early-warning separation，当前还谈不上可执行提前量。",
                    row.horizon_days
                ));
            }
            _ => {}
        }
    }
    takeaways
}
