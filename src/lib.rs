//! Definition of date/time structures commonly useful for time station decoders.

//! Build with no_std for embedded platforms.
#![cfg_attr(not(test), no_std)]

use heapless::Vec;

/**
 * Return the difference in microseconds between two timestamps.
 *
 * This function takes wrapping of the parameters into account,
 * as they are u32, so they wrap each 71m35.
 *
 * # Arguments
 * * `t0` - old timestamp in microseconds
 * * `t1` - new timestamp in microseconds
 */
pub fn time_diff(t0: u32, t1: u32) -> u32 {
    if t1 == t0 {
        0
    } else if t1 > t0 {
        t1 - t0
    } else if t0 > 0 {
        u32::MAX - t0 + t1 + 1 // wrapped, each 1h11m35s
    } else {
        0 // cannot happen, because t1 < t0 && t0 == 0, but prevents E0317 on Rust 1.61
    }
}

/**
 * Returns the BCD-encoded value of the given buffer over the given range, or None if the input is invalid.
 *
 * # Arguments
 * * `bit_buffer` - buffer containing the bits
 * * `start` - start bit position (least significant)
 * * `stop` - stop bit position (most significant)
 */
pub fn get_bcd_value(bit_buffer: &[Option<bool>], start: usize, stop: usize) -> Option<u8> {
    const MAX_RANGE: usize = 8;
    let (p0, p1) = min_max(start, stop);
    if p1 - p0 >= MAX_RANGE {
        return None;
    }
    let mut r: Vec<bool, MAX_RANGE> = Vec::new();
    for b in &bit_buffer[p0..=p1] {
        if b.is_none() || r.push(b.unwrap()).is_err() {
            return None;
        }
    }
    if stop < start {
        r.reverse();
    }

    let mut bcd = 0;
    let mut mult = 1;
    for bit in r {
        bcd += mult * if bit { 1 } else { 0 };
        mult *= 2;
        if mult == 16 {
            if bcd > 9 {
                return None;
            }
            mult = 10;
        }
    }
    if bcd < 100 {
        Some(bcd)
    } else {
        None
    }
}

/**
 * Returns even parity of the given buffer over the given range, or None if the input is invalid.
 *
 * # Arguments
 * * `bit_buffer` - buffer containing the bits to check.
 * * `start` - start bit position
 * * `stop` - stop bit position
 * * `parity` - parity bit value
 */
pub fn get_parity(
    bit_buffer: &[Option<bool>],
    start: usize,
    stop: usize,
    parity: Option<bool>,
) -> Option<bool> {
    let (p0, p1) = min_max(start, stop);
    parity?;
    let mut par = parity.unwrap();
    for bit in &bit_buffer[p0..=p1] {
        (*bit)?;
        par ^= bit.unwrap();
    }
    Some(par)
}

/**
 * Return a tuple of the two parameters in ascending order.
 *
 * # Arguments
 * * `a` - first argument
 * * `b` - second argument
*/
#[inline]
fn min_max(a: usize, b: usize) -> (usize, usize) {
    if a < b {
        (a, b)
    } else {
        (b, a)
    }
}

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
    dst_count: u8, // internal counter for set_dst()
    leap_second: Option<u8>,
    leap_second_count: u8, // internal counter for set_leap_second()
    jump_year: bool,
    jump_month: bool,
    jump_day: bool,
    jump_weekday: bool,
    jump_hour: bool,
    jump_minute: bool,
    min_weekday: u8,
    max_weekday: u8,
    minutes_running: u8, // internal counter for set_dst() and set_leap_second()
    first_minute: bool,  // internal flag for set_dst()
}

impl RadioDateTimeUtils {
    /**
     * Initialize a new RadioDateTimeUtils instance
     *
     * # Arguments
     * * `sunday` - the numeric value of Sunday, i.e. 7 for DCF77 or 0 for NPL
     */
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
            min_weekday: if sunday == 0 { 0 } else { 1 },
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

