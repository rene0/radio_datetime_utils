//! Definition of date/time structures commonly useful for time station decoders.

//! Build with no_std for embedded platforms.
#![cfg_attr(not(test), no_std)]

pub mod radio_datetime_helpers;

/// DST change has been announced
pub const DST_ANNOUNCED: u8 = 1;
/// DST change has been processed
pub const DST_PROCESSED: u8 = 2;
/// unexpected jump in DST state
pub const DST_JUMP: u8 = 4;
/// DST is active
pub const DST_SUMMER: u8 = 8;

/// Leap second has been announced
pub const LEAP_ANNOUNCED: u8 = 1;
/// Leap second has been processed
pub const LEAP_PROCESSED: u8 = 2;
/// Leap second is unexpectedly absent
pub const LEAP_MISSING: u8 = 4;

/// Size of bit buffer in seconds plus one spare because we cannot know
/// which method accessing the buffer is called after increase_second().
pub const BIT_BUFFER_SIZE: usize = 61 + 1;

/// Represents a date and time transmitted over radio.
#[derive(Clone, Copy)]
pub struct RadioDateTimeUtils {
    year: Option<u8>,
    month: Option<u8>,
    day: Option<u8>,
    weekday: Option<u8>,
    hour: Option<u8>,
    minute: Option<u8>,
    dst: Option<u8>,
    leap_second: Option<u8>,
    jump_year: bool,
    jump_month: bool,
    jump_day: bool,
    jump_weekday: bool,
    jump_hour: bool,
    jump_minute: bool,
    min_weekday: u8,
    max_weekday: u8,
    minutes_running: u8,   // internal counter for set_dst() and set_leap_second()
    dst_count: u8,         // internal counter for set_dst()
    first_minute: bool,    // internal flag for set_dst()
    leap_second_count: u8, // internal counter for set_leap_second()
}

impl RadioDateTimeUtils {
    /// Increase or wrap the passed second counter.
    ///
    /// Returns if the second counter was increased/wrapped normally (true)
    /// or due to an overflow (false).
    ///
    /// # Arguments
    /// * `second` - the value of the current second (so normally 0..59)
    /// * `new_minute` - whether a new minute arrived
    /// * `minute_length` - the length of this minute in seconds
    pub fn increase_second(second: &mut u8, new_minute: bool, minute_length: u8) -> bool {
        if new_minute {
            *second = 0;
            true
        } else {
            *second += 1;
            // wrap in case we missed the minute marker to prevent index-out-of-range
            if *second == minute_length || (*second as usize) == BIT_BUFFER_SIZE {
                *second = 0;
                false
            } else {
                true
            }
        }
    }

    /// Initialize a new RadioDateTimeUtils instance
    ///
    /// # Arguments
    /// * `sunday` - the numeric value of Sunday, i.e. 7 for DCF77 or 0 for MSF
    pub fn new(sunday: u8) -> Self {
        Self {
            year: None,
            month: None,
            day: None,
            weekday: None,
            hour: None,
            minute: None,
            dst: None,
            dst_count: 0,
            leap_second: None,
            leap_second_count: 0,
            jump_year: false,
            jump_month: false,
            jump_day: false,
            jump_weekday: false,
            jump_hour: false,
            jump_minute: false,
            min_weekday: (sunday != 0) as u8,
            max_weekday: if sunday == 7 { 7 } else { 6 },
            minutes_running: 0,
            first_minute: true,
        }
    }

    /// Get the current year, truncated to two digits.
    pub fn get_year(&self) -> Option<u8> {
        self.year
    }

    /// Get the current month.
    pub fn get_month(&self) -> Option<u8> {
        self.month
    }

    /// Get the current day of the month.
    pub fn get_day(&self) -> Option<u8> {
        self.day
    }

    /// Get the current day of the week as a number.
    pub fn get_weekday(&self) -> Option<u8> {
        self.weekday
    }

    /// Get the current hour.
    pub fn get_hour(&self) -> Option<u8> {
        self.hour
    }

    /// Get the current minute.
    pub fn get_minute(&self) -> Option<u8> {
        self.minute
    }

    /// Get the current bitmask value (if any) of the daylight saving time status.
    pub fn get_dst(&self) -> Option<u8> {
        self.dst
    }

    /// Get the current bitmask value of the leap second status.
    pub fn get_leap_second(&self) -> Option<u8> {
        self.leap_second
    }

    /// Return if the year has jumped unexpectedly.
    pub fn get_jump_year(&self) -> bool {
        self.jump_year
    }

    /// Return if the month has jumped unexpectedly.
    pub fn get_jump_month(&self) -> bool {
        self.jump_month
    }

    /// Return if the day-of-month has jumped unexpectedly.
    pub fn get_jump_day(&self) -> bool {
        self.jump_day
    }

    /// Return if the day-of-week has jumped unexpectedly.
    pub fn get_jump_weekday(&self) -> bool {
        self.jump_weekday
    }

    /// Return if the hour has jumped unexpectedly.
    pub fn get_jump_hour(&self) -> bool {
        self.jump_hour
    }

    /// Return if the minute has jumped unexpectedly.
    pub fn get_jump_minute(&self) -> bool {
        self.jump_minute
    }

    /// Returns if the current date/time is valid (date, time, DST are all `is_some()`).
    pub fn is_valid(&self) -> bool {
        self.dst.is_some()
            && self.year.is_some()
            && self.month.is_some()
            && self.day.is_some()
            && self.weekday.is_some()
            && self.hour.is_some()
            && self.minute.is_some()
    }

    /// Clear all jump values.
    pub fn clear_jumps(&mut self) {
        self.jump_year = false;
        self.jump_month = false;
        self.jump_day = false;
        self.jump_weekday = false;
        self.jump_hour = false;
        self.jump_minute = false;
        if self.dst.is_some() {
            self.dst = Some(self.dst.unwrap() & !DST_JUMP);
        }
    }

    /// Add one minute to the current date and time, return if the operation succeeded.
    ///
    /// * Years are limited to 2 digits, so this function wraps after 100 years.
    pub fn add_minute(&mut self) -> bool {
        if !self.is_valid()
        {
            return false;
        }
        let mut s_minute = self.minute.unwrap();
        let mut s_hour = self.hour.unwrap();
        let mut s_day = self.day.unwrap();
        let mut s_weekday = self.weekday.unwrap();
        let mut s_month = self.month.unwrap();
        let mut s_year = self.year.unwrap();
        s_minute += 1;
        if s_minute == 60 {
            s_minute = 0;
            if (self.dst.unwrap() & DST_ANNOUNCED) != 0 {
                if (self.dst.unwrap() & DST_SUMMER) != 0 {
                    s_hour -= 1; // changing to winter
                } else {
                    s_hour += 1; // changing to summer
                }
            }
            s_hour += 1;
            if s_hour == 24 {
                s_hour = 0;
                let old_last_day = self.last_day(s_day).unwrap();
                s_weekday += 1;
                if s_weekday == self.max_weekday + 1 {
                    s_weekday = self.min_weekday;
                }
                s_day += 1;
                if s_day > old_last_day {
                    s_day = 1;
                    s_month += 1;
                    if s_month == 13 {
                        s_month = 1;
                        s_year += 1;
                        if s_year == 100 {
                            s_year = 0;
                        }
                    }
                }
            }
        }
        self.minute = Some(s_minute);
        self.hour = Some(s_hour);
        self.day = Some(s_day);
        self.weekday = Some(s_weekday);
        self.month = Some(s_month);
        self.year = Some(s_year);
        true
    }

