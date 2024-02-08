/// Return the difference in microseconds between two timestamps.
///
/// This function takes wrapping of the parameters into account,
/// as they are u32, so they wrap each 71m35.
///
/// # Arguments
/// * `t0` - old timestamp in microseconds
/// * `t1` - new timestamp in microseconds
pub fn time_diff(t0: u32, t1: u32) -> u32 {
    if t1 >= t0 {
        t1 - t0
    } else if t0 > 0 {
        u32::MAX - t0 + t1 + 1 // wrapped, each 1h11m35s
    } else {
        0 // cannot happen, because t1 < t0 && t0 == 0, but prevents E0317 (missing else clause)
    }
}

/// Returns the BCD-encoded value of the given buffer over the given range, or None if the input is invalid.
///
/// # Arguments
/// * `bit_buffer` - buffer containing the bits
/// * `start` - start bit position (least significant)
/// * `stop` - stop bit position (most significant)
pub fn get_bcd_value(bit_buffer: &[Option<bool>], start: usize, stop: usize) -> Option<u8> {
    const MAX_RANGE: usize = 8;
    let (p0, p1) = min_max(start, stop);
    if p1 - p0 >= MAX_RANGE {
        return None;
    }
    let mut bcd = 0;
    let mut mult = 1;
    // Index the bits using a manual loop instead of enumerating them in a range.
    // Doing so obsoletes the need to first flip the range if start > stop.
    let mut idx = start;
    let step: isize = if start < stop { 1 } else { -1 };
    // The test value for idx is usize::MAX if stop is 0, but we stop just in time.
    while idx != (stop as isize + step) as usize {
        let bit = bit_buffer[idx]?;
        bcd += mult * bit as u8;
        mult *= 2;
        if mult == 16 {
            if bcd > 9 {
                return None;
            }
            mult = 10;
        }
        idx = (idx as isize + step) as usize;
    }
    if bcd < 100 {
        Some(bcd)
    } else {
        None
    }
}

/// Returns parity of the given buffer over the given range, or None if the input is invalid.
/// Should be Some(false) for even parity and Some(true) for odd parity.
///
/// # Arguments
/// * `bit_buffer` - buffer containing the bits to check.
/// * `start` - start bit position
/// * `stop` - stop bit position
/// `parity` - parity bit value
pub fn get_parity(
    bit_buffer: &[Option<bool>],
    start: usize,
    stop: usize,
    parity: Option<bool>,
) -> Option<bool> {
    parity?;
    let mut s_parity = parity.unwrap();
    let (p0, p1) = min_max(start, stop);
    for bit in &bit_buffer[p0..=p1] {
        (*bit)?;
        s_parity ^= bit.unwrap();
    }
    Some(s_parity)
}

/// Return a tuple of the two parameters in ascending order.
///
/// # Arguments
/// * `a` - first argument
/// * `b` - second argument
fn min_max(a: usize, b: usize) -> (usize, usize) {
    if a < b {
        (a, b)
    } else {
        (b, a)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_time_diff_difference_1() {
        assert_eq!(time_diff(2, 3), 1);
    }
    #[test]
    fn test_time_diff_difference_3() {
        assert_eq!(time_diff(0, 3), 3);
    }
    #[test]
    fn test_time_diff_flipped_m100_0() {
        assert_eq!(time_diff(u32::MAX - 100, 0), 101);
    }
    #[test]
    fn test_time_diff_flipped_m100_100() {
        assert_eq!(time_diff(u32::MAX - 100, 100), 201);
    }
    #[test]
    fn test_time_diff_zero() {
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
    fn ok_get_bcd_value_regular() {
        assert_eq!(get_bcd_value(&BIT_BUFFER[0..=4], 0, 4), Some(12));
    }
    #[test]
    fn ok_get_bcd_value_single_bit() {
        assert_eq!(get_bcd_value(&BIT_BUFFER[1..=1], 0, 0), Some(1)); // single-bit value, must be a slice
    }
    #[test]
    fn bad_get_bcd_value_too_large_total_bcd() {
        assert_eq!(get_bcd_value(&BIT_BUFFER[0..=7], 0, 7), None);
    }
    #[test]
    fn bad_get_bcd_value_too_large_single_bcd() {
        assert_eq!(get_bcd_value(&BIT_BUFFER[4..=7], 0, 3), None);
    }
    #[test]
    fn bad_get_bcd_value_none() {
        assert_eq!(get_bcd_value(&BIT_BUFFER[7..=9], 0, 2), None);
    }
    #[test]
    fn bad_get_bcd_value_too_wide() {
        assert_eq!(get_bcd_value(&BIT_BUFFER, 0, 9), None);
    }
    #[test]
    fn ok_get_bcd_value_backwards() {
        assert_eq!(get_bcd_value(&BIT_BUFFER[0..=5], 5, 0), Some(13));
    }

    #[test]
    fn ok_get_parity_regular_even() {
        assert_eq!(
            get_parity(&BIT_BUFFER[0..=4], 0, 3, BIT_BUFFER[4]),
            Some(false)
        );
    }
    #[test]
    fn bad_get_parity_none() {
        assert_eq!(get_parity(&BIT_BUFFER[7..=9], 0, 1, BIT_BUFFER[2]), None);
    }
    #[test]
    fn ok_get_parity_regular_odd() {
        assert_eq!(
            get_parity(&BIT_BUFFER[0..=3], 0, 2, BIT_BUFFER[3]),
            Some(true)
        );
    }
    #[test]
    fn ok_get_parity_backwards() {
        assert_eq!(
            get_parity(&BIT_BUFFER[0..=3], 3, 1, BIT_BUFFER[0]),
            Some(true)
        );
    }
}
