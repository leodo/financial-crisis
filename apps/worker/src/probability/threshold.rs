mod calibration;
mod decision;
mod diagnostics;

pub(crate) use calibration::{
    probability_calibration_selection_rows, select_probability_calibration_strategy,
};
pub(crate) use decision::{
    adjust_probability_decision_threshold_for_regime_support,
    probability_decision_threshold_selection, select_probability_decision_threshold,
};
pub(crate) use diagnostics::build_probability_threshold_diagnostics;

#[derive(Debug, Clone)]
pub(crate) struct ProbabilityCalibrationSelection<'a> {
    pub(crate) rows: Vec<&'a crate::ProbabilityTrainingRow>,
    pub(crate) eligible_row_count: usize,
    pub(crate) eligible_positive_count: usize,
    pub(crate) eligible_negative_count: usize,
    pub(crate) used_full_split_fallback: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct ProbabilityThresholdSelection<'a> {
    pub(crate) rows: Vec<&'a crate::ProbabilityTrainingRow>,
    pub(crate) probabilities: Vec<f64>,
    pub(crate) labels: Vec<f64>,
    pub(crate) used_full_split_fallback: bool,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct ProbabilityThresholdDiagnosticsInput<'a> {
    pub(crate) full_calibration_rows: &'a [crate::ProbabilityTrainingRow],
    pub(crate) calibration_selection: &'a ProbabilityCalibrationSelection<'a>,
    pub(crate) threshold_selection: &'a ProbabilityThresholdSelection<'a>,
    pub(crate) horizon_days: u32,
    pub(crate) label_mode: crate::ProbabilityTargetLabelMode,
    pub(crate) base_threshold: f64,
    pub(crate) final_threshold: f64,
}

#[derive(Debug, Clone, Copy, Default)]
pub(super) struct ProbabilityThresholdDecisionMetrics {
    regime_hits: ProbabilityThresholdRegimeHitSummary,
    predicted_positive_count: u32,
    true_positive_count: u32,
    precision: f64,
    recall: f64,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct ProbabilityThresholdScoreInputs {
    horizon_days: u32,
    precision: f64,
    recall: f64,
    f_beta: f64,
    threshold: f64,
    predicted_positive_count: u32,
    prediction_ceiling: u32,
    actual_positive_count: u32,
}

#[derive(Debug, Clone, Copy, Default)]
pub(super) struct ProbabilityThresholdRegimeHitSummary {
    early_warning_row_count: u32,
    early_warning_hit_count: u32,
    normal_row_count: u32,
    normal_hit_count: u32,
    positive_window_row_count: u32,
    positive_window_hit_count: u32,
    in_crisis_row_count: u32,
    in_crisis_hit_count: u32,
    cooldown_row_count: u32,
    cooldown_hit_count: u32,
}

#[derive(Debug, Clone, Copy, Default)]
pub(super) struct ProbabilityCalibrationRegimeEvidenceBucket {
    full_row_count: u32,
    calibration_eligible_row_count: u32,
    calibration_used_row_count: u32,
    threshold_selected_row_count: u32,
    positive_label_count: u32,
    hard_label_sum: f64,
    training_target_sum: f64,
    objective_weight_sum: f64,
    protected_action_window_count: u32,
    episode_native_objective_row_count: u32,
    protected_no_positive_main_row_count: u32,
    protected_no_positive_main_training_target_sum: f64,
    protected_no_positive_main_objective_weight_sum: f64,
}

#[cfg(test)]
mod tests {
    use super::{decision::probability_threshold_score_tuple, ProbabilityThresholdScoreInputs};

    #[test]
    fn twenty_day_threshold_score_penalizes_overbroad_thresholds() {
        let restrained = probability_threshold_score_tuple(ProbabilityThresholdScoreInputs {
            horizon_days: 20,
            precision: 0.19,
            recall: 0.30,
            f_beta: 0.23,
            threshold: 0.48,
            predicted_positive_count: 80,
            prediction_ceiling: 40,
            actual_positive_count: 10,
        });
        let overbroad = probability_threshold_score_tuple(ProbabilityThresholdScoreInputs {
            horizon_days: 20,
            precision: 0.18,
            recall: 0.35,
            f_beta: 0.232,
            threshold: 0.44,
            predicted_positive_count: 120,
            prediction_ceiling: 40,
            actual_positive_count: 10,
        });

        assert!(restrained > overbroad);
    }
}
