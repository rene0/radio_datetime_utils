//! Definition of date/time structures commonly useful for time station decoders.

//! Build with no_std for embedded platforms.
#![cfg_attr(not(test), no_std)]

use core::fmt::Write;
use heapless::{String, Vec};

/**
 * Return the difference in microseconds between two timestamps.
 *
 * This function takes wrapping of the parameters into account,
 * as they are u32 so they wrap each 71m35.
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
 * * `parity` - parity bit position, must be outside of start..=stop (or stop..=start)
 */
pub fn get_parity(
    bit_buffer: &[Option<bool>],
    start: usize,
    stop: usize,
    parity: usize,
) -> Option<bool> {
    let (p0, p1) = min_max(start, stop);
    if bit_buffer[parity].is_none() || (p0..=p1).contains(&parity) {
        return None;
    }
    let mut par = bit_buffer[parity].unwrap();
    for bit in &bit_buffer[p0..=p1] {
        (*bit)?;
        par ^= bit.unwrap();
    }
    Some(par)
}

#[inline]
fn min_max(a: usize, b: usize) -> (usize, usize) {
    if a < b {
        (a, b)
    } else {
        (b, a)
    }
}

/**
 * Returns the last calendar day of the given year and month, or None in case of error.
 *
 * # Arguments
 * * `year` - year of the given date
 * * `month` - month of the given date
 * * `day` - day of the month in February '00, used to see if `year` is a leap year
 * * `weekday` - day of the week in February '00, used to see if `year` is a leap year
 */
