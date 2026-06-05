pub(in super::super) fn release_review_primary_failure_mode(
    dominant_blocks: &[String],
    dominant_block_count: u32,
    dominant_facets: &[String],
    dominant_facet_count: u32,
) -> Option<String> {
    if dominant_block_count > 0 {
        if dominant_blocks
            .iter()
            .any(|category| category == "review_gate_gap")
        {
            return Some("strict_gate_mismatch".to_string());
        }
        if dominant_blocks
            .iter()
            .any(|category| category == "posture_bucket_normal")
        {
            return Some("posture_continuity_failure".to_string());
        }
        if dominant_blocks
            .iter()
            .any(|category| category.ends_with("score_confirmation"))
        {
            return Some("score_confirmation_failure".to_string());
        }
        if dominant_blocks
            .iter()
            .any(|category| category.ends_with("bridge_not_armed"))
        {
            return Some("transitional_bridge_failure".to_string());
        }
        return Some("residual_review_l3_failure".to_string());
    }

    if dominant_facet_count > 0 {
        if dominant_facets
            .iter()
            .any(|facet| facet == "posture:normal")
            || dominant_facets.iter().any(|facet| facet == "bucket:normal")
            || dominant_facets.iter().any(|facet| facet == "trigger:none")
        {
            return Some("posture_continuity_failure".to_string());
        }
        if dominant_facets
            .iter()
            .any(|facet| facet.starts_with("gate_gap:") && facet != "gate_gap:none")
        {
            return Some("strict_gate_mismatch".to_string());
        }
        if dominant_facets.iter().any(|facet| {
            facet.starts_with("confirmation:") && facet != "confirmation:ok_or_not_needed"
        }) {
            return Some("score_confirmation_failure".to_string());
        }
        return Some("runtime_continuity_failure".to_string());
    }

    None
}
