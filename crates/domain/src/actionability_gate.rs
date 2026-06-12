use crate::{AssessmentHistoryPoint, DecisionPosture, TimeToRiskBucket};

const LEGACY_STRICT_PREPARE_P20D_THRESHOLD: f64 = 0.18;
const STRICT_PREPARE_P20D_THRESHOLD_RATIO: f64 = 0.60;
const STRICT_PREPARE_P20D_THRESHOLD_MIN: f64 = 0.12;
const LEGACY_STRICT_PREPARE_P60D_THRESHOLD: f64 = 0.45;
const STRICT_PREPARE_P60D_THRESHOLD_BUFFER: f64 = 0.04;
const STRICT_PREPARE_P60D_THRESHOLD_LIFT: f64 = 1.10;
const STRICT_PREPARE_P60D_THRESHOLD_MIN: f64 = 0.25;
const STRICT_PREPARE_PLATEAU_P20D_BUFFER: f64 = 0.10;
const STRICT_PREPARE_PLATEAU_P20D_MIN: f64 = 0.35;
const STRICT_PREPARE_PLATEAU_P20D_MAX: f64 = 0.45;
const STRICT_PREPARE_PLATEAU_RELAXED_P20D_BUFFER: f64 = 0.10;
const STRICT_PREPARE_PLATEAU_RELAXED_P20D_FLOOR_MIN: f64 = 0.45;
const STRICT_PREPARE_PLATEAU_P60D_THRESHOLD: f64 = 0.70;
const STRICT_PREPARE_PLATEAU_RELAXED_P60D_THRESHOLD: f64 = 0.65;
const STRICT_PREPARE_PLATEAU_OVERALL_FLOOR: f64 = 42.0;
const STRICT_PREPARE_PLATEAU_EXTERNAL_FLOOR: f64 = 32.0;
const STRICT_PREPARE_PLATEAU_RELAXED_EXTERNAL_FLOOR: f64 = 40.0;
const STRICT_PREPARE_WEEKS_TRIGGER_OVERALL_FLOOR: f64 = 51.5;
const STRICT_PREPARE_WEEKS_TRIGGER_EXTERNAL_FLOOR: f64 = 33.0;
const STRICT_WEEKS_TRIGGER_DOMINANT_P20D_FLOOR: f64 = 0.25;
const STRICT_WEEKS_TRIGGER_DOMINANT_P20D_SPREAD_FLOOR: f64 = 0.15;
const STRICT_WEEKS_TRIGGER_DOMINANT_OVERALL_FLOOR: f64 = 53.0;
const STRICT_WEEKS_TRIGGER_DOMINANT_EXTERNAL_FLOOR: f64 = 35.0;
const STRICT_HISTORY_HYSTERESIS_MONTHS_P20D_FLOOR: f64 = 0.35;
const STRICT_HISTORY_HYSTERESIS_MONTHS_P60D_FLOOR: f64 = 0.65;
const STRICT_HISTORY_HYSTERESIS_MONTHS_OVERALL_FLOOR: f64 = 43.0;
const STRICT_HISTORY_HYSTERESIS_MONTHS_EXTERNAL_FLOOR: f64 = 39.0;
const STRICT_HISTORY_HYSTERESIS_MONTHS_STRUCTURAL_CARRY_P20D_FLOOR: f64 = 0.25;
const STRICT_HISTORY_HYSTERESIS_MONTHS_STRUCTURAL_CARRY_P60D_FLOOR: f64 = 0.80;
const STRICT_HISTORY_HYSTERESIS_MONTHS_STRUCTURAL_CARRY_OVERALL_FLOOR: f64 = 43.5;
const STRICT_HISTORY_HYSTERESIS_MONTHS_STRUCTURAL_CARRY_EXTERNAL_FLOOR: f64 = 30.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ActionableGateThresholds {
    pub prepare_p60d: f64,
    pub hedge_p20d: f64,
    pub defend_p5d: f64,
    pub external_prepare_p20d: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActionableGateFloorHits {
    pub prepare: bool,
    pub hedge: bool,
    pub defend: bool,
}

pub fn actionable_warning_point(
    point: &AssessmentHistoryPoint,
    use_transitional_bridge: bool,
    thresholds: Option<ActionableGateThresholds>,
) -> bool {
    let strict_prepare_p20d_threshold = strict_prepare_p20d_threshold(thresholds);
    let strict_prepare_p60d_threshold = strict_prepare_p60d_threshold(thresholds);
    let strict_prepare_plateau_p20d_threshold = strict_prepare_plateau_p20d_threshold(thresholds);
    let strict_prepare_relaxed_plateau_p20d_threshold =
        strict_prepare_relaxed_plateau_p20d_threshold(thresholds);
    let strict_short_horizon_signal =
        matches!(
            point.posture,
            DecisionPosture::Hedge | DecisionPosture::Defend
        ) || (matches!(point.time_to_risk_bucket, TimeToRiskBucket::Now)
            && point.overall_score >= 60.0
            && point.p_5d >= 0.18)
            || (matches!(point.time_to_risk_bucket, TimeToRiskBucket::Weeks)
                && point.overall_score >= 58.0
                && point.p_20d >= 0.25
                && point.external_shock_score >= 44.0);

    let high_probability_prepare_signal = matches!(point.posture, DecisionPosture::Prepare)
        && point.p_20d >= strict_prepare_p20d_threshold
        && point.p_60d >= strict_prepare_p60d_threshold
        && ((point.overall_score >= 60.0 && point.external_shock_score >= 46.0)
            || (point.overall_score >= 53.0
                && !matches!(point.time_to_risk_bucket, TimeToRiskBucket::Normal)
                && strong_prepare_trigger_code(point)));
    let probability_plateau_prepare_setup = matches!(point.posture, DecisionPosture::Prepare)
        && matches!(point.time_to_risk_bucket, TimeToRiskBucket::Months)
        && probability_plateau_trigger_code(point);
    let standard_probability_plateau_prepare_signal = probability_plateau_prepare_setup
        && point.p_20d >= strict_prepare_plateau_p20d_threshold
        && point.p_60d >= strict_prepare_p60d_threshold.max(STRICT_PREPARE_PLATEAU_P60D_THRESHOLD)
        && point.overall_score >= STRICT_PREPARE_PLATEAU_OVERALL_FLOOR
        && point.external_shock_score >= STRICT_PREPARE_PLATEAU_EXTERNAL_FLOOR;
    let relaxed_probability_plateau_prepare_signal = probability_plateau_prepare_setup
        && point.p_20d >= strict_prepare_relaxed_plateau_p20d_threshold
        && point.p_60d >= STRICT_PREPARE_PLATEAU_RELAXED_P60D_THRESHOLD
        && point.overall_score >= STRICT_PREPARE_PLATEAU_OVERALL_FLOOR
        && point.external_shock_score >= STRICT_PREPARE_PLATEAU_RELAXED_EXTERNAL_FLOOR;
    let weeks_trigger_dominant_signal = actionable_weeks_trigger_dominant_signal(
        point,
        thresholds,
        strict_prepare_p20d_threshold,
        strict_prepare_p60d_threshold,
    );
    let prepare_weeks_plateau_hysteresis_signal =
        actionable_prepare_weeks_plateau_hysteresis_signal(point, thresholds);
    let high_probability_months_signal =
        matches!(point.time_to_risk_bucket, TimeToRiskBucket::Months)
            && point.overall_score >= 62.0
            && point.p_20d >= strict_prepare_p20d_threshold
            && point.p_60d >= strict_prepare_p60d_threshold
            && point.external_shock_score >= 48.0;
    let history_hysteresis_months_signal =
        matches!(point.time_to_risk_bucket, TimeToRiskBucket::Months)
            && history_hysteresis_trigger_code(point)
            && point.p_20d >= STRICT_HISTORY_HYSTERESIS_MONTHS_P20D_FLOOR
            && point.p_60d
                >= strict_prepare_p60d_threshold.max(STRICT_HISTORY_HYSTERESIS_MONTHS_P60D_FLOOR)
            && (point.overall_score >= STRICT_HISTORY_HYSTERESIS_MONTHS_OVERALL_FLOOR
                || point.external_shock_score >= STRICT_HISTORY_HYSTERESIS_MONTHS_EXTERNAL_FLOOR);
    let history_hysteresis_months_structural_carry_signal =
        actionable_history_hysteresis_months_structural_carry_signal(
            point,
            thresholds,
            strict_prepare_p60d_threshold,
        );

    let prepare_bridge_signal = use_transitional_bridge
        && matches!(point.posture, DecisionPosture::Prepare)
        && point.overall_score >= 58.0
        && point.external_shock_score >= 46.0;
    let months_bridge_signal = use_transitional_bridge
        && matches!(point.time_to_risk_bucket, TimeToRiskBucket::Months)
        && point.overall_score >= 58.0
        && point.external_shock_score >= 42.0;

    strict_short_horizon_signal
        || high_probability_prepare_signal
        || standard_probability_plateau_prepare_signal
        || relaxed_probability_plateau_prepare_signal
        || weeks_trigger_dominant_signal
        || prepare_weeks_plateau_hysteresis_signal
        || high_probability_months_signal
        || history_hysteresis_months_signal
        || history_hysteresis_months_structural_carry_signal
        || prepare_bridge_signal
        || months_bridge_signal
}

pub fn actionable_runtime_floor_hits(
    point: &AssessmentHistoryPoint,
    thresholds: Option<ActionableGateThresholds>,
) -> Option<ActionableGateFloorHits> {
    let thresholds = thresholds?;
    Some(ActionableGateFloorHits {
        prepare: point.p_60d >= thresholds.prepare_p60d,
        hedge: point.p_20d >= thresholds.hedge_p20d,
        defend: point.p_5d >= thresholds.defend_p5d,
    })
}

pub fn actionable_runtime_floor_reached(
    point: &AssessmentHistoryPoint,
    thresholds: Option<ActionableGateThresholds>,
) -> bool {
    actionable_runtime_floor_hits(point, thresholds)
        .is_some_and(|hits| hits.prepare || hits.hedge || hits.defend)
}

pub fn weak_defend_only_runtime_floor(
    point: &AssessmentHistoryPoint,
    thresholds: Option<ActionableGateThresholds>,
) -> bool {
    actionable_runtime_floor_hits(point, thresholds).is_some_and(|hits| {
        hits.defend
            && !hits.hedge
            && !hits.prepare
            && matches!(point.posture, DecisionPosture::Normal)
            && matches!(point.time_to_risk_bucket, TimeToRiskBucket::Normal)
            && point.posture_trigger_codes.is_empty()
    })
}

pub fn strict_prepare_p20d_threshold(thresholds: Option<ActionableGateThresholds>) -> f64 {
    thresholds
        .map(|thresholds| {
            (thresholds.external_prepare_p20d * STRICT_PREPARE_P20D_THRESHOLD_RATIO).clamp(
                STRICT_PREPARE_P20D_THRESHOLD_MIN,
                LEGACY_STRICT_PREPARE_P20D_THRESHOLD,
            )
        })
        .unwrap_or(LEGACY_STRICT_PREPARE_P20D_THRESHOLD)
}

pub fn strict_prepare_p60d_threshold(thresholds: Option<ActionableGateThresholds>) -> f64 {
    thresholds
        .map(|thresholds| {
            (thresholds.prepare_p60d + STRICT_PREPARE_P60D_THRESHOLD_BUFFER)
                .max(thresholds.prepare_p60d * STRICT_PREPARE_P60D_THRESHOLD_LIFT)
                .clamp(
                    STRICT_PREPARE_P60D_THRESHOLD_MIN,
                    LEGACY_STRICT_PREPARE_P60D_THRESHOLD,
                )
        })
        .unwrap_or(LEGACY_STRICT_PREPARE_P60D_THRESHOLD)
}

pub fn actionable_prepare_weeks_score_confirmation_gap(
    point: &AssessmentHistoryPoint,
    thresholds: Option<ActionableGateThresholds>,
) -> bool {
    has_prepare_weeks_score_confirmation_setup(point)
        && point.p_20d >= strict_prepare_p20d_threshold(thresholds)
        && point.p_60d >= strict_prepare_p60d_threshold(thresholds)
        && !actionable_prepare_weeks_plateau_hysteresis_signal(point, thresholds)
        && point.overall_score < 53.0
}

pub fn strong_prepare_trigger_code(point: &AssessmentHistoryPoint) -> bool {
    point.posture_trigger_codes.iter().any(|code| {
        matches!(
            code.as_str(),
            "prepare_p60d_structural"
                | "prepare_structural_downgrade"
                | "prepare_carry_structural"
                | "prepare_external_structural"
                | "prepare_continuity_bridge"
                | "prepare_history_hysteresis"
                | "prepare_probability_plateau"
        )
    })
}

pub fn probability_plateau_trigger_code(point: &AssessmentHistoryPoint) -> bool {
    point
        .posture_trigger_codes
        .iter()
        .any(|code| code == "prepare_probability_plateau")
}

pub fn history_hysteresis_trigger_code(point: &AssessmentHistoryPoint) -> bool {
    point
        .posture_trigger_codes
        .iter()
        .any(|code| code == "prepare_history_hysteresis")
}

fn strict_prepare_plateau_p20d_threshold(thresholds: Option<ActionableGateThresholds>) -> f64 {
    thresholds
        .map(|thresholds| {
            (thresholds.hedge_p20d + STRICT_PREPARE_PLATEAU_P20D_BUFFER).clamp(
                STRICT_PREPARE_PLATEAU_P20D_MIN,
                STRICT_PREPARE_PLATEAU_P20D_MAX,
            )
        })
        .unwrap_or(STRICT_PREPARE_PLATEAU_P20D_MAX)
}

fn strict_prepare_relaxed_plateau_p20d_threshold(
    thresholds: Option<ActionableGateThresholds>,
) -> f64 {
    (strict_prepare_plateau_p20d_threshold(thresholds) + STRICT_PREPARE_PLATEAU_RELAXED_P20D_BUFFER)
        .max(STRICT_PREPARE_PLATEAU_RELAXED_P20D_FLOOR_MIN)
}

fn has_prepare_weeks_score_confirmation_setup(point: &AssessmentHistoryPoint) -> bool {
    matches!(point.posture, DecisionPosture::Prepare)
        && matches!(point.time_to_risk_bucket, TimeToRiskBucket::Weeks)
        && (probability_plateau_trigger_code(point) || history_hysteresis_trigger_code(point))
}

fn has_prepare_weeks_plateau_hysteresis_setup(point: &AssessmentHistoryPoint) -> bool {
    matches!(point.posture, DecisionPosture::Prepare)
        && matches!(point.time_to_risk_bucket, TimeToRiskBucket::Weeks)
        && probability_plateau_trigger_code(point)
        && history_hysteresis_trigger_code(point)
}

fn actionable_prepare_weeks_plateau_hysteresis_signal(
    point: &AssessmentHistoryPoint,
    thresholds: Option<ActionableGateThresholds>,
) -> bool {
    has_prepare_weeks_plateau_hysteresis_setup(point)
        && point.p_20d >= strict_prepare_relaxed_plateau_p20d_threshold(thresholds)
        && point.p_60d >= STRICT_PREPARE_PLATEAU_RELAXED_P60D_THRESHOLD
        && point.overall_score >= STRICT_PREPARE_WEEKS_TRIGGER_OVERALL_FLOOR
        && point.external_shock_score >= STRICT_PREPARE_WEEKS_TRIGGER_EXTERNAL_FLOOR
}

fn actionable_weeks_trigger_dominant_signal(
    point: &AssessmentHistoryPoint,
    thresholds: Option<ActionableGateThresholds>,
    strict_prepare_p20d_threshold: f64,
    strict_prepare_p60d_threshold: f64,
) -> bool {
    thresholds.is_some()
        && matches!(point.time_to_risk_bucket, TimeToRiskBucket::Weeks)
        && point.p_20d
            >= strict_prepare_p20d_threshold.max(STRICT_WEEKS_TRIGGER_DOMINANT_P20D_FLOOR)
        && point.p_60d < strict_prepare_p60d_threshold
        && (point.p_20d - point.p_60d) >= STRICT_WEEKS_TRIGGER_DOMINANT_P20D_SPREAD_FLOOR
        && point.overall_score >= STRICT_WEEKS_TRIGGER_DOMINANT_OVERALL_FLOOR
        && point.external_shock_score >= STRICT_WEEKS_TRIGGER_DOMINANT_EXTERNAL_FLOOR
}

fn actionable_history_hysteresis_months_structural_carry_signal(
    point: &AssessmentHistoryPoint,
    thresholds: Option<ActionableGateThresholds>,
    strict_prepare_p60d_threshold: f64,
) -> bool {
    thresholds.is_some()
        && matches!(point.posture, DecisionPosture::Prepare)
        && matches!(point.time_to_risk_bucket, TimeToRiskBucket::Months)
        && history_hysteresis_trigger_code(point)
        && point.p_20d >= STRICT_HISTORY_HYSTERESIS_MONTHS_STRUCTURAL_CARRY_P20D_FLOOR
        && point.p_60d
            >= strict_prepare_p60d_threshold
                .max(STRICT_HISTORY_HYSTERESIS_MONTHS_STRUCTURAL_CARRY_P60D_FLOOR)
        && point.overall_score >= STRICT_HISTORY_HYSTERESIS_MONTHS_STRUCTURAL_CARRY_OVERALL_FLOOR
        && point.external_shock_score
            >= STRICT_HISTORY_HYSTERESIS_MONTHS_STRUCTURAL_CARRY_EXTERNAL_FLOOR
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;

    use super::*;

    fn base_point() -> AssessmentHistoryPoint {
        AssessmentHistoryPoint {
            as_of_date: NaiveDate::from_ymd_opt(2024, 1, 5).expect("date"),
            overall_score: 40.0,
            p_5d: 0.01,
            p_20d: 0.01,
            p_60d: 0.01,
            raw_p_5d: None,
            raw_p_20d: None,
            raw_p_60d: None,
            posture: DecisionPosture::Normal,
            time_to_risk_bucket: TimeToRiskBucket::Normal,
            external_shock_score: 20.0,
            posture_trigger_codes: Vec::new(),
            posture_blocker_codes: Vec::new(),
            replay_run_id: None,
            feature_snapshot_id: None,
            history_source: None,
        }
    }

    #[test]
    fn strict_prepare_p20d_threshold_respects_formal_runtime_floor() {
        let thresholds = ActionableGateThresholds {
            prepare_p60d: 0.12,
            hedge_p20d: 0.06,
            defend_p5d: 0.05,
            external_prepare_p20d: 0.042,
        };

        assert_eq!(strict_prepare_p20d_threshold(Some(thresholds)), 0.12);
    }

    #[test]
    fn actionable_warning_point_accepts_prepare_weeks_plateau_hysteresis() {
        let mut point = base_point();
        point.posture = DecisionPosture::Prepare;
        point.time_to_risk_bucket = TimeToRiskBucket::Weeks;
        point.p_20d = 0.45;
        point.p_60d = 0.66;
        point.overall_score = 52.0;
        point.external_shock_score = 33.0;
        point.posture_trigger_codes = vec![
            "prepare_probability_plateau".to_string(),
            "prepare_history_hysteresis".to_string(),
        ];

        let thresholds = ActionableGateThresholds {
            prepare_p60d: 0.12,
            hedge_p20d: 0.06,
            defend_p5d: 0.05,
            external_prepare_p20d: 0.042,
        };

        assert!(actionable_warning_point(&point, false, Some(thresholds)));
    }

    #[test]
    fn weak_defend_only_runtime_floor_ignores_non_normal_posture() {
        let mut point = base_point();
        point.p_5d = 0.08;
        point.posture = DecisionPosture::Hedge;

        let thresholds = ActionableGateThresholds {
            prepare_p60d: 0.12,
            hedge_p20d: 0.06,
            defend_p5d: 0.05,
            external_prepare_p20d: 0.042,
        };

        assert!(!weak_defend_only_runtime_floor(&point, Some(thresholds)));
    }
}
