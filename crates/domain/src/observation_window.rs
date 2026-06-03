use chrono::NaiveDate;

use crate::Observation;

pub fn observation_history_for_indicator<'a>(
    observations: &'a [Observation],
    indicator_id: &str,
    as_of_date: NaiveDate,
) -> Vec<&'a Observation> {
    observation_history_for_indicator_where(observations, indicator_id, as_of_date, |_| true)
}

pub fn observation_history_for_indicator_where<'a, F>(
    observations: &'a [Observation],
    indicator_id: &str,
    as_of_date: NaiveDate,
    include: F,
) -> Vec<&'a Observation>
where
    F: Fn(&Observation) -> bool,
{
    let mut history = observations
        .iter()
        .filter(|observation| observation.indicator_id == indicator_id)
        .filter(|observation| observation.as_of_date <= as_of_date)
        .filter(|observation| include(observation))
        .collect::<Vec<_>>();
    history.sort_by_key(|observation| observation.as_of_date);
    history
}

pub fn observation_value_difference_from_tail(
    observations: &[&Observation],
    lookback: usize,
) -> Option<f64> {
    let latest = observations.last()?;
    let previous_index = observations.len().checked_sub(lookback + 1)?;
    let previous = observations.get(previous_index)?;
    Some(latest.value - previous.value)
}

pub fn observation_value_difference_for_indicator(
    observations: &[Observation],
    indicator_id: &str,
    as_of_date: NaiveDate,
    lookback: usize,
) -> Option<f64> {
    let history = observation_history_for_indicator(observations, indicator_id, as_of_date);
    observation_value_difference_from_tail(&history, lookback)
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;

    use crate::{Frequency, Observation};

    use super::{
        observation_history_for_indicator, observation_history_for_indicator_where,
        observation_value_difference_for_indicator, observation_value_difference_from_tail,
    };

    fn observation(indicator_id: &str, day: u32, value: f64) -> Observation {
        Observation {
            indicator_id: indicator_id.to_string(),
            entity_id: "us".to_string(),
            as_of_date: NaiveDate::from_ymd_opt(2026, 1, day).unwrap(),
            period_start: None,
            period_end: None,
            frequency: Frequency::Daily,
            value,
            unit: "index".to_string(),
            source_id: "test".to_string(),
            dataset_id: "test".to_string(),
            revision_time: None,
            publication_time: None,
            quality_score: 1.0,
            quality_flags: Vec::new(),
        }
    }

    #[test]
    fn history_filters_indicator_and_sorts_by_date() {
        let observations = vec![
            observation("vix", 3, 30.0),
            observation("other", 2, 99.0),
            observation("vix", 1, 10.0),
            observation("vix", 4, 40.0),
        ];

        let history = observation_history_for_indicator(
            &observations,
            "vix",
            NaiveDate::from_ymd_opt(2026, 1, 3).unwrap(),
        );

        assert_eq!(
            history
                .iter()
                .map(|observation| observation.value)
                .collect::<Vec<_>>(),
            vec![10.0, 30.0]
        );
    }

    #[test]
    fn history_accepts_visibility_filter() {
        let observations = vec![
            observation("vix", 1, 10.0),
            observation("vix", 2, 20.0),
            observation("vix", 3, 30.0),
        ];

        let history = observation_history_for_indicator_where(
            &observations,
            "vix",
            NaiveDate::from_ymd_opt(2026, 1, 3).unwrap(),
            |observation| observation.value >= 20.0,
        );

        assert_eq!(history.len(), 2);
        assert_eq!(history[0].value, 20.0);
    }

    #[test]
    fn tail_difference_requires_full_lookback() {
        let observations = vec![
            observation("vix", 1, 10.0),
            observation("vix", 2, 12.0),
            observation("vix", 3, 17.0),
        ];
        let history = observation_history_for_indicator(
            &observations,
            "vix",
            NaiveDate::from_ymd_opt(2026, 1, 3).unwrap(),
        );

        assert_eq!(
            observation_value_difference_from_tail(&history, 2),
            Some(7.0)
        );
        assert_eq!(
            observation_value_difference_for_indicator(
                &observations,
                "vix",
                history[2].as_of_date,
                3
            ),
            None
        );
    }
}
