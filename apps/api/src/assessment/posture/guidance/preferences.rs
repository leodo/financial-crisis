use fc_domain::{
    DecisionPosture, EventAssessment, EventConfirmationState, UserRiskPreferences, UserRiskProfile,
};

pub(super) fn adjust_posture_for_preferences(
    base_posture: DecisionPosture,
    user_preferences: &UserRiskPreferences,
    event_assessment: &EventAssessment,
) -> DecisionPosture {
    match user_preferences.profile {
        UserRiskProfile::Conservative => escalate_posture(base_posture),
        UserRiskProfile::Aggressive => {
            if matches!(
                event_assessment.state,
                EventConfirmationState::Quiet | EventConfirmationState::Watching
            ) {
                deescalate_posture(base_posture)
            } else {
                base_posture
            }
        }
        UserRiskProfile::Neutral => base_posture,
    }
}

pub(super) fn preference_adjustment_code(user_preferences: &UserRiskPreferences) -> &'static str {
    match user_preferences.profile {
        UserRiskProfile::Conservative => "preference_conservative_escalation",
        UserRiskProfile::Aggressive => "preference_aggressive_deescalation",
        UserRiskProfile::Neutral => "preference_neutral_no_adjustment",
    }
}

fn escalate_posture(posture: DecisionPosture) -> DecisionPosture {
    match posture {
        DecisionPosture::Normal => DecisionPosture::Prepare,
        DecisionPosture::Prepare => DecisionPosture::Hedge,
        DecisionPosture::Hedge | DecisionPosture::Defend => DecisionPosture::Defend,
    }
}

fn deescalate_posture(posture: DecisionPosture) -> DecisionPosture {
    match posture {
        DecisionPosture::Defend => DecisionPosture::Hedge,
        DecisionPosture::Hedge => DecisionPosture::Prepare,
        DecisionPosture::Prepare | DecisionPosture::Normal => DecisionPosture::Normal,
    }
}
