use chrono::NaiveDate;
use fc_domain::{observation_is_visible_for_date_for_point_in_time_mode, Observation};

use super::options::PointInTimeMode;

pub(crate) fn observation_is_visible_for_date(
    observation: &Observation,
    as_of_date: NaiveDate,
    point_in_time_mode: PointInTimeMode,
) -> bool {
    observation_is_visible_for_date_for_point_in_time_mode(
        observation,
        as_of_date,
        point_in_time_mode.as_str(),
    )
}
