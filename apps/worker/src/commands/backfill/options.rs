use anyhow::{bail, Context};
use chrono::{NaiveDate, Utc};
use fc_domain::Frequency;
use fc_ingestion::BojDataset;
use fc_storage::ExternalIndicatorMapping;

#[derive(Debug, Clone)]
pub(crate) struct BackfillOptions {
    pub(crate) start: NaiveDate,
    pub(crate) end: NaiveDate,
    pub(crate) chunk_days: Option<i64>,
    pub(crate) indicator_filter: Option<String>,
    pub(crate) external_code_filter: Option<String>,
    pub(crate) watermark_overlap_days: Option<i64>,
    pub(crate) respect_frequency_watermark: bool,
}

impl BackfillOptions {
    pub(crate) fn parse(args: &[String]) -> anyhow::Result<Self> {
        let mut start = NaiveDate::from_ymd_opt(1990, 1, 1).expect("valid date");
        let mut end = Utc::now().date_naive();
        let mut chunk_days = None;
        let mut indicator_filter = None;
        let mut external_code_filter = None;
        let mut watermark_overlap_days = None;
        let mut index = 0;
        while index < args.len() {
            match args[index].as_str() {
                "--start" => {
                    index += 1;
                    start = crate::parse_date_arg(args.get(index), "--start")?;
                }
                "--end" => {
                    index += 1;
                    end = crate::parse_date_arg(args.get(index), "--end")?;
                }
                "--chunk-days" => {
                    index += 1;
                    let value = args
                        .get(index)
                        .with_context(|| "--chunk-days requires a positive integer")?
                        .parse::<i64>()
                        .with_context(|| "--chunk-days requires a positive integer")?;
                    if value <= 0 {
                        bail!("--chunk-days requires a positive integer");
                    }
                    chunk_days = Some(value);
                }
                "--indicator" => {
                    index += 1;
                    indicator_filter = Some(
                        args.get(index)
                            .with_context(|| "--indicator requires an indicator_id")?
                            .clone(),
                    );
                }
                "--external-code" => {
                    index += 1;
                    external_code_filter = Some(
                        args.get(index)
                            .with_context(|| "--external-code requires a source code")?
                            .clone(),
                    );
                }
                "--watermark-overlap-days" => {
                    index += 1;
                    watermark_overlap_days = Some(crate::parse_positive_i64(
                        args.get(index),
                        "--watermark-overlap-days",
                    )?);
                }
                other => bail!("unknown backfill option: {other}"),
            }
            index += 1;
        }
        if start > end {
            bail!("--start must be on or before --end");
        }
        Ok(Self {
            start,
            end,
            chunk_days,
            indicator_filter,
            external_code_filter,
            watermark_overlap_days,
            respect_frequency_watermark: false,
        })
    }

    pub(crate) fn with_default_chunk_days(mut self, chunk_days: i64) -> Self {
        if self.chunk_days.is_none() {
            self.chunk_days = Some(chunk_days);
        }
        self
    }

    pub(crate) fn with_frequency_watermark_refresh(mut self) -> Self {
        self.respect_frequency_watermark = true;
        self
    }

    pub(super) fn chunks(&self) -> Vec<(NaiveDate, NaiveDate)> {
        self.chunks_for_range(self.start, self.end)
    }

    fn chunks_for_range(&self, start: NaiveDate, end: NaiveDate) -> Vec<(NaiveDate, NaiveDate)> {
        let Some(chunk_days) = self.chunk_days else {
            return vec![(start, end)];
        };

        let mut chunks = Vec::new();
        let mut cursor = start;
        while cursor <= end {
            let chunk_end = (cursor + chrono::Duration::days(chunk_days - 1)).min(end);
            chunks.push((cursor, chunk_end));
            if chunk_end == end {
                break;
            }
            cursor = chunk_end + chrono::Duration::days(1);
        }
        chunks
    }

    pub(super) fn filter_mappings(
        &self,
        mappings: Vec<ExternalIndicatorMapping>,
    ) -> Vec<ExternalIndicatorMapping> {
        mappings
            .into_iter()
            .filter(|mapping| {
                self.indicator_filter
                    .as_ref()
                    .map(|filter| mapping.indicator_id == *filter)
                    .unwrap_or(true)
                    && self
                        .external_code_filter
                        .as_ref()
                        .map(|filter| mapping.external_code == *filter)
                        .unwrap_or(true)
            })
            .collect()
    }