    /// Set the year value, valid values are 0 through 99.
    ///
    /// # Arguments
    /// * `value` - the new year value. None or invalid values keep the old value.
    /// * `valid` - extra validation to pass.
    /// * `check_jump` - check if the value has jumped unexpectedly compared to `add_minute()`.
    pub fn set_year(&mut self, value: Option<u8>, valid: bool, check_jump: bool) {
        let year = if value.is_some() && (0..=99).contains(&value.unwrap()) && valid {
            value
        } else {
            self.year
        };
        self.jump_year = check_jump && year.is_some() && self.year.is_some() && year != self.year;
        self.year = year;
    }

    /// Set the month value, valid values are 1 through 12.
    ///
    /// # Arguments
    /// * `value` - the new month value. None or invalid values keep the old value.
    /// * `valid` - extra validation to pass.
    /// * `check_jump` - check if the value has jumped unexpectedly compared to `add_minute()`.
    pub fn set_month(&mut self, value: Option<u8>, valid: bool, check_jump: bool) {
        let month = if value.is_some() && (1..=12).contains(&value.unwrap()) && valid {
            value
        } else {
            self.month
        };
        self.jump_month =
            check_jump && month.is_some() && self.month.is_some() && month != self.month;
        self.month = month;
    }

    /// Set the day-of-week value, valid values are 0/1 through 6/7, depending on how this
    /// instance was created.
    ///
    /// # Arguments
    /// * `value` - the new day-of-week value. None or invalid values keep the old value.
    /// * `valid` - extra validation to pass.
    /// * `check_jump` - check if the value has jumped unexpectedly compared to `add_minute()`.
    pub fn set_weekday(&mut self, value: Option<u8>, valid: bool, check_jump: bool) {
        let weekday = if value.is_some()
            && (self.min_weekday..=self.max_weekday).contains(&value.unwrap())
            && valid
        {
            value
        } else {
            self.weekday
        };
        self.jump_weekday =
            check_jump && weekday.is_some() && self.weekday.is_some() && weekday != self.weekday;
        self.weekday = weekday;
    }

    /// Set the day-in-month value, valid values are 1 through the last day of that month.
    ///
    /// If the year, month, or weekday are absent, the last day of the month cannot be
    /// calculated which means the old day-in-month value is kept.
    ///
    /// # Arguments
    /// * `value` - the new day-in-month value. None or invalid values keep the old value.
    /// * `valid` - extra validation to pass.
    /// * `check_jump` - check if the value has jumped unexpectedly compared to `add_minute()`.
    pub fn set_day(&mut self, value: Option<u8>, valid: bool, check_jump: bool) {
        let day = if value.is_some()
            && self.last_day(value.unwrap()).is_some()
            && (1..=self.last_day(value.unwrap()).unwrap()).contains(&value.unwrap())
            && valid
        {
            value
        } else {
            self.day
        };
        self.jump_day = check_jump && day.is_some() && self.day.is_some() && day != self.day;
        self.day = day;
    }

    /// Set the hour value, valid values are 0 through 23.
    ///
    /// # Arguments
    /// * `value` - the new hour value. None or invalid values keep the old value.
    /// * `valid` - extra validation to pass.
    /// * `check_jump` - check if the value has jumped unexpectedly compared to `add_minute()`.
    pub fn set_hour(&mut self, value: Option<u8>, valid: bool, check_jump: bool) {
        let hour = if value.is_some() && (0..=23).contains(&value.unwrap()) && valid {
            value
        } else {
            self.hour
        };
        self.jump_hour = check_jump && hour.is_some() && self.hour.is_some() && hour != self.hour;
        self.hour = hour;
    }

    /// Set the minute value, valid values are 0 through 59.
    ///
    /// # Arguments
    /// * `value` - the new minute value. None or invalid values keep the old value.
    /// * `valid` - extra validation to pass.
    /// * `check_jump` - check if the value has jumped unexpectedly compared to `add_minute()`.
    pub fn set_minute(&mut self, value: Option<u8>, valid: bool, check_jump: bool) {
        let minute = if value.is_some() && (0..=59).contains(&value.unwrap()) && valid {
            value
        } else {
            self.minute
        };
        self.jump_minute =
            check_jump && minute.is_some() && self.minute.is_some() && minute != self.minute;
        self.minute = minute;
    }

    /// Set the DST mask value, both the actual value and any information on transitions.
    ///
    /// # Arguments
    /// * `value` - the new DST value. None or unannounced changes keep the old value.
    /// * `announce` - if any announcement is made on a transition. The history of this
    ///                value of the last hour (or part thereof if started later) is kept
    ///                to compensate for spurious True values.
    /// * `check_jump` - check if the value changed unexpectedly.
    pub fn set_dst(&mut self, value: Option<bool>, announce: Option<bool>, check_jump: bool) {
        if value.is_none() || announce.is_none() {
            return;
        }
        if self.dst.is_none() {
            self.dst = Some(0);
        }
        // Clear any jump flag from the previous decoding:
        self.dst = Some(self.dst.unwrap() & !DST_JUMP);
        // Determine if a DST change is announced:
        if announce == Some(true) {
            self.dst_count += 1;
        }
        if self.minute.is_some() && self.minute.unwrap() > 0 {
            if 2 * self.dst_count > self.minutes_running {
                self.dst = Some(self.dst.unwrap() | DST_ANNOUNCED);
            } else {
                self.dst = Some(self.dst.unwrap() & !DST_ANNOUNCED);
            }
        }
        if value.unwrap() != ((self.dst.unwrap() & DST_SUMMER) != 0) {
            // Time offset changed.
            if self.first_minute
                || ((self.dst.unwrap() & DST_ANNOUNCED) != 0 && self.minute == Some(0))
            {
                // Change is valid.
                if value.unwrap() {
                    self.dst = Some(self.dst.unwrap() | DST_SUMMER);
                } else {
                    self.dst = Some(self.dst.unwrap() & !DST_SUMMER);
                }
            } else if check_jump {
                self.dst = Some(self.dst.unwrap() | DST_JUMP);
            }
        }
        if self.minute == Some(0) && (self.dst.unwrap() & DST_ANNOUNCED) != 0 {
            // DST change processed:
            self.dst = Some(self.dst.unwrap() | DST_PROCESSED);
        } else if self.minute.is_some() {
            self.dst = Some(self.dst.unwrap() & !DST_PROCESSED);
        }
        // Always reset announcement at the hour:
        if self.minute == Some(0) {
            self.dst = Some(self.dst.unwrap() & !DST_ANNOUNCED);
            self.dst_count = 0;
        }
        self.first_minute = false;
    }