    /**
     * Adds one minute to the current date and time, returns if the operation succeeded.
     *
     * * Years are limited to 2 digits, so this function wraps after 100 years.
     */
    pub fn add_minute(&mut self) -> bool {
        if self.minute.is_none()
            || self.hour.is_none()
            || self.day.is_none()
            || self.weekday.is_none()
            || self.month.is_none()
            || self.year.is_none()
            || self.dst.is_none()
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
                let old_last_day = self.last_day(s_day);
                if old_last_day.is_none() {
                    return false;
                }
                s_weekday += 1;
                if s_weekday == self.max_weekday + 1 {
                    s_weekday = self.min_weekday;
                }
                s_day += 1;
                if s_day > old_last_day.unwrap() {
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

    /**
     * Set the year value, valid values are 0 through 99.
     *
     * # Arguments
     * * `value` - the new year value. None or invalid values keep the old value.
     * * `valid` - extra validation to pass.
     * * `check_jump` - check if the value has jumped unexpectedly compared to `add_minute()`.
     */
    pub fn set_year(&mut self, value: Option<u8>, valid: bool, check_jump: bool) {
        let year = if value.is_some() && (0..=99).contains(&value.unwrap()) && valid {
            value
        } else {
            self.year
        };
        self.jump_year = check_jump && year.is_some() && self.year.is_some() && year != self.year;
        self.year = year;
    }

    /**
     * Set the month value, valid values are 1 through 12.
     *
     * # Arguments
     * * `value` - the new month value. None or invalid values keep the old value.
     * * `valid` - extra validation to pass.
     * * `check_jump` - check if the value has jumped unexpectedly compared to `add_minute()`.
     */
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

    /**
     * Set the day-of-week value, valid values are 0/1 through 6/7, depending on how this
     * instance was created.
     *
     * # Arguments
     * * `value` - the new day-of-week value. None or invalid values keep the old value.
     * * `valid` - extra validation to pass.
     * * `check_jump` - check if the value has jumped unexpectedly compared to `add_minute()`.
     */
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

    /**
     * Set the day-in-month value, valid values are 1 through 31.
     *
     * If possible, this function further restricts the range of valid days to the last day
     * in the current month, taking leap years into account.
     *
     * # Arguments
     * * `value` - the new day-in-month value. None or invalid values keep the old value.
     * * `valid` - extra validation to pass.
     * * `check_jump` - check if the value has jumped unexpectedly compared to `add_minute()`.
     */
    pub fn set_day(&mut self, value: Option<u8>, valid: bool, check_jump: bool) {
        let days_in_month = if let Some(s_value) = value {
            self.last_day(s_value)
        } else {
            Some(31)
        };
        let day = if value.is_some()
            && days_in_month.is_some()
            && (1..=days_in_month.unwrap()).contains(&value.unwrap())
            && valid
        {
            value
        } else {
            self.day
        };
        self.jump_day = check_jump && day.is_some() && self.day.is_some() && day != self.day;
        self.day = day;
    }

    /**
     * Set the hour value, valid values are 0 through 23.
     *
     * # Arguments
     * * `value` - the new hour value. None or invalid values keep the old value.
     * * `valid` - extra validation to pass.
     * * `check_jump` - check if the value has jumped unexpectedly compared to `add_minute()`.
     */
    pub fn set_hour(&mut self, value: Option<u8>, valid: bool, check_jump: bool) {
        let hour = if value.is_some() && (0..=23).contains(&value.unwrap()) && valid {
            value
        } else {
            self.hour
        };
        self.jump_hour = check_jump && hour.is_some() && self.hour.is_some() && hour != self.hour;
        self.hour = hour;
    }

    /**
     * Set the minute value, valid values are 0 through 59.
     *
     * # Arguments
     * * `value` - the new minute value. None or invalid values keep the old value.
     * * `valid` - extra validation to pass.
     * * `check_jump` - check if the value has jumped unexpectedly compared to `add_minute()`.
     */
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

    /**
     * Set the DST mask value, both the actual value and any information on transitions.
     *
     * # Arguments
     * * `value` - the new DST value. None or unannounced changes keep the old value.
     * * `announce` - if any announcement is made on a transition. The history of this
     *                value of the last hour (or part thereof if started later) is kept
     *                to compensate for spurious True values.
     * * `check_jump` - check if the value changed unexpectedly.
     */
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
                self.first_minute = false;
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
            // DST change processsed:
            self.dst = Some(self.dst.unwrap() | DST_PROCESSED);
        } else {
            self.dst = Some(self.dst.unwrap() & !DST_PROCESSED);
        }
        // Always reset announcement at the hour:
        if self.minute == Some(0) {
            self.dst = Some(self.dst.unwrap() & !DST_ANNOUNCED);
            self.dst_count = 0;
        }
    }

