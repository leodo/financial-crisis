pub(super) fn posture_confirmation_count(
    trigger_score: f64,
    external_shock_score: f64,
    event_confirmation_score: f64,
) -> u8 {
    [
        trigger_score >= 60.0,
        external_shock_score >= 55.0,
        event_confirmation_score >= 55.0,
    ]
    .into_iter()
    .filter(|flag| *flag)
    .count() as u8
}

pub(super) fn prepare_context_confirmation_count(
    trigger_score: f64,
    external_shock_score: f64,
    breadth_score: f64,
    event_confirmation_score: f64,
    carry_funding_pressure_score: f64,
) -> u8 {
    [
        trigger_score >= 45.0,
        external_shock_score >= 50.0,
        breadth_score >= 36.0,
        event_confirmation_score >= 38.0,
        carry_funding_pressure_score >= 48.0,
    ]
    .into_iter()
    .filter(|flag| *flag)
    .count() as u8
}

pub(super) fn prepare_context_confirmation_count_without_events(
    trigger_score: f64,
    external_shock_score: f64,
    breadth_score: f64,
    carry_funding_pressure_score: f64,
) -> u8 {
    [
        trigger_score >= 45.0,
        external_shock_score >= 50.0,
        breadth_score >= 36.0,
        carry_funding_pressure_score >= 48.0,
    ]
    .into_iter()
    .filter(|flag| *flag)
    .count() as u8
}

pub(super) fn prepare_non_external_confirmation_count(
    trigger_score: f64,
    breadth_score: f64,
    event_confirmation_score: f64,
    carry_funding_pressure_score: f64,
) -> u8 {
    [
        trigger_score >= 45.0,
        breadth_score >= 36.0,
        event_confirmation_score >= 38.0,
        carry_funding_pressure_score >= 48.0,
    ]
    .into_iter()
    .filter(|flag| *flag)
    .count() as u8
}

pub(super) fn prepare_non_external_confirmation_count_without_events(
    trigger_score: f64,
    breadth_score: f64,
    carry_funding_pressure_score: f64,
) -> u8 {
    [
        trigger_score >= 45.0,
        breadth_score >= 36.0,
        carry_funding_pressure_score >= 48.0,
    ]
    .into_iter()
    .filter(|flag| *flag)
    .count() as u8
}

pub(super) fn prepare_non_carry_confirmation_count(
    trigger_score: f64,
    external_shock_score: f64,
    breadth_score: f64,
    event_confirmation_score: f64,
) -> u8 {
    [
        trigger_score >= 45.0,
        external_shock_score >= 50.0,
        breadth_score >= 36.0,
        event_confirmation_score >= 38.0,
    ]
    .into_iter()
    .filter(|flag| *flag)
    .count() as u8
}

pub(super) fn prepare_non_carry_confirmation_count_without_events(
    trigger_score: f64,
    external_shock_score: f64,
    breadth_score: f64,
) -> u8 {
    [
        trigger_score >= 45.0,
        external_shock_score >= 50.0,
        breadth_score >= 36.0,
    ]
    .into_iter()
    .filter(|flag| *flag)
    .count() as u8
}
