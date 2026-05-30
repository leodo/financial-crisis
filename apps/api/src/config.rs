use std::{env, net::SocketAddr};

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub bind_addr: SocketAddr,
}

impl AppConfig {
    pub fn from_env() -> Self {
        let bind_addr = env::var("FC_API_BIND")
            .unwrap_or_else(|_| "127.0.0.1:18080".to_string())
            .parse()
            .expect("FC_API_BIND must be a socket address");
        Self { bind_addr }
    }
}
