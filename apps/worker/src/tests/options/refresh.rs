use super::*;

#[test]
fn parses_refresh_latest_defaults() {
    let options = RefreshLatestOptions::parse(&[]).unwrap();
    assert_eq!(options.fast_lookback_days, 45);
    assert_eq!(options.slow_lookback_years, 15);
    assert_eq!(options.fred_chunk_days, 45);
    assert!(!options.skip_world_bank);
    assert!(!options.include_gdelt);
    assert!(!options.mvp_key_only);
    assert!(options.reload_api);
    assert_eq!(options.max_retries, 2);
    assert_eq!(options.retry_backoff_secs, 5);
}

#[test]
fn parses_refresh_latest_overrides() {
    let args = vec![
        "--fast-lookback-days".to_string(),
        "90".to_string(),
        "--skip-world-bank".to_string(),
        "--include-gdelt".to_string(),
        "--mvp-key-only".to_string(),
        "--no-reload-api".to_string(),
        "--max-retries".to_string(),
        "4".to_string(),
        "--retry-backoff-secs".to_string(),
        "0".to_string(),
    ];
    let options = RefreshLatestOptions::parse(&args).unwrap();
    assert_eq!(options.fast_lookback_days, 90);
    assert!(options.skip_world_bank);
    assert!(options.include_gdelt);
    assert!(options.mvp_key_only);
    assert!(!options.reload_api);
    assert_eq!(options.max_retries, 4);
    assert_eq!(options.retry_backoff_secs, 0);
}

#[test]
fn parses_audit_export_overrides() {
    let args = vec![
        "--api-base-url".to_string(),
        "http://127.0.0.1:18081".to_string(),
        "--output-dir".to_string(),
        "tmp/audit".to_string(),
    ];
    let options = AuditExportOptions::parse(&args).unwrap();
    assert_eq!(options.api_base_url, "http://127.0.0.1:18081");
    assert_eq!(options.output_dir, PathBuf::from("tmp/audit"));
}
