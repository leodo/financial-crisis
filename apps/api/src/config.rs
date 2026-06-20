use std::{env, net::SocketAddr};

use serde::Serialize;

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub bind_addr: SocketAddr,
    pub refresh_interval_seconds: u64,
    pub default_history_points: usize,
    pub max_history_points: usize,
}

impl AppConfig {
    pub fn from_env() -> Self {
        let bind_addr = env::var("FC_API_BIND")
            .unwrap_or_else(|_| "127.0.0.1:18080".to_string())
            .parse()
            .expect("FC_API_BIND must be a socket address");
        let refresh_interval_seconds = env::var("FC_API_REFRESH_SECONDS")
            .ok()
            .and_then(|value| value.parse::<u64>().ok())
            .unwrap_or(60);
        let default_history_points = env::var("FC_DEFAULT_HISTORY_POINTS")
            .ok()
            .and_then(|value| value.parse::<usize>().ok())
            .unwrap_or(260);
        let max_history_points = env::var("FC_MAX_HISTORY_POINTS")
            .ok()
            .and_then(|value| value.parse::<usize>().ok())
            .unwrap_or(2000)
            .max(default_history_points);
        Self {
            bind_addr,
            refresh_interval_seconds,
            default_history_points,
            max_history_points,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct RiskAlertThresholds {
    pub overall_score: f64,
    pub trigger_score: f64,
    pub min_posture: String,
    pub max_production_source_issues: usize,
    pub alert_on_reference_only: bool,
    pub source: &'static str,
}

impl RiskAlertThresholds {
    pub fn from_env() -> Self {
        Self {
            overall_score: number_env("FC_RISK_ALERT_OVERALL_SCORE", 55.0),
            trigger_score: number_env("FC_RISK_ALERT_TRIGGER_SCORE", 60.0),
            min_posture: env::var("FC_RISK_ALERT_MIN_POSTURE")
                .unwrap_or_else(|_| "prepare".to_string()),
            max_production_source_issues: usize_env("FC_RISK_ALERT_MAX_SOURCE_ISSUES", 0),
            alert_on_reference_only: bool_env("FC_RISK_ALERT_ON_REFERENCE_ONLY", false),
            source: "api_env",
        }
    }
}

fn number_env(name: &str, fallback: f64) -> f64 {
    env::var(name)
        .ok()
        .and_then(|value| value.parse::<f64>().ok())
        .filter(|value| value.is_finite())
        .unwrap_or(fallback)
}

fn usize_env(name: &str, fallback: usize) -> usize {
    env::var(name)
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(fallback)
}

fn bool_env(name: &str, fallback: bool) -> bool {
    env::var(name)
        .ok()
        .map(|value| {
            matches!(
                value.to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
        .unwrap_or(fallback)
}
