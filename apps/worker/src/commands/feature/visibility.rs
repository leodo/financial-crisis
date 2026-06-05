use chrono::{DateTime, Datelike, Duration, NaiveDate, Utc, Weekday};
use fc_domain::{Frequency, Observation};

use super::options::PointInTimeMode;

pub(crate) fn observation_is_visible_for_date(
    observation: &Observation,
    as_of_date: NaiveDate,
    point_in_time_mode: PointInTimeMode,
) -> bool {
    observation_visible_at_for_mode(observation, point_in_time_mode)
        .map(|visible_at| visible_at <= assessment_cutoff_utc(as_of_date))
        .unwrap_or(false)
}

pub(super) fn observation_visible_at_for_mode(
    observation: &Observation,
    point_in_time_mode: PointInTimeMode,
) -> Option<DateTime<Utc>> {
    match point_in_time_mode {
        PointInTimeMode::BestEffort => best_effort_visible_at(observation),
        PointInTimeMode::Strict => strict_visible_at(observation),
    }
}

fn best_effort_visible_at(observation: &Observation) -> Option<DateTime<Utc>> {
    let anchor_date = observation.period_end.unwrap_or(observation.as_of_date);
    match observation.source_id.as_str() {
        "treasury" => Some(new_york_time_to_utc(anchor_date, 18, 0)),
        "world_bank" => anchor_date
            .checked_add_signed(Duration::days(270))
            .map(|date| new_york_time_to_utc(date, 17, 30)),
        "boj" => Some(tokyo_time_to_utc(anchor_date, 17, 0)),
        "sec_edgar" => Some(
            observation
                .publication_time
                .unwrap_or_else(|| new_york_time_to_utc(anchor_date, 18, 0)),
        ),
        "gdelt" => None,
        "mock" => Some(
            observation
                .publication_time
                .unwrap_or_else(|| new_york_time_to_utc(anchor_date, 17, 30)),
        ),
        _ => anchor_date
            .checked_add_signed(Duration::days(default_visibility_lag_days(
                observation.frequency,
            )))
            .map(|date| new_york_time_to_utc(date, 17, 30)),
    }
}

fn strict_visible_at(observation: &Observation) -> Option<DateTime<Utc>> {
    match observation.source_id.as_str() {
        "sec_edgar" | "mock" => observation.publication_time,
        _ => None,
    }
}

fn default_visibility_lag_days(frequency: Frequency) -> i64 {
    match frequency {
        Frequency::Daily | Frequency::Event => 0,
        Frequency::Weekly => 3,
        Frequency::Monthly => 15,
        Frequency::Quarterly => 45,
        Frequency::Annual => 270,
    }
}

fn assessment_cutoff_utc(as_of_date: NaiveDate) -> DateTime<Utc> {
    new_york_time_to_utc(as_of_date, 17, 30)
}

fn new_york_time_to_utc(date: NaiveDate, hour: u32, minute: u32) -> DateTime<Utc> {
    let utc_offset_hours = if is_new_york_dst(date) { 4 } else { 5 };
    let local = date
        .and_hms_opt(hour, minute, 0)
        .expect("local wall-clock timestamp must be valid");
    DateTime::<Utc>::from_naive_utc_and_offset(local + Duration::hours(utc_offset_hours), Utc)
}

fn tokyo_time_to_utc(date: NaiveDate, hour: u32, minute: u32) -> DateTime<Utc> {
    let local = date
        .and_hms_opt(hour, minute, 0)
        .expect("tokyo wall-clock timestamp must be valid");
    DateTime::<Utc>::from_naive_utc_and_offset(local - Duration::hours(9), Utc)
}

fn is_new_york_dst(date: NaiveDate) -> bool {
    let year = date.year();
    let (start, end) = if year >= 2007 {
        (
            nth_weekday_of_month(year, 3, Weekday::Sun, 2),
            nth_weekday_of_month(year, 11, Weekday::Sun, 1),
        )
    } else {
        (
            nth_weekday_of_month(year, 4, Weekday::Sun, 1),
            last_weekday_of_month(year, 10, Weekday::Sun),
        )
    };
    date >= start && date < end
}

fn nth_weekday_of_month(year: i32, month: u32, weekday: Weekday, nth: u32) -> NaiveDate {
    let first_day = NaiveDate::from_ymd_opt(year, month, 1).expect("valid calendar date");
    let first_weekday_offset = (7 + weekday.num_days_from_monday() as i64
        - first_day.weekday().num_days_from_monday() as i64)
        % 7;
    first_day
        .checked_add_signed(Duration::days(
            first_weekday_offset + 7 * i64::from(nth - 1),
        ))
        .expect("nth weekday must be representable")
}

fn last_weekday_of_month(year: i32, month: u32, weekday: Weekday) -> NaiveDate {
    let next_month = if month == 12 {
        NaiveDate::from_ymd_opt(year + 1, 1, 1).expect("valid calendar date")
    } else {
        NaiveDate::from_ymd_opt(year, month + 1, 1).expect("valid calendar date")
    };
    let last_day = next_month
        .checked_sub_signed(Duration::days(1))
        .expect("previous day must be valid");
    let backward_offset = (7 + last_day.weekday().num_days_from_monday() as i64
        - weekday.num_days_from_monday() as i64)
        % 7;
    last_day
        .checked_sub_signed(Duration::days(backward_offset))
        .expect("last weekday must be representable")
}