    /// Set the leap second value.
    ///
    /// # Arguments
    /// * `announce` - if any announcement is made on a positive leap second. The history
    ///                of this value of the last hour (or part thereof if started later) is
    ///                kept to compensate for spurious Some(True) values.
    /// * `minute_length` - the length of the decoded minute in seconds.
    pub fn set_leap_second(&mut self, announce: Option<bool>, minute_length: u8) {
        if announce.is_none() || !(60..=61).contains(&minute_length) {
            return;
        }
        if self.leap_second.is_none() {
            self.leap_second = Some(0);
        }
        // Determine if a leap second is announced:
        if announce == Some(true) {
            self.leap_second_count += 1;
        }
        if self.minute.is_some() && self.minute.unwrap() > 0 {
            if 2 * self.leap_second_count > self.minutes_running {
                self.leap_second = Some(self.leap_second.unwrap() | LEAP_ANNOUNCED);
            } else {
                self.leap_second = Some(self.leap_second.unwrap() & !LEAP_ANNOUNCED);
            }
        }
        // Process possible leap second:
        if self.minute == Some(0) && (self.leap_second.unwrap() & LEAP_ANNOUNCED) != 0 {
            self.leap_second = Some(self.leap_second.unwrap() | LEAP_PROCESSED);
            if minute_length == 60 {
                // Leap second processed, but missing:
                self.leap_second = Some(self.leap_second.unwrap() | LEAP_MISSING);
            } else {
                // Leap second processed and present:
                self.leap_second = Some(self.leap_second.unwrap() & !LEAP_MISSING);
            }
        } else if self.minute.is_some() {
            self.leap_second = Some(self.leap_second.unwrap() & !LEAP_PROCESSED & !LEAP_MISSING);
        }
        // Always reset announcement at the hour:
        if self.minute == Some(0) {
            self.leap_second = Some(self.leap_second.unwrap() & !LEAP_ANNOUNCED);
            self.leap_second_count = 0;
        }
    }

    /// Bump the internal minute counter needed for set_dst() and set_leap_second()
    ///
    /// The code above this library must call this function, as this library cannot
    /// know which function got called first, or if just one of them should be called.
    pub fn bump_minutes_running(&mut self) {
        self.minutes_running += 1;
        if self.minute == Some(0) {
            self.minutes_running = 0;
        }
    }

    /// Return the last calendar day of the current date, or None in case of error.
    ///
    /// # Arguments
    /// * `day` - day of the month in February '00, used to see if `year` is a leap year
    fn last_day(&self, day: u8) -> Option<u8> {
        // We need to check for day being 1..=31 here because set_day() uses this function in its input checks.
        if self.year.is_none()
            || self.month.is_none()
            || self.weekday.is_none()
            || !(1..=31).contains(&day)
        {
            return None;
        }
        let s_year = self.year.unwrap();
        let s_month = self.month.unwrap();
        let s_weekday = self.weekday.unwrap();
        if s_month == 2 {
            if (s_year != 0 && s_year % 4 == 0)
                || (s_year == 0 && RadioDateTimeUtils::is_leap_century(day, s_weekday))
            {
                Some(29)
            } else {
                Some(28)
            }
        } else if s_month == 4 || s_month == 6 || s_month == 9 || s_month == 11 {
            Some(30)
        } else {
            Some(31)
        }
    }

