use radio_datetime_utils::RadioDateTimeUtils;

fn main() {
    let mut dcf77 = RadioDateTimeUtils::new(7);

    // set date of now, leave out extra checks, do not compare to previous non-existing value:

    // day must be set *after* year, month, and weekday because set_day() checks if its argument
    // is within the current month which could be 28 or 29 for February depending on the year and
    // day-of-week for 00 years.

    dcf77.set_year(Some(24), true, false);
    // year is clipped to century
    dcf77.set_month(Some(1), true, false);
    dcf77.set_day(Some(25), true, false);
    println!("Day-of-month: {:?}", dcf77.get_day());
    dcf77.set_weekday(Some(4), true, false);
    dcf77.set_day(Some(25), true, false);
    dcf77.set_hour(Some(22), true, false);
    dcf77.set_minute(Some(34), true, false);
    // seconds are not present in RadioDateTimeUtils

    // Show the date and time:
    println!(
        "Date is {:?}-{:?}-{:?} weekday={:?} {:?}:{:?}",
        dcf77.get_year(),
        dcf77.get_month(),
        dcf77.get_day(),
        dcf77.get_weekday(),
        dcf77.get_hour(),
        dcf77.get_minute()
    );
}
