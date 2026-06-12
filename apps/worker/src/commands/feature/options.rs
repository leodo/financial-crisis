use anyhow::{bail, Context};
use chrono::NaiveDate;

#[derive(Debug, Clone)]
pub(crate) struct FeatureSnapshotBuildOptions {
    pub(crate) market_scope: String,
    pub(crate) from: Option<NaiveDate>,
    pub(crate) to: Option<NaiveDate>,
    pub(crate) feature_set_version: String,
    pub(crate) point_in_time_mode: String,
    pub(crate) force_rebuild: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PointInTimeMode {
    BestEffort,
    Strict,
}

impl PointInTimeMode {
    pub(crate) fn parse(raw: &str) -> anyhow::Result<Self> {
        match raw {
            "best_effort" => Ok(Self::BestEffort),
            "strict" => Ok(Self::Strict),
            other => bail!(
                "unsupported --point-in-time-mode value: {other}; supported values are best_effort and strict"
            ),
        }
    }

    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::BestEffort => "best_effort",
            Self::Strict => "strict",
        }
    }
}

impl FeatureSnapshotBuildOptions {
    pub(crate) fn parse(args: &[String]) -> anyhow::Result<Self> {
        let mut market_scope = "financial_system".to_string();
        let mut from = None;
        let mut to = None;
        let mut feature_set_version = crate::DEFAULT_FORMAL_FEATURE_SET_VERSION.to_string();
        let mut point_in_time_mode = "best_effort".to_string();
        let mut force_rebuild = false;
        let mut index = 0;
        while index < args.len() {
            match args[index].as_str() {
                "--market-scope" => {
                    index += 1;
                    market_scope = args
                        .get(index)
                        .with_context(|| "--market-scope requires a value")?
                        .clone();
                }
                "--from" => {
                    index += 1;
                    from = Some(crate::parse_date_arg(args.get(index), "--from")?);
                }
                "--to" => {
                    index += 1;
                    to = Some(crate::parse_date_arg(args.get(index), "--to")?);
                }
                "--feature-set-version" => {
                    index += 1;
                    feature_set_version = args
                        .get(index)
                        .with_context(|| "--feature-set-version requires a value")?
                        .clone();
                }
                "--point-in-time-mode" => {
                    index += 1;
                    point_in_time_mode = args
                        .get(index)
                        .with_context(|| "--point-in-time-mode requires a value")?
                        .clone();
                }
                "--force-rebuild" => {
                    force_rebuild = true;
                }
                other => bail!("unknown feature snapshot build option: {other}"),
            }
            index += 1;
        }
        PointInTimeMode::parse(&point_in_time_mode)?;
        Ok(Self {
            market_scope,
            from,
            to,
            feature_set_version,
            point_in_time_mode,
            force_rebuild,
        })
    }
}

#[derive(Debug, Clone)]
pub(crate) struct FeatureSnapshotListOptions {
    pub(crate) market_scope: Option<String>,
    pub(crate) feature_set_version: Option<String>,
    pub(crate) from: Option<NaiveDate>,
    pub(crate) to: Option<NaiveDate>,
    pub(crate) limit: Option<usize>,
}

impl FeatureSnapshotListOptions {
    pub(crate) fn parse(args: &[String]) -> anyhow::Result<Self> {
        let mut market_scope = None;
        let mut feature_set_version = None;
        let mut from = None;
        let mut to = None;
        let mut limit = Some(20_usize);
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
                "--feature-set-version" => {
                    index += 1;
                    feature_set_version = Some(
                        args.get(index)
                            .with_context(|| "--feature-set-version requires a value")?
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
                other => bail!("unknown feature snapshot list option: {other}"),
            }
            index += 1;
        }
        Ok(Self {
            market_scope,
            feature_set_version,
            from,
            to,
            limit,
        })
    }
}
