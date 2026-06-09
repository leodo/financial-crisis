use chrono::{Datelike, Duration, NaiveDate, Weekday};

pub(super) fn business_lag_days(latest_date: NaiveDate, requested_as_of_date: NaiveDate) -> i64 {
    if requested_as_of_date <= latest_date {
        return 0;
    }

    let mut count = 0_i64;
    let mut cursor = latest_date
        .checked_add_signed(Duration::days(1))
        .unwrap_or(latest_date);
    while cursor < requested_as_of_date {
        if !matches!(cursor.weekday(), Weekday::Sat | Weekday::Sun) {
            count += 1;
        }
        let Some(next) = cursor.checked_add_signed(Duration::days(1)) else {
            break;
        };
        cursor = next;
    }

    count
}