fn last_day(year: u8, month: u8, day: u8, weekday: u8) -> Option<u8> {
    if !(0..=99).contains(&year)
        || !(1..=12).contains(&month)
        || !(1..=31).contains(&day)
        || !(0..=7).contains(&weekday)
    {
        None
    } else if month == 2 {
        if (year != 0 && year % 4 == 0) || (year == 0 && is_leap_century(day, weekday)) {
            Some(29)
        } else {
            Some(28)
        }
    } else if month == 4 || month == 6 || month == 9 || month == 11 {
        Some(30)
    } else {
        Some(31)
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

// Only used with DCF77 :
/// No leap second expected or present
pub const LEAP_NONE: u8 = 0;
/// Leap second has been announced
pub const LEAP_ANNOUNCED: u8 = 1;
/// Leap second has been processed
pub const LEAP_PROCESSED: u8 = 2;
/// Leap second bit value is 1 instead of 0
pub const LEAP_NON_ZERO: u8 = 4;
/// Leap second is unexpectedly absent
pub const LEAP_MISSING: u8 = 8;

/// Represents a date and time transmitted over radio.
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
}

impl RadioDateTimeUtils {
    /**
     * Initialize a new RadioDateTimeUtils instance
     *
     * # Arguments
     * * `min_weekday` - the numeric value of the first day of the week, e.g.,
     *                   1 (Monday) for DCF77 or 0 (Sunday) for NPL
     * * `max_weekday` - the numeric value of the last day of the week, e.g.,
     *                   7 (Sunday) for DCF77 or 6 (Saturday) for NPL
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
            leap_second: None,
            jump_year: false,
            jump_month: false,
            jump_day: false,
            jump_weekday: false,
            jump_hour: false,
            jump_minute: false,
            min_weekday: if sunday == 0 { 0 } else { 1 },
            max_weekday: if sunday == 7 { 7 } else { 6 },
        }
    }

    pub fn get_year(&self) -> Option<u8> {
        self.year
    }

    pub fn str_year(&self) -> String<2> {
        let mut s = String::<2>::from("");
        if self.year.is_some() {
            write!(s, "{:>02}", self.year.unwrap()).unwrap();
        } else {
            write!(s, "**").unwrap();
        }
        s
    }

    pub fn get_month(&self) -> Option<u8> {
        self.month
    }

    pub fn str_month(&self) -> String<2> {
        let mut s = String::<2>::from("");
        if self.month.is_some() {
            write!(s, "{:>02}", self.month.unwrap()).unwrap();
        } else {
            write!(s, "**").unwrap();
        }
        s
    }

    pub fn get_day(&self) -> Option<u8> {
        self.day
    }

    pub fn str_day(&self) -> String<2> {
        let mut s = String::<2>::from("");
        if self.day.is_some() {
            write!(s, "{:>02}", self.day.unwrap()).unwrap();
        } else {
            write!(s, "**").unwrap();
        }
        s
    }

    pub fn get_weekday(&self) -> Option<u8> {
        self.weekday
    }

    /// Return a textual representation of the weekday
    pub fn str_weekday(&self) -> String<2> {
        String::<2>::from(match self.weekday {
            Some(0) => "Su",
            Some(1) => "Mo",
            Some(2) => "Tu",
            Some(3) => "We",
            Some(4) => "Th",
            Some(5) => "Fr",
            Some(6) => "Sa",
            Some(7) => "Su",
            _ => "**",
        })
    }

    pub fn get_hour(&self) -> Option<u8> {
        self.hour
    }

    pub fn str_hour(&self) -> String<2> {
        let mut s = String::<2>::from("");
        if self.hour.is_some() {
            write!(s, "{:>02}", self.hour.unwrap()).unwrap();
        } else {
            write!(s, "**").unwrap();
        }
        s
    }

    pub fn get_minute(&self) -> Option<u8> {
        self.minute
    }

    pub fn str_minute(&self) -> String<2> {
        let mut s = String::<2>::from("");
        if self.minute.is_some() {
            write!(s, "{:>02}", self.minute.unwrap()).unwrap();
        } else {
            write!(s, "**").unwrap();
        }
        s
    }

    pub fn get_dst(&self) -> Option<u8> {
        self.dst
    }

    pub fn get_leap_second(&self) -> Option<u8> {
        self.leap_second
    }

    pub fn get_jump_year(&self) -> bool {
        self.jump_year
    }

    pub fn str_jump_year(&self) -> char {
        if self.jump_year {
            'y'
        } else {
            ' '
        }
    }

    pub fn get_jump_month(&self) -> bool {
        self.jump_month
    }

    pub fn str_jump_month(&self) -> char {
        if self.jump_month {
            'm'
        } else {
            ' '
        }
    }

    pub fn get_jump_day(&self) -> bool {
        self.jump_day
    }

    pub fn str_jump_day(&self) -> char {
        if self.jump_day {
            'd'
        } else {
            ' '
        }
    }

    pub fn get_jump_weekday(&self) -> bool {
        self.jump_weekday
    }

    pub fn str_jump_weekday(&self) -> char {
        if self.jump_weekday {
            'w'
        } else {
            ' '
        }
    }

    pub fn get_jump_hour(&self) -> bool {
        self.jump_hour
    }

    pub fn str_jump_hour(&self) -> char {
        if self.jump_hour {
            'h'
        } else {
            ' '
        }
    }

    pub fn get_jump_minute(&self) -> bool {
        self.jump_minute
    }

    pub fn str_jump_minute(&self) -> char {
        if self.jump_minute {
            'm'
        } else {
            ' '
        }
    }

    pub fn str_jump_dst(&self) -> char {
        if self.dst.is_some() && (self.dst.unwrap() & DST_JUMP) != 0 {
            't'
        } else {
            ' '
        }
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
        let mut t_minute = self.minute.unwrap();
        let mut t_hour = self.hour.unwrap();
        let mut t_day = self.day.unwrap();
        let mut t_weekday = self.weekday.unwrap();
        let mut t_month = self.month.unwrap();
        let mut t_year = self.year.unwrap();
        t_minute += 1;
        if t_minute == 60 {
            t_minute = 0;
            if (self.dst.unwrap() & DST_ANNOUNCED) != 0 {
                if (self.dst.unwrap() & DST_SUMMER) != 0 {
                    t_hour -= 1; // changing to winter
                } else {
                    t_hour += 1; // changing to summer
                }
            }
            t_hour += 1;
            if t_hour == 24 {
                t_hour = 0;
                let old_last_day = last_day(t_year, t_month, t_day, t_weekday);
                if old_last_day.is_none() {
                    return false;
                }
                t_weekday += 1;
                if t_weekday == self.max_weekday + 1 {
                    t_weekday = self.min_weekday;
                }
                t_day += 1;
                if t_day > old_last_day.unwrap() {
                    t_day = 1;
                    t_month += 1;
                    if t_month == 13 {
                        t_month = 1;
                        t_year += 1;
                        if t_year == 100 {
                            t_year = 0;
                        }
                    }
                }
            }
        }
        self.minute = Some(t_minute);
        self.hour = Some(t_hour);
        self.day = Some(t_day);
        self.weekday = Some(t_weekday);
        self.month = Some(t_month);
        self.year = Some(t_year);
        true
    }

    pub fn set_year(&mut self, value: Option<u8>, valid: bool, check_jump: bool) {
        let year = if value.is_some() && (0..=99).contains(&value.unwrap()) && valid {
            value
        } else {
            self.year
        };
        self.jump_year = check_jump && year != self.year;
        self.year = year;
    }

    pub fn set_month(&mut self, value: Option<u8>, valid: bool, check_jump: bool) {
        let month = if value.is_some() && (1..=12).contains(&value.unwrap()) && valid {
            value
        } else {
            self.month
        };
        self.jump_month = check_jump && month != self.month;
        self.month = month;
    }

    pub fn set_weekday(&mut self, value: Option<u8>, valid: bool, check_jump: bool) {
        let weekday = if value.is_some()
            && (self.min_weekday..=self.max_weekday).contains(&value.unwrap())
            && valid
        {
            value
        } else {
            self.weekday
        };
        self.jump_weekday = check_jump && weekday != self.weekday;
        self.weekday = weekday;
    }

    pub fn set_day(&mut self, value: Option<u8>, valid: bool, check_jump: bool) {
        let mut day = self.day;
        let mut days_in_month = Some(31);

        if let Some(s_year) = self.year {
            if let Some(s_month) = self.month {
                if let Some(s_value) = value {
                    if let Some(s_weekday) = self.weekday {
                        days_in_month = last_day(s_year, s_month, s_value, s_weekday);
                    }
                }
            }
        }
        if days_in_month.is_some()
            && value.is_some()
            && (1..=days_in_month.unwrap()).contains(&value.unwrap())
            && valid
        {
            day = value;
        }
        self.jump_day = check_jump && day != self.day;
        self.day = day;
    }

    pub fn set_hour(&mut self, value: Option<u8>, valid: bool, check_jump: bool) {
        let hour = if value.is_some() && (0..=23).contains(&value.unwrap()) && valid {
            value
        } else {
            self.hour
        };
        self.jump_hour = check_jump && hour != self.hour;
        self.hour = hour;
    }

    pub fn set_minute(&mut self, value: Option<u8>, valid: bool, check_jump: bool) {
        let minute = if value.is_some() && (0..=59).contains(&value.unwrap()) && valid {
            value
        } else {
            self.minute
        };
        self.jump_minute = check_jump && minute != self.minute;
        self.minute = minute;
    }

    /// Returns a character representation of the current DST status.
    pub fn str_dst(&self) -> char {
        if self.dst.is_none() {
            return 'x';
        }
        let mut dst = if (self.dst.unwrap() & DST_SUMMER) != 0 {
            's'
        } else {
            'w'
        };
        if (self.dst.unwrap() & (DST_ANNOUNCED | DST_PROCESSED)) != 0 {
            dst = dst.to_ascii_uppercase();
        }
        dst
    }

    /// Returns a character representation of the current leap second status.
    pub fn str_leap_second(&self) -> char {
        if self.leap_second.is_none() {
            return 'x';
        }
        if (self.leap_second.unwrap() & LEAP_PROCESSED) != 0 {
            'L'
        } else if (self.leap_second.unwrap() & LEAP_NON_ZERO) != 0 {
            '1'
        } else if (self.leap_second.unwrap() & LEAP_MISSING) != 0 {
            'm'
        } else if (self.leap_second.unwrap() & LEAP_ANNOUNCED) != 0 {
            'l'
        } else {
            ' ' // LEAP_NONE
        }
    }
}

/// Checks if the century based on the given date is divisible by 400.
/// Based on xx00-02-28 is a Monday <=> xx00 is a leap year
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
    use crate::{get_bcd_value, get_parity, last_day, time_diff};

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
        assert_eq!(get_parity(&BIT_BUFFER[0..=4], 0, 3, 4), Some(false));
        assert_eq!(get_parity(&BIT_BUFFER[0..=4], 0, 4, 3), None); // parity in middle of range
        assert_eq!(get_parity(&BIT_BUFFER[7..=9], 0, 1, 2), None); // has a None value
        assert_eq!(get_parity(&BIT_BUFFER[0..=3], 0, 2, 3), Some(true));
        assert_eq!(get_parity(&BIT_BUFFER[0..=3], 3, 1, 0), Some(true)); // backwards
    }

    #[test]
    fn test_last_day() {
        assert_eq!(last_day(22, 6, 5, 7), Some(30)); // today
        assert_eq!(last_day(100, 6, 5, 7), None); // year too large
        assert_eq!(last_day(22, 2, 29, 4), Some(28)); // non-existent date
        assert_eq!(last_day(0, 1, 1, 1), Some(31)); // first day, weekday off/do-not-care
        assert_eq!(last_day(20, 2, 3, 1), Some(29)); // regular leap year
        assert_eq!(last_day(20, 2, 3, 4), Some(29)); // same date with bogus weekday
        assert_eq!(last_day(0, 2, 1, 2), Some(29)); // century-leap-year, day/weekday must match, this Tuesday 2000-02-01
        assert_eq!(last_day(0, 2, 1, 1), Some(28)); // century-regular-year, Monday 2100-02-01
        assert_eq!(last_day(0, 2, 6, 7), Some(29)); // century-leap-year, Sunday 2000-02-06, DCF77
        assert_eq!(last_day(0, 2, 6, 0), Some(29)); // century-leap-year, Sunday 2000-02-06, NPL
    }
}
