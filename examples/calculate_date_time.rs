use radio_datetime_utils::radio_datetime_helpers;

fn main() {
    // Calculate parity and value of some bits

    const BITS: [Option<bool>; 7] = [
        Some(false),
        Some(true),
        Some(true),
        Some(false),
        Some(false),
        Some(true),
        Some(true),
    ];
    const P: Option<bool> = Some(false);

    let parity = radio_datetime_helpers::get_parity(&BITS, 0, 6, P);
    println!(
        "Even parity over {:?} with check {:?} is {}",
        BITS,
        P,
        if parity == Some(false) { "OK" } else { "bad" }
    );

    const P2: Option<bool> = Some(true);
    let parity = radio_datetime_helpers::get_parity(&BITS, 2, 5, P2);
    println!(
        "Odd parity over {:?} with check {:?} is {}",
        &BITS[2..6],
        P2,
        if parity == Some(true) { "OK" } else { "bad" }
    );

    let value_lsb = radio_datetime_helpers::get_bcd_value(&BITS, 0, 6);
    let value_msb = radio_datetime_helpers::get_bcd_value(&BITS, 6, 0);
    println!("BCD value of {:?} left-to-right is {:?}", BITS, value_lsb);
    println!("BCD value of {:?} right-to-left is {:?}", BITS, value_msb);

    const BAD_BITS: [Option<bool>; 4] = [Some(false), None, Some(true), Some(true)];
    let parity = radio_datetime_helpers::get_parity(&BAD_BITS, 0, 3, P);
    println!(
        "Parity over {:?} with check {:?} is {:?}",
        BAD_BITS, P, parity
    );
    let parity = radio_datetime_helpers::get_parity(&BITS, 0, 6, None);
    const Q: Option<bool> = None;
    println!("Parity over {:?} with check {:?} is {:?}", BITS, Q, parity);
    let value_lsb = radio_datetime_helpers::get_bcd_value(&BAD_BITS, 0, 6);
    println!(
        "BCD value of {:?} left-to-right is {:?}",
        BAD_BITS, value_lsb
    );
}
