use fc_domain::{
    AlertEvent, EventAssessment, EventConfirmationState, EventSignalSummary, RiskDimension,
    RiskSnapshot,
};

use super::super::round1;

pub(in super::super) fn build_event_assessment(
    snapshot: &RiskSnapshot,
    alerts: &[AlertEvent],
) -> EventAssessment {
    let recent_event_count = alerts.len() as u32;
    let recent_events = alerts
        .iter()
        .take(4)
        .map(|alert| EventSignalSummary {
            event_type: alert.event_type,
            level: alert.level,
            triggered_as_of_date: alert.triggered_as_of_date,
            trigger_reason: alert.trigger_reason.clone(),
            related_indicators: alert.related_indicators.clone(),
        })
        .collect::<Vec<_>>();
    let confirmation_score = round1(
        (snapshot
            .dimensions
            .iter()
            .find(|dimension| dimension.dimension == RiskDimension::EventsSentiment)
            .map(|dimension| dimension.score)
            .unwrap_or(0.0)
            * 0.7
            + recent_event_count as f64 * 9.0)
            .clamp(0.0, 100.0),
    );
    let state = if confirmation_score >= 70.0 {
        EventConfirmationState::Escalating
    } else if confirmation_score >= 55.0 {
        EventConfirmationState::Confirmed
    } else if confirmation_score >= 30.0 {
        EventConfirmationState::Watching
    } else {
        EventConfirmationState::Quiet
    };

    let confirmed_signals = alerts
        .iter()
        .map(|alert| alert.trigger_reason.clone())
        .take(3)
        .collect::<Vec<_>>();
    let mut pending_gaps = Vec::new();
    if recent_event_count == 0 {
        pending_gaps.push("事件层还没有给出足够确认，当前更多依赖价格和宏观层信号。".to_string());
    }
    if snapshot.trigger_score >= 60.0 && recent_event_count < 2 {
        pending_gaps.push("触发层已抬升，但银行/公告/新闻事件还没有形成更强共振。".to_string());
    }

    let summary = match state {
        EventConfirmationState::Quiet => {
            "事件层暂时安静，当前风险判断主要来自价格和融资信号。".to_string()
        }
        EventConfirmationState::Watching => {
            "事件层开始出现支持证据，但还不足以单独驱动强结论。".to_string()
        }
        EventConfirmationState::Confirmed => {
            "事件层已经提供了实质性确认，当前风险判断不再只是市场噪声。".to_string()
        }
        EventConfirmationState::Escalating => {
            "事件层与市场层正在同步升级，需优先防范短期风险压缩。".to_string()
        }
    };

    EventAssessment {
        state,
        confirmation_score,
        recent_event_count,
        summary,
        confirmed_signals,
        pending_gaps,
        recent_events,
    }
}