    /**
     * Set the leap second value.
     *
     * # Arguments
     * * `announce` - if any announcement is made on a positive leap second. The history
     *                of this value of the last hour (or part thereof if started later) is
     *                kept to compensate for spurious Some(True) values.
     * * `minute_length` - the length of the decoded minute in seconds.
     */
    pub fn set_leap_second(&mut self, announce: Option<bool>, minute_length: u8) {
        if announce.is_none() {
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
        } else {
            self.leap_second = Some(self.leap_second.unwrap() & !LEAP_PROCESSED);
        }
        // Always reset announcement at the hour:
        if self.minute == Some(0) {
            self.leap_second = Some(self.leap_second.unwrap() & !LEAP_ANNOUNCED);
            self.leap_second_count = 0;
        }
    }

    /**
     * Bump the internal minute counter needed for set_dst() and set_leap_second()
     *
     * The code above this library must call this function, as this library cannot
     * know which function got called first, or if just one of them should be called.
     */
    pub fn bump_minutes_running(&mut self) {
        self.minutes_running += 1;
        if self.minutes_running == 60 {
            self.minutes_running = 0;
        }
    }

    /**
     * Returns the last calendar day of the current date, or None in case of error.
     *
     * # Arguments
     * * `day` - day of the month in February '00, used to see if `year` is a leap year
     */
    fn last_day(&self, day: u8) -> Option<u8> {
        if let Some(s_year) = self.year {
            if let Some(s_month) = self.month {
                if let Some(s_weekday) = self.weekday {
                    if !(1..=31).contains(&day) {
                        None
                    } else if s_month == 2 {
                        if (s_year != 0 && s_year % 4 == 0)
                            || (s_year == 0 && is_leap_century(day, s_weekday))
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
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        }
    }
}

/**
 * Checks if the century based on the given date is divisible by 400.
 *
 * Based on xx00-02-28 is a Monday <=> xx00 is a leap year
 *
 * # Arguments
 * * `day` - day of the month in February '00
 * * `weekday` - day of the week in February '00
 */
fn is_leap_century(day: u8, weekday: u8) -> bool {
    let mut wd = weekday % 7;
    if wd == 0 {
        wd = 7;
    }

    // Week day 1 is a Monday, assume this is a leap year.
    // If so, we should reach Monday xx00-02-28
    if day < 29 {
        day + 7 * ((28 - day) / 7) + 8 - wd == 28
    } else {
        day - 7 * ((day - 28) / 7) + 1 - wd == 28
    }
}

#[cfg(test)]
mod tests {
    use crate::{get_bcd_value, get_parity, time_diff, RadioDateTimeUtils};

    #[test]
    fn test_time_diff() {
        assert_eq!(time_diff(2, 3), 1);
        assert_eq!(time_diff(0, 3), 3);
        assert_eq!(time_diff(u32::MAX - 100, 0), 101); // flipped
        assert_eq!(time_diff(u32::MAX - 100, 100), 201); // also flipped
        assert_eq!(time_diff(2, 2), 0);
    }

    const BIT_BUFFER: [Option<bool>; 10] = [
        Some(false),
        Some(true),
        Some(false),
        Some(false),
        Some(true),
        Some(true),
        Some(true),
        Some(true),
        None,
        Some(false),
    ];

    #[test]
    fn test_get_bcd_value() {
        assert_eq!(get_bcd_value(&BIT_BUFFER[0..=4], 0, 4), Some(12));
        assert_eq!(get_bcd_value(&BIT_BUFFER[1..=1], 0, 0), Some(1)); // single-bit value, must be a slice
        assert_eq!(get_bcd_value(&BIT_BUFFER[0..=7], 0, 7), None); // too large for BCD, test 8 bit range
        assert_eq!(get_bcd_value(&BIT_BUFFER[4..=7], 0, 3), None); // too large for BCD
        assert_eq!(get_bcd_value(&BIT_BUFFER[7..=9], 0, 2), None); // has a None value
        assert_eq!(get_bcd_value(&BIT_BUFFER, 0, 9), None); // range too wide
        assert_eq!(get_bcd_value(&BIT_BUFFER[0..=5], 5, 0), Some(13)); // backwards
        assert_ne!(get_bcd_value(&BIT_BUFFER[0..=5], 5, 0), Some(32)); // backwards with forwards result
    }

    #[test]
    fn test_get_parity() {
        assert_eq!(
            get_parity(&BIT_BUFFER[0..=4], 0, 3, BIT_BUFFER[4]),
            Some(false)
        );
        assert_eq!(get_parity(&BIT_BUFFER[7..=9], 0, 1, BIT_BUFFER[2]), None); // has a None value
        assert_eq!(
            get_parity(&BIT_BUFFER[0..=3], 0, 2, BIT_BUFFER[3]),
            Some(true)
        );
        assert_eq!(
            get_parity(&BIT_BUFFER[0..=3], 3, 1, BIT_BUFFER[0]),
            Some(true)
        ); // backwards
    }

    #[test]
    fn test_last_day() {
        let mut dcf77 = RadioDateTimeUtils::new(7);
        dcf77.set_year(Some(22), true, false);
        dcf77.set_month(Some(6), true, false);
        dcf77.set_weekday(Some(7), true, false);
        assert_eq!(dcf77.last_day(5), Some(30)); // today, Sunday 2022-06-05
        dcf77.set_month(Some(2), true, false);
        dcf77.set_weekday(Some(4), true, false);
        assert_eq!(dcf77.last_day(29), Some(28)); // non-existent date, Thursday 22-02-29
        dcf77.set_year(Some(0), true, false);
        dcf77.set_month(Some(1), true, false);
        dcf77.set_weekday(Some(1), true, false);
        assert_eq!(dcf77.last_day(1), Some(31)); // first day, weekday off/do-not-care, Monday 00-01-01
        dcf77.set_year(Some(20), true, false);
        dcf77.set_month(Some(2), true, false);
        assert_eq!(dcf77.last_day(3), Some(29)); // regular leap year, Wednesday 2020-02-03
        dcf77.set_weekday(Some(4), true, false);
        assert_eq!(dcf77.last_day(3), Some(29)); // same date with bogus weekday, "Thursday" 2020-02-03
        dcf77.set_year(Some(0), true, false);
        dcf77.set_weekday(Some(2), true, false);
        assert_eq!(dcf77.last_day(1), Some(29)); // century-leap-year, day/weekday must match, Tuesday 2000-02-01
        dcf77.set_weekday(Some(1), true, false);
        assert_eq!(dcf77.last_day(1), Some(28)); // century-regular-year, Monday 2100-02-01
        dcf77.set_weekday(Some(7), true, false);
        assert_eq!(dcf77.last_day(6), Some(29)); // century-leap-year, Sunday 2000-02-06
        let mut npl = RadioDateTimeUtils::new(0);
        npl.set_year(Some(0), true, false);
        npl.set_month(Some(2), true, false);
        npl.set_weekday(Some(0), true, false);
        assert_eq!(npl.last_day(6), Some(29)); // century-leap-year, Sunday 2000-02-06
    }
}