    /// Check if the century based on the given date is divisible by 400.
    ///
    ///  Based on xx00-02-28 is a Monday <=> xx00 is a leap year
    ///
    /// # Arguments
    ///  * `day` - day of the month in February '00
    ///  * `weekday` - day of the week in February '00
    fn is_leap_century(day: u8, weekday: u8) -> bool {
        // Ensure Sunday is 7 when dealing with e.g. MSF :
        let wd = if weekday == 0 { 7 } else { weekday };

        // Week day 1 is a Monday, assume this is a leap year.
        // If so, we should reach Monday xx00-02-28
        if day < 29 {
            // (8 - wd) == ((8-1)..=(8-7)) == (7..=1) --> Monday=7, Tuesday=6, .., Sunday=1
            // Transpose day to 22..=28, then check if the result plus the inverted day-of-week adds up to 28
            day + 7 * ((28 - day) / 7) + (8 - wd) == 28
        } else {
            wd == 2 // Tuesday xx00-02-29
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_increase_second_regular() {
        let mut second = 54;
        assert_eq!(
            RadioDateTimeUtils::increase_second(&mut second, false, 60),
            true
        );
        assert_eq!(second, 55);
    }
    #[test]
    fn test_increase_second_new_minute() {
        let mut second = 54; // can be less than minute_length-1 during the first partial minute
        assert_eq!(
            RadioDateTimeUtils::increase_second(&mut second, true, 60),
            true
        );
        assert_eq!(second, 0);
    }
    #[test]
    fn test_increase_second_overflow_minute_length_hit() {
        let mut second = 59;
        assert_eq!(
            RadioDateTimeUtils::increase_second(&mut second, false, 60),
            false
        );
        assert_eq!(second, 0);
    }
    #[test]
    #[should_panic] // caller is lagging, this should never happen
    fn test_increase_second_overflow_minute_length_over() {
        let mut second = 60;
        assert_eq!(
            RadioDateTimeUtils::increase_second(&mut second, false, 60),
            false
        );
        assert_eq!(second, 0);
    }
    #[test]
    fn test_increase_second_overflow_buffer_hit() {
        let mut second = (BIT_BUFFER_SIZE - 1) as u8;
        assert_eq!(
            RadioDateTimeUtils::increase_second(&mut second, false, 60),
            false
        );
        assert_eq!(second, 0);
    }
    #[test]
    #[should_panic] // caller is lagging, this should never happen
    fn test_increase_second_overflow_buffer_over() {
        let mut second = BIT_BUFFER_SIZE as u8;
        assert_eq!(
            RadioDateTimeUtils::increase_second(&mut second, false, 60),
            false
        );
        assert_eq!(second, 0);
    }

    #[test]
    fn test_set_year_some_invalid_jump() {
        let mut rdt = RadioDateTimeUtils::new(0);
        rdt.set_year(Some(22), false, true);
        assert_eq!(rdt.year, None);
        assert_eq!(rdt.jump_year, false);
    }
    #[test]
    fn test_set_year_none_valid_jump() {
        let mut rdt = RadioDateTimeUtils::new(0);
        rdt.set_year(None, true, true);
        assert_eq!(rdt.year, None);
        assert_eq!(rdt.jump_year, false);
    }
    #[test]
    fn test_set_year_too_large_valid_no_jump() {
        let mut rdt = RadioDateTimeUtils::new(0);
        rdt.set_year(Some(100), true, false);
        assert_eq!(rdt.year, None);
        assert_eq!(rdt.jump_year, false);
    }
    #[test]
    fn test_set_year_some_valid_no_jump() {
        let mut rdt = RadioDateTimeUtils::new(0);
        rdt.set_year(Some(22), true, false);
        assert_eq!(rdt.year, Some(22));
        assert_eq!(rdt.jump_year, false);
    }
    #[test]
    fn continue_set_year_too_large_valid_jump() {
        let mut rdt = RadioDateTimeUtils::new(0);
        rdt.set_year(Some(22), true, false);
        rdt.set_year(Some(100), true, true);
        assert_eq!(rdt.year, Some(22));
        assert_eq!(rdt.jump_year, false);
    }
    #[test]
    fn test_set_year_some_valid_jump() {
        let mut rdt = RadioDateTimeUtils::new(0);
        rdt.set_year(Some(22), true, true);
        assert_eq!(rdt.year, Some(22));
        assert_eq!(rdt.jump_year, false);
    }
    #[test]
    fn continue_set_year_some_valid_jump() {
        let mut rdt = RadioDateTimeUtils::new(0);
        rdt.set_year(Some(22), true, true);
        rdt.set_year(Some(23), true, true);
        assert_eq!(rdt.year, Some(23));
        assert_eq!(rdt.jump_year, true);
    }

    #[test]
    fn test_set_month_some_invalid_jump() {
        let mut rdt = RadioDateTimeUtils::new(0);
        rdt.set_month(Some(9), false, true);
        assert_eq!(rdt.month, None);
        assert_eq!(rdt.jump_month, false);
    }
    #[test]
    fn test_set_month_none_valid_jump() {
        let mut rdt = RadioDateTimeUtils::new(0);
        rdt.set_month(None, true, true);
        assert_eq!(rdt.month, None);
        assert_eq!(rdt.jump_month, false);
    }
    #[test]
    fn test_set_month_too_small_valid_no_jump() {
        let mut rdt = RadioDateTimeUtils::new(0);
        rdt.set_month(Some(0), true, false);
        assert_eq!(rdt.month, None);
        assert_eq!(rdt.jump_month, false);
    }
    #[test]
    fn test_set_month_some_valid_no_jump() {
        let mut rdt = RadioDateTimeUtils::new(0);
        rdt.set_month(Some(9), true, false);
        assert_eq!(rdt.month, Some(9));
        assert_eq!(rdt.jump_month, false);
    }
    #[test]
    fn continue_set_month_too_large_valid_jump() {
        let mut rdt = RadioDateTimeUtils::new(0);
        rdt.set_month(Some(9), true, false);
        rdt.set_month(Some(13), true, true);
        assert_eq!(rdt.month, Some(9));
        assert_eq!(rdt.jump_month, false);
    }
    #[test]
    fn test_set_month_some_valid_jump() {
        let mut rdt = RadioDateTimeUtils::new(0);
        rdt.set_month(Some(9), true, true);
        assert_eq!(rdt.month, Some(9));
        assert_eq!(rdt.jump_month, false);
    }
    #[test]
    fn continue_set_month_some_valid_jump() {
        let mut rdt = RadioDateTimeUtils::new(0);
        rdt.set_month(Some(9), true, true);
        rdt.set_month(Some(10), true, true);
        assert_eq!(rdt.month, Some(10));
        assert_eq!(rdt.jump_month, true);
    }

    #[test]
    fn test_set_day_some_invalid_jump() {
        let mut rdt = RadioDateTimeUtils::new(0);
        rdt.set_day(Some(23), false, true);
        assert_eq!(rdt.day, None);
        assert_eq!(rdt.jump_day, false);
    }
    #[test]
    fn test_set_day_none_valid_jump() {
        let mut rdt = RadioDateTimeUtils::new(0);
        rdt.set_day(None, true, true);
        assert_eq!(rdt.day, None);
        assert_eq!(rdt.jump_day, false);
    }
    #[test]
    fn test_set_day_too_small_valid_no_jump() {
        let mut rdt = RadioDateTimeUtils::new(0);
        rdt.set_day(Some(0), true, false);
        assert_eq!(rdt.day, None);
        assert_eq!(rdt.jump_day, false);
    }
    #[test]
    fn test_set_day_some_valid_no_jump() {
        let mut rdt = RadioDateTimeUtils::new(0);
        // set_day() requires a full date and weekday to work
        rdt.year = Some(22);
        rdt.month = Some(9);
        rdt.weekday = Some(10); // any Some value works here because rdt.month != Some(2)
        rdt.set_day(Some(23), true, false);
        assert_eq!(rdt.day, Some(23));
        assert_eq!(rdt.jump_day, false);
    }
    #[test]
    fn continue_set_day_too_large_valid_jump() {
        let mut rdt = RadioDateTimeUtils::new(0);
        // set_day() requires a full date and weekday to work
        rdt.year = Some(22);
        rdt.month = Some(9);
        rdt.weekday = Some(5); // any Some value works here because rdt.month != Some(2)
        rdt.set_day(Some(23), true, false);
        rdt.set_day(Some(32), true, true);
        assert_eq!(rdt.day, Some(23));
        assert_eq!(rdt.jump_day, false);
    }
    #[test]
    fn test_set_day_some_valid_jump() {
        let mut rdt = RadioDateTimeUtils::new(0);
        rdt.year = Some(22);
        rdt.month = Some(9);
        rdt.weekday = Some(0); // any Some value works here because rdt.month != Some(2)
        rdt.set_day(Some(23), true, true);
        assert_eq!(rdt.day, Some(23));
        assert_eq!(rdt.jump_day, false);
    }
    #[test]
    fn continue_set_day_some_valid_jump() {
        let mut rdt = RadioDateTimeUtils::new(0);
        rdt.year = Some(22);
        rdt.month = Some(9);
        rdt.weekday = Some(7); // any Some value works here because rdt.month != Some(2)
        rdt.set_day(Some(23), true, true);
        rdt.set_day(Some(24), true, true);
        assert_eq!(rdt.day, Some(24));
        assert_eq!(rdt.jump_day, true);
    }

    #[test]
    fn test_set_weekday_some_invalid_jump() {
        let mut rdt = RadioDateTimeUtils::new(0);
        rdt.set_weekday(Some(5), false, true);
        assert_eq!(rdt.weekday, None);
        assert_eq!(rdt.jump_weekday, false);
    }
    #[test]
    fn test_set_weekday_none_valid_jump() {
        let mut rdt = RadioDateTimeUtils::new(0);
        rdt.set_weekday(None, true, true);
        assert_eq!(rdt.weekday, None);
        assert_eq!(rdt.jump_weekday, false);
    }
    #[test]
    fn test_set_weekday_too_large_valid_no_jump() {
        let mut rdt = RadioDateTimeUtils::new(0);
        rdt.set_weekday(Some(7), true, false);
        assert_eq!(rdt.weekday, None);
        assert_eq!(rdt.jump_weekday, false);
    }
    #[test]
    fn test_set_weekday_some_valid_no_jump() {
        let mut rdt = RadioDateTimeUtils::new(0);
        rdt.set_weekday(Some(5), true, false);
        assert_eq!(rdt.weekday, Some(5));
        assert_eq!(rdt.jump_weekday, false);
    }
    #[test]
    fn continue_set_weekday_too_large_valid_jump() {
        let mut rdt = RadioDateTimeUtils::new(0);
        rdt.set_weekday(Some(5), true, false);
        rdt.set_weekday(Some(7), true, true);
        assert_eq!(rdt.weekday, Some(5));
        assert_eq!(rdt.jump_weekday, false);
    }
    #[test]
    fn test_set_weekday_some_valid_jump() {
        let mut rdt = RadioDateTimeUtils::new(0);
        rdt.set_weekday(Some(5), true, true);
        assert_eq!(rdt.weekday, Some(5));
        assert_eq!(rdt.jump_weekday, false);
    }
    #[test]
    fn continue_set_weekday_some_valid_jump() {
        let mut rdt = RadioDateTimeUtils::new(0);
        rdt.set_weekday(Some(5), true, true);
        rdt.set_weekday(Some(6), true, true);
        assert_eq!(rdt.weekday, Some(6));
        assert_eq!(rdt.jump_weekday, true);
    }
    #[test]
    fn test_set_weekday7_too_small_valid_no_jump() {
        let mut rdt = RadioDateTimeUtils::new(7);
        rdt.set_weekday(Some(0), true, false);
        assert_eq!(rdt.weekday, None);
        assert_eq!(rdt.jump_weekday, false);
    }
    #[test]
    fn continue_set_weekday7_too_small_valid_jump() {
        let mut rdt = RadioDateTimeUtils::new(7);
        rdt.set_weekday(Some(5), true, false);
        rdt.set_weekday(Some(0), true, true);
        assert_eq!(rdt.weekday, Some(5));
        assert_eq!(rdt.jump_weekday, false);
    }

    #[test]
    fn test_set_hour_some_invalid_jump() {
        let mut rdt = RadioDateTimeUtils::new(0);
        rdt.set_hour(Some(22), false, true);
        assert_eq!(rdt.hour, None);
        assert_eq!(rdt.jump_hour, false);
    }
    #[test]
    fn test_set_hour_none_valid_jump() {
        let mut rdt = RadioDateTimeUtils::new(0);
        rdt.set_hour(None, true, true);
        assert_eq!(rdt.hour, None);
        assert_eq!(rdt.jump_hour, false);
    }
    #[test]
    fn test_set_hour_too_large_valid_no_jump() {
        let mut rdt = RadioDateTimeUtils::new(0);
        rdt.set_hour(Some(24), true, false);
        assert_eq!(rdt.hour, None);
        assert_eq!(rdt.jump_hour, false);
    }
    #[test]
    fn test_set_hour_some_valid_no_jump() {
        let mut rdt = RadioDateTimeUtils::new(0);
        rdt.set_hour(Some(22), true, false);
        assert_eq!(rdt.hour, Some(22));
        assert_eq!(rdt.jump_hour, false);
    }
    #[test]
    fn continue_set_hour_too_large_valid_jump() {
        let mut rdt = RadioDateTimeUtils::new(0);
        rdt.set_hour(Some(22), true, false);
        rdt.set_hour(Some(24), true, true);
        assert_eq!(rdt.hour, Some(22));
        assert_eq!(rdt.jump_hour, false);
    }
    #[test]
    fn test_set_hour_some_valid_jump() {
        let mut rdt = RadioDateTimeUtils::new(0);
        rdt.set_hour(Some(22), true, true);
        assert_eq!(rdt.hour, Some(22));
        assert_eq!(rdt.jump_hour, false);
    }
    #[test]
    fn continue_set_hour_some_valid_jump() {
        let mut rdt = RadioDateTimeUtils::new(0);
        rdt.set_hour(Some(22), true, true);
        rdt.set_hour(Some(23), true, true);
        assert_eq!(rdt.hour, Some(23));
        assert_eq!(rdt.jump_hour, true);
    }

    #[test]
    fn test_set_minute_some_invalid_jump() {
        let mut rdt = RadioDateTimeUtils::new(0);
        rdt.set_minute(Some(47), false, true);
        assert_eq!(rdt.minute, None);
        assert_eq!(rdt.jump_minute, false);
    }
    #[test]
    fn test_set_minute_none_valid_jump() {
        let mut rdt = RadioDateTimeUtils::new(0);
        rdt.set_minute(None, true, true);
        assert_eq!(rdt.minute, None);
        assert_eq!(rdt.jump_minute, false);
    }
    #[test]
    fn test_set_minute_too_large_valid_no_jump() {
        let mut rdt = RadioDateTimeUtils::new(0);
        rdt.set_minute(Some(60), true, false);
        assert_eq!(rdt.minute, None);
        assert_eq!(rdt.jump_minute, false);
    }
    #[test]
    fn test_set_minute_some_valid_no_jump() {
        let mut rdt = RadioDateTimeUtils::new(0);
        rdt.set_minute(Some(47), true, false);
        assert_eq!(rdt.minute, Some(47));
        assert_eq!(rdt.jump_minute, false);
    }
    #[test]
    fn continue_set_minute_too_large_valid_jump() {
        let mut rdt = RadioDateTimeUtils::new(0);
        rdt.set_minute(Some(47), true, false);
        rdt.set_minute(Some(60), true, true);
        assert_eq!(rdt.minute, Some(47));
        assert_eq!(rdt.jump_minute, false);
    }
    #[test]
    fn test_set_minute_some_valid_jump() {
        let mut rdt = RadioDateTimeUtils::new(0);
        rdt.set_minute(Some(47), true, true);
        assert_eq!(rdt.minute, Some(47));
        assert_eq!(rdt.jump_minute, false);
    }
    #[test]
    fn continue_set_minute_some_valid_jump() {
        let mut rdt = RadioDateTimeUtils::new(0);
        rdt.set_minute(Some(47), true, true);
        rdt.set_minute(Some(48), true, true);
        assert_eq!(rdt.minute, Some(48));
        assert_eq!(rdt.jump_minute, true);
    }

    #[test]
    fn test_last_day7_regular() {
        let mut dcf77 = RadioDateTimeUtils::new(7);
        dcf77.year = Some(22);
        dcf77.month = Some(6);
        dcf77.weekday = Some(7);
        assert_eq!(dcf77.last_day(5), Some(30)); // today, Sunday 2022-06-05
    }
    #[test]
    fn test_last_day7_non_existent() {
        let mut dcf77 = RadioDateTimeUtils::new(7);
        dcf77.year = Some(22);
        dcf77.month = Some(2);
        dcf77.weekday = Some(4);
        assert_eq!(dcf77.last_day(29), Some(28)); // non-existent date, Thursday 22-02-29
    }
    #[test]
    fn test_last_day7_happy_new_year() {
        let mut dcf77 = RadioDateTimeUtils::new(7);
        dcf77.year = Some(0);
        dcf77.month = Some(1);
        dcf77.weekday = Some(1);
        assert_eq!(dcf77.last_day(1), Some(31)); // first day, weekday off/do-not-care, Monday 00-01-01
    }
    #[test]
    fn test_last_day7_regular_leap() {
        let mut dcf77 = RadioDateTimeUtils::new(7);
        dcf77.year = Some(20);
        dcf77.month = Some(2);
        dcf77.weekday = Some(3);
        assert_eq!(dcf77.last_day(3), Some(29)); // regular leap year, Wednesday 2020-02-03
    }
    #[test]
    fn test_last_day7_bogus_weekday() {
        let mut dcf77 = RadioDateTimeUtils::new(7);
        dcf77.year = Some(20);
        dcf77.month = Some(2);
        dcf77.weekday = Some(4);
        assert_eq!(dcf77.last_day(3), Some(29)); // same date with bogus weekday, "Thursday" 2020-02-03
    }
    #[test]
    fn test_last_day7_century_leap_1() {
        let mut dcf77 = RadioDateTimeUtils::new(7);
        dcf77.year = Some(0);
        dcf77.month = Some(2);
        dcf77.weekday = Some(2);
        assert_eq!(dcf77.last_day(1), Some(29)); // century-leap-year, day/weekday must match, Tuesday 2000-02-01
    }
    #[test]
    fn test_last_day7_century_regular() {
        let mut dcf77 = RadioDateTimeUtils::new(7);
        dcf77.year = Some(0);
        dcf77.month = Some(2);
        dcf77.weekday = Some(1);
        assert_eq!(dcf77.last_day(1), Some(28)); // century-regular-year, Monday 2100-02-01
    }
    #[test]
    fn test_last_day7_century_leap_6() {
        let mut dcf77 = RadioDateTimeUtils::new(7);
        dcf77.year = Some(0);
        dcf77.month = Some(2);
        dcf77.weekday = Some(7);
        assert_eq!(dcf77.last_day(6), Some(29)); // century-leap-year, Sunday 2000-02-06
    }
    #[test]
    fn test_last_day0_century_leap() {
        let mut msf = RadioDateTimeUtils::new(0);
        msf.year = Some(0);
        msf.month = Some(2);
        msf.weekday = Some(0);
        assert_eq!(msf.last_day(6), Some(29)); // century-leap-year, Sunday 2000-02-06
    }
    #[test]
    fn test_last_day0_too_large_day() {
        let mut msf = RadioDateTimeUtils::new(0);
        msf.year = Some(0);
        msf.month = Some(2);
        msf.weekday = Some(0);
        assert_eq!(msf.last_day(32), None); // invalid input, Sunday 00-02-32
    }
    #[test]
    fn test_last_day0_none_weekday() {
        let mut msf = RadioDateTimeUtils::new(0);
        msf.year = Some(0);
        msf.month = Some(2);
        assert_eq!(msf.last_day(6), None); // invalid input, None-day 00-02-06
    }

    #[test]
    fn test_dst_some_starting_no_dst_no_announcement_no_jump() {
        let mut rdt = RadioDateTimeUtils::new(0);
        // Simple initial minute update, no announcement:
        rdt.minute = Some(11);
        rdt.set_dst(Some(false), Some(false), false);
        assert_eq!(rdt.dst, Some(0)); // no flags
        assert_eq!(rdt.dst_count, 0);
    }
    #[test]
    fn test_dst_some_starting_dst_no_announcement_no_jump() {
        let mut rdt = RadioDateTimeUtils::new(0);
        // Simple initial minute update, no announcement:
        rdt.minute = Some(11);
        rdt.set_dst(Some(true), Some(false), false);
        assert_eq!(rdt.dst, Some(DST_SUMMER));
        assert_eq!(rdt.dst_count, 0);
    }
    #[test]
    fn test_dst_starting_at_new_hour_no_announcement_jump() {
        let mut rdt = RadioDateTimeUtils::new(0);
        // Simple initial minute update at top-of-hour, no announcement:
        rdt.minute = Some(0);
        rdt.set_dst(Some(false), Some(false), true);
        assert_eq!(rdt.dst, Some(0)); // no flags
        assert_eq!(rdt.dst_count, 0);
    }
    #[test]
    fn test_dst_running_no_announcement_jump() {
        let mut rdt = RadioDateTimeUtils::new(0);
        // A bit further in the hour. no announcement:
        rdt.minute = Some(15);
        rdt.minutes_running = 15;
        rdt.set_dst(Some(false), Some(false), true);
        assert_eq!(rdt.dst, Some(0)); // no flags
        assert_eq!(rdt.dst_count, 0);
    }
    #[test]
    fn test_dst_spurious_announcement_jump() {
        let mut rdt = RadioDateTimeUtils::new(7);
        // DST change announced spuriously:
        rdt.minute = Some(15);
        rdt.minutes_running = 15;
        rdt.set_dst(Some(false), Some(true), true);
        assert_eq!(rdt.dst, Some(0)); // no flags
        assert_eq!(rdt.dst_count, 1);
    }
    #[test]
    fn test_dst_announced() {
        let mut rdt = RadioDateTimeUtils::new(7);
        // Change our mind, the previous announcement was valid:
        // Do not cheat with self.dst_count:
        rdt.minute = Some(0);
        for _ in 0..10 {
            rdt.minute = Some(rdt.minute.unwrap() + 1);
            rdt.minutes_running += 1;
            rdt.set_dst(Some(false), Some(true), true);
        }
        assert_eq!(rdt.dst, Some(DST_ANNOUNCED));
        assert_eq!(rdt.minutes_running, 10);
        assert_eq!(rdt.dst_count, 10);
    }
    #[test]
    fn continue_dst_to_summer() {
        let mut rdt = RadioDateTimeUtils::new(7);
        // Announcement bit was gone, but there should be enough evidence:
        rdt.minute = Some(0);
        for _ in 0..11 {
            rdt.minute = Some(rdt.minute.unwrap() + 1);
            rdt.minutes_running += 1;
            rdt.set_dst(Some(false), Some(true), true);
        }
        assert_eq!(rdt.dst, Some(DST_ANNOUNCED));
        rdt.minute = Some(0);
        rdt.set_dst(Some(true), Some(false), true);
        // Top of hour, so announcement should be reset:
        assert_eq!(rdt.dst, Some(DST_PROCESSED | DST_SUMMER));
        assert_eq!(rdt.dst_count, 0);
    }
    #[test]
    fn continue_dst_to_winter() {
        let mut rdt = RadioDateTimeUtils::new(7);
        // Announcement bit was gone, but there should be enough evidence:
        rdt.minute = Some(0);
        for _ in 0..12 {
            rdt.minute = Some(rdt.minute.unwrap() + 1);
            rdt.minutes_running += 1;
            rdt.set_dst(Some(true), Some(true), true);
        }
        assert_eq!(rdt.dst, Some(DST_ANNOUNCED | DST_SUMMER));
        rdt.minute = Some(0);
        rdt.set_dst(Some(false), Some(false), true);
        // Top of hour, so announcement should be reset:
        assert_eq!(rdt.dst, Some(DST_PROCESSED));
        assert_eq!(rdt.dst_count, 0);
    }
    #[test]
    fn continue2_dst_none_minute() {
        let mut rdt = RadioDateTimeUtils::new(7);
        rdt.minute = Some(0);
        for _ in 0..13 {
            rdt.minute = Some(rdt.minute.unwrap() + 1);
            rdt.minutes_running += 1;
            rdt.set_dst(Some(false), Some(true), true);
        }
        assert_eq!(rdt.dst, Some(DST_ANNOUNCED));
        rdt.minute = Some(0);
        rdt.set_dst(Some(true), Some(false), true);
        // Nothing should change because of the None minute:
        rdt.minute = None;
        rdt.set_dst(Some(true), Some(true), true);
        assert_eq!(rdt.dst, Some(DST_PROCESSED | DST_SUMMER));
        assert_eq!(rdt.dst_count, 1);
    }
    #[test]
    fn continue_dst_jump_no_dst_no_jump() {
        let mut rdt = RadioDateTimeUtils::new(0);
        rdt.minute = Some(11);
        rdt.set_dst(Some(false), Some(false), false);
        assert_eq!(rdt.dst, Some(0));
        assert_eq!(rdt.dst_count, 0);
        rdt.set_dst(Some(true), Some(false), false);
        assert_eq!(rdt.dst, Some(0)); // DST jumped but we do not care
        assert_eq!(rdt.dst_count, 0);
    }
    #[test]
    fn continue_dst_jump_no_dst_jump() {
        let mut rdt = RadioDateTimeUtils::new(0);
        rdt.minute = Some(11);
        rdt.set_dst(Some(false), Some(false), true);
        assert_eq!(rdt.dst, Some(0));
        assert_eq!(rdt.dst_count, 0);
        rdt.set_dst(Some(true), Some(false), true);
        assert_eq!(rdt.dst, Some(DST_JUMP)); // DST jumped
        assert_eq!(rdt.dst_count, 0);
    }
    #[test]
    fn continue_dst_jump_dst_no_jump() {
        let mut rdt = RadioDateTimeUtils::new(0);
        rdt.minute = Some(11);
        rdt.set_dst(Some(true), Some(false), false);
        assert_eq!(rdt.dst, Some(DST_SUMMER));
        assert_eq!(rdt.dst_count, 0);
        rdt.set_dst(Some(false), Some(false), false);
        assert_eq!(rdt.dst, Some(DST_SUMMER)); // DST jumped but we do not care
        assert_eq!(rdt.dst_count, 0);
    }
    #[test]
    fn continue_dst_jump_dst_jump() {
        let mut rdt = RadioDateTimeUtils::new(0);
        rdt.minute = Some(11);
        rdt.set_dst(Some(true), Some(false), true);
        assert_eq!(rdt.dst, Some(DST_SUMMER));
        assert_eq!(rdt.dst_count, 0);
        rdt.set_dst(Some(false), Some(false), true);
        assert_eq!(rdt.dst, Some(DST_JUMP | DST_SUMMER)); // DST jumped
        assert_eq!(rdt.dst_count, 0);
    }

    #[test]
    fn test_leap_second_some_starting_no_announcement() {
        let mut rdt = RadioDateTimeUtils::new(7);
        // Simple initial minute update, no announcement:
        rdt.minute = Some(11);
        rdt.set_leap_second(Some(false), 60);
        assert_eq!(rdt.leap_second, Some(0)); // no flags
        assert_eq!(rdt.leap_second_count, 0);
    }
    #[test]
    fn test_leap_second_starting_at_new_hour_no_announcement() {
        let mut rdt = RadioDateTimeUtils::new(7);
        // Simple initial minute update at top-of-hour, no announcement:
        rdt.minute = Some(0);
        rdt.set_leap_second(Some(false), 60);
        assert_eq!(rdt.leap_second, Some(0)); // no flags
        assert_eq!(rdt.leap_second_count, 0);
    }
    #[test]
    fn test_leap_second_running_no_announcement() {
        let mut rdt = RadioDateTimeUtils::new(7);
        // A bit further in the hour. no announcement:
        rdt.minute = Some(15);
        rdt.minutes_running = 15;
        rdt.set_leap_second(Some(false), 60);
        assert_eq!(rdt.leap_second, Some(0)); // no flags
        assert_eq!(rdt.leap_second_count, 0);
    }
    #[test]
    fn test_leap_second_spurious_announcement() {
        let mut rdt = RadioDateTimeUtils::new(7);
        // Leap second announced spuriously:
        rdt.minute = Some(15);
        rdt.minutes_running = 15;
        rdt.set_leap_second(Some(true), 60);
        assert_eq!(rdt.leap_second, Some(0)); // no flags
        assert_eq!(rdt.leap_second_count, 1);
    }
    #[test]
    fn test_leap_second_announced() {
        let mut rdt = RadioDateTimeUtils::new(7);
        // Change our mind, the previous announcement was valid:
        // Do not cheat with self.leap_second_count:
        rdt.minute = Some(0);
        for _ in 0..10 {
            rdt.minute = Some(rdt.minute.unwrap() + 1);
            rdt.minutes_running += 1;
            rdt.set_leap_second(Some(true), 60);
        }
        assert_eq!(rdt.leap_second, Some(LEAP_ANNOUNCED));
        assert_eq!(rdt.minutes_running, 10);
        assert_eq!(rdt.leap_second_count, 10);
    }
    #[test]
    fn continue2_leap_second_missing() {
        let mut rdt = RadioDateTimeUtils::new(7);
        // Missing leap second.
        // Announcement bit was gone, but there should be enough evidence:
        rdt.minute = Some(0);
        for _ in 0..11 {
            rdt.minute = Some(rdt.minute.unwrap() + 1);
            rdt.minutes_running += 1;
            rdt.set_leap_second(Some(true), 60);
        }
        assert_eq!(rdt.leap_second, Some(LEAP_ANNOUNCED));
        rdt.minute = Some(0);
        rdt.set_leap_second(Some(false), 60 /* not 61 */);
        // Top of hour, so announcement should be reset:
        assert_eq!(rdt.leap_second, Some(LEAP_PROCESSED | LEAP_MISSING));
        assert_eq!(rdt.leap_second_count, 0);
        rdt.minute = Some(1);
        // New hour has started:
        rdt.set_leap_second(Some(false), 60);
        assert_eq!(rdt.leap_second, Some(0));
    }
    #[test]
    fn continue_leap_second_present() {
        let mut rdt = RadioDateTimeUtils::new(7);
        // We got a leap second.
        // Announcement bit was gone, but there should be enough evidence:
        rdt.minute = Some(0);
        for _ in 0..12 {
            rdt.minute = Some(rdt.minute.unwrap() + 1);
            rdt.minutes_running += 1;
            rdt.set_leap_second(Some(true), 60);
        }
        assert_eq!(rdt.leap_second, Some(LEAP_ANNOUNCED));
        rdt.minute = Some(0);
        rdt.set_leap_second(Some(false), 61);
        // Top of hour, so announcement should be reset:
        assert_eq!(rdt.leap_second, Some(LEAP_PROCESSED));
        assert_eq!(rdt.leap_second_count, 0);
    }
    #[test]
    fn continue2_leap_second_none_minute() {
        let mut rdt = RadioDateTimeUtils::new(7);
        rdt.minute = Some(0);
        for _ in 0..13 {
            rdt.minute = Some(rdt.minute.unwrap() + 1);
            rdt.minutes_running += 1;
            rdt.set_leap_second(Some(true), 60);
        }
        assert_eq!(rdt.leap_second, Some(LEAP_ANNOUNCED));
        rdt.minute = Some(0);
        rdt.set_leap_second(Some(false), 60 /* not 61 */);
        // Nothing should happen because of the None minute:
        rdt.minute = None;
        rdt.set_leap_second(Some(true), 61);
        assert_eq!(rdt.leap_second, Some(LEAP_PROCESSED | LEAP_MISSING));
        assert_eq!(rdt.leap_second_count, 1);
    }

    #[test]
    fn test_add_minute_invalid_input() {
        let mut rdt = RadioDateTimeUtils::new(0);
        // Test invalid input:
        assert_eq!(rdt.add_minute(), false);
        assert_eq!(rdt.minute, None);
    }
    #[test]
    fn test_add_minute_century_flip() {
        let mut rdt = RadioDateTimeUtils::new(0);
        // Test the big century flip, these fields must all be set:
        rdt.minute = Some(59);
        rdt.hour = Some(23);
        rdt.day = Some(31);
        rdt.month = Some(12);
        rdt.year = Some(99);
        rdt.weekday = Some(5); // 1999-12-31 is a Friday
        rdt.dst = Some(0); // no flags set, i.e. daylight saving time unset
        assert_eq!(rdt.add_minute(), true);
        assert_eq!(rdt.minute, Some(0));
        assert_eq!(rdt.hour, Some(0));
        assert_eq!(rdt.day, Some(1));
        assert_eq!(rdt.month, Some(1));
        assert_eq!(rdt.year, Some(0));
        assert_eq!(rdt.weekday, Some(6));
    }
    #[test]
    fn test_add_minute_set_dst() {
        let mut rdt = RadioDateTimeUtils::new(0);
        // Test DST becoming active, any hour and date are fine:
        rdt.minute = Some(59);
        rdt.hour = Some(17);
        rdt.day = Some(1);
        rdt.month = Some(1);
        rdt.year = Some(0);
        rdt.weekday = Some(6); // 2000-01-01 is a Saturday
        rdt.dst = Some(DST_ANNOUNCED);
        assert_eq!(rdt.add_minute(), true);
        assert_eq!(rdt.dst, Some(DST_ANNOUNCED)); // add_minute() does not change any DST flag
        assert_eq!(rdt.minute, Some(0));
        assert_eq!(rdt.hour, Some(19));
        assert_eq!(rdt.day, Some(1));
        assert_eq!(rdt.month, Some(1));
        assert_eq!(rdt.year, Some(0));
        assert_eq!(rdt.weekday, Some(6));
    }
    #[test]
    fn test_add_minute_unset_dst() {
        let mut rdt = RadioDateTimeUtils::new(0);
        // Test DST becoming inactive:
        rdt.minute = Some(59);
        rdt.hour = Some(19);
        rdt.day = Some(1);
        rdt.month = Some(1);
        rdt.year = Some(0);
        rdt.weekday = Some(6); // 2000-01-01 is a Saturday
        rdt.dst = Some(DST_SUMMER | DST_ANNOUNCED);
        assert_eq!(rdt.add_minute(), true);
        assert_eq!(rdt.dst, Some(DST_SUMMER | DST_ANNOUNCED)); // add_minute() does not change any DST flag
        assert_eq!(rdt.minute, Some(0));
        assert_eq!(rdt.hour, Some(19));
        assert_eq!(rdt.day, Some(1));
        assert_eq!(rdt.month, Some(1));
        assert_eq!(rdt.year, Some(0));
        assert_eq!(rdt.weekday, Some(6));
    }
    #[test]
    fn test_add_minute_msf_saturday_sunday() {
        let mut rdt = RadioDateTimeUtils::new(0);
        // Test flipping to min_weekday (MSF), Saturday 6 -> Sunday 0:
        rdt.minute = Some(59);
        rdt.hour = Some(23);
        rdt.day = Some(1);
        rdt.month = Some(1);
        rdt.year = Some(0);
        rdt.weekday = Some(6); // 2000-01-01 is a Saturday
        rdt.dst = Some(0);
        assert_eq!(rdt.add_minute(), true);
        assert_eq!(rdt.minute, Some(0));
        assert_eq!(rdt.hour, Some(0));
        assert_eq!(rdt.day, Some(2));
        assert_eq!(rdt.month, Some(1));
        assert_eq!(rdt.year, Some(0));
        assert_eq!(rdt.weekday, Some(0));
    }
    #[test]
    fn test_add_minute_dcf77_sunday_monday() {
        // Test flipping to min_weekday (DCF77), Sunday 7 -> Monday 1:
        let mut rdt = RadioDateTimeUtils::new(7);
        rdt.minute = Some(59);
        rdt.hour = Some(23);
        rdt.day = Some(2);
        rdt.month = Some(1);
        rdt.year = Some(0);
        rdt.weekday = Some(7); // 2000-01-02 is a Sunday
        rdt.dst = Some(0); // no flags set, i.e. daylight saving time unset
        assert_eq!(rdt.add_minute(), true);
        assert_eq!(rdt.minute, Some(0));
        assert_eq!(rdt.hour, Some(0));
        assert_eq!(rdt.day, Some(3));
        assert_eq!(rdt.month, Some(1));
        assert_eq!(rdt.year, Some(0));
        assert_eq!(rdt.weekday, Some(1));
    }
}
