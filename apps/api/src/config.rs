use std::{env, net::SocketAddr};

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
