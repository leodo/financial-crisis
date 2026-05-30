use std::{process::Command, str};

use url::Url;

use crate::ConnectorError;

// Some FRED/Akamai edges time out on custom project user agents in this environment.
const USER_AGENT: &str = "curl/8.14.1";

pub fn user_agent() -> &'static str {
    USER_AGENT
}

pub fn curl_get_text(url: &Url, max_time_seconds: u64) -> Result<String, ConnectorError> {
    let output = Command::new(curl_binary())
        .args([
            "--fail",
            "--location",
            "--silent",
            "--show-error",
            "--max-time",
            &max_time_seconds.to_string(),
            "--noproxy",
            "*",
            "-A",
            USER_AGENT,
            url.as_str(),
        ])
        .output()
        .map_err(|error| {
            ConnectorError::TemporaryNetwork(format!("failed to start curl fallback: {error}"))
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(ConnectorError::TemporaryNetwork(format!(
            "curl fallback exited with {}: {}",
            output.status, stderr
        )));
    }

    String::from_utf8(output.stdout).map_err(|error| {
        ConnectorError::Parse(format!("curl fallback returned non-UTF-8: {error}"))
    })
}

fn curl_binary() -> &'static str {
    if cfg!(windows) {
        "curl.exe"
    } else {
        "curl"
    }
}
