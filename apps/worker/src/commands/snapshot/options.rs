use std::path::PathBuf;

use anyhow::{bail, Context};
use chrono::NaiveDate;

#[derive(Debug, Clone)]
pub(crate) struct PredictionSnapshotQueryOptions {
    pub(crate) market_scope: Option<String>,
    pub(crate) release_id: Option<String>,
    pub(crate) from: Option<NaiveDate>,
    pub(crate) to: Option<NaiveDate>,
    pub(crate) limit: Option<usize>,
}

impl PredictionSnapshotQueryOptions {
    pub(crate) fn parse(args: &[String]) -> anyhow::Result<Self> {
        Self::parse_with_default_limit(args, Some(20))
    }

    pub(crate) fn parse_with_default_limit(
        args: &[String],
        default_limit: Option<usize>,
    ) -> anyhow::Result<Self> {
        let mut market_scope = None;
        let mut release_id = None;
        let mut from = None;
        let mut to = None;
        let mut limit = default_limit;
        let mut index = 0;
        while index < args.len() {
            match args[index].as_str() {
                "--market-scope" => {
                    index += 1;
                    market_scope = Some(
                        args.get(index)
                            .with_context(|| "--market-scope requires a value")?
                            .clone(),
                    );
                }
                "--release-id" => {
                    index += 1;
                    release_id = Some(
                        args.get(index)
                            .with_context(|| "--release-id requires a value")?
                            .clone(),
                    );
                }
                "--from" => {
                    index += 1;
                    from = Some(crate::parse_date_arg(args.get(index), "--from")?);
                }
                "--to" => {
                    index += 1;
                    to = Some(crate::parse_date_arg(args.get(index), "--to")?);
                }
                "--limit" => {
                    index += 1;
                    limit = Some(
                        args.get(index)
                            .with_context(|| "--limit requires a number")?
                            .parse::<usize>()
                            .context("--limit must be an integer")?,
                    );
                }
                other => bail!("unknown prediction snapshot query option: {other}"),
            }
            index += 1;
        }
        Ok(Self {
            market_scope,
            release_id,
            from,
            to,
            limit,
        })
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum ExportFormat {
    Json,
    Csv,
}

impl ExportFormat {
    pub(crate) fn parse(raw: &str) -> anyhow::Result<Self> {
        match raw {
            "json" => Ok(Self::Json),
            "csv" => Ok(Self::Csv),
            other => bail!("unsupported format: {other}"),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct PredictionSnapshotExportOptions {
    pub(crate) query: PredictionSnapshotQueryOptions,
    pub(crate) format: ExportFormat,
    pub(crate) output_path: Option<PathBuf>,
}

impl PredictionSnapshotExportOptions {
    pub(crate) fn parse(args: &[String]) -> anyhow::Result<Self> {
        let mut output_path = None;
        let mut format = ExportFormat::Json;
        let mut query_args = Vec::new();
        let mut index = 0;
        while index < args.len() {
            match args[index].as_str() {
                "--output-path" => {
                    index += 1;
                    output_path = Some(PathBuf::from(
                        args.get(index)
                            .with_context(|| "--output-path requires a path")?,
                    ));
                }
                "--format" => {
                    index += 1;
                    format = ExportFormat::parse(
                        args.get(index)
                            .with_context(|| "--format requires json or csv")?,
                    )?;
                }
                other => query_args.push(other.to_string()),
            }
            index += 1;
        }

        Ok(Self {
            query: PredictionSnapshotQueryOptions::parse(&query_args)?,
            format,
            output_path,
        })
    }
}

#[derive(Debug, Clone)]
pub(crate) struct SnapshotDatasetExportOptions {
    pub(crate) query: PredictionSnapshotQueryOptions,
    pub(crate) format: ExportFormat,
    pub(crate) output_path: Option<PathBuf>,
}

impl SnapshotDatasetExportOptions {
    pub(crate) fn parse(args: &[String]) -> anyhow::Result<Self> {
        let mut output_path = None;
        let mut format = ExportFormat::Json;
        let mut query_args = Vec::new();
        let mut index = 0;
        while index < args.len() {
            match args[index].as_str() {
                "--output-path" => {
                    index += 1;
                    output_path = Some(PathBuf::from(
                        args.get(index)
                            .with_context(|| "--output-path requires a path")?,
                    ));
                }
                "--format" => {
                    index += 1;
                    format = ExportFormat::parse(
                        args.get(index)
                            .with_context(|| "--format requires json or csv")?,
                    )?;
                }
                other => query_args.push(other.to_string()),
            }
            index += 1;
        }

        Ok(Self {
            query: PredictionSnapshotQueryOptions::parse_with_default_limit(&query_args, None)?,
            format,
            output_path,
        })
    }
}