    pub(super) fn effective_start(
        &self,
        watermark: Option<NaiveDate>,
        overlap_days: i64,
    ) -> NaiveDate {
        watermark
            .map(|date| (date - chrono::Duration::days(overlap_days)).max(self.start))
            .unwrap_or(self.start)
    }

    pub(super) fn should_skip_due_to_frequency_watermark(
        &self,
        frequency: Frequency,
        watermark: Option<NaiveDate>,
    ) -> bool {
        if !self.respect_frequency_watermark {
            return false;
        }

        let Some(watermark) = watermark else {
            return false;
        };

        let cadence_days = match frequency {
            Frequency::Daily | Frequency::Event => 0,
            Frequency::Weekly => 5,
            Frequency::Monthly => 20,
            Frequency::Quarterly => 60,
            Frequency::Annual => 300,
        };

        cadence_days > 0 && (self.end - watermark).num_days() < cadence_days
    }
}

#[derive(Debug, Clone)]
pub(crate) struct FredBackfillOptions {
    pub(crate) options: BackfillOptions,
    pub(crate) fred_mode: FredBackfillMode,
}

impl FredBackfillOptions {
    pub(crate) fn parse(args: &[String]) -> anyhow::Result<Self> {
        let mut filtered_args = Vec::new();
        let mut fred_mode = FredBackfillMode::GraphCsv;
        for arg in args {
            match arg.as_str() {
                "--api" => fred_mode = FredBackfillMode::Api,
                "--graph-csv" => fred_mode = FredBackfillMode::GraphCsv,
                _ => filtered_args.push(arg.clone()),
            }
        }
        Ok(Self {
            options: BackfillOptions::parse(&filtered_args)?,
            fred_mode,
        })
    }
}

#[derive(Debug, Clone)]
pub(crate) struct BojBackfillOptions {
    pub(crate) options: BackfillOptions,
    pub(crate) dataset: BojDataset,
}

impl BojBackfillOptions {
    pub(crate) fn parse(args: &[String]) -> anyhow::Result<Self> {
        let mut filtered_args = Vec::new();
        let mut dataset = BojDataset::FxDaily;
        let mut index = 0;
        while index < args.len() {
            if args[index] == "--dataset" {
                index += 1;
                let value = args
                    .get(index)
                    .context("--dataset requires `fx-daily` or `money-market`")?;
                dataset = match value.as_str() {
                    "fx-daily" => BojDataset::FxDaily,
                    "money-market" => BojDataset::MoneyMarketRates,
                    other => bail!("unsupported BOJ dataset: {other}"),
                };
            } else {
                filtered_args.push(args[index].clone());
            }
            index += 1;
        }
        Ok(Self {
            options: BackfillOptions::parse(&filtered_args)?,
            dataset,
        })
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum FredBackfillMode {
    GraphCsv,
    Api,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refresh_frequency_skip_ignores_daily_but_defers_slow_series_with_recent_watermark() {
        let mut options = BackfillOptions::parse(&[])
            .unwrap()
            .with_frequency_watermark_refresh();
        options.end = NaiveDate::from_ymd_opt(2026, 6, 9).unwrap();
        let recent = NaiveDate::from_ymd_opt(2026, 6, 5).unwrap();

        assert!(!options.should_skip_due_to_frequency_watermark(Frequency::Daily, Some(recent)));
        assert!(options.should_skip_due_to_frequency_watermark(Frequency::Weekly, Some(recent)));
        assert!(options.should_skip_due_to_frequency_watermark(Frequency::Monthly, Some(recent)));
        assert!(options.should_skip_due_to_frequency_watermark(Frequency::Annual, Some(recent)));
    }

    #[test]
    fn refresh_frequency_skip_keeps_stale_monthly_series_eligible() {
        let mut options = BackfillOptions::parse(&[])
            .unwrap()
            .with_frequency_watermark_refresh();
        options.end = NaiveDate::from_ymd_opt(2026, 6, 9).unwrap();
        let old_monthly = NaiveDate::from_ymd_opt(2026, 5, 1).unwrap();

        assert!(
            !options.should_skip_due_to_frequency_watermark(Frequency::Monthly, Some(old_monthly))
        );
    }
}
