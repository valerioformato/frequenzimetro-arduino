use fixed::{types::extra::U8, FixedU64};
use heapless::String;

fn digit_to_char(digit: usize) -> Option<char> {
    const DIGITS: [char; 10] = ['0', '1', '2', '3', '4', '5', '6', '7', '8', '9'];

    match digit {
        0..=9 => Some(DIGITS[digit]),
        _ => None,
    }
}

pub fn format_freq(freq: FixedU64<U8>) -> String<7> {
    let powers_of_ten: [FixedU64<U8>; 4] = [
        FixedU64::<U8>::from(1_u32),
        FixedU64::<U8>::from(10_u32),
        FixedU64::<U8>::from(100_u32),
        FixedU64::<U8>::from(1000_u32),
    ];

    let mut result = String::<7>::new();
    let mut leading_zero = true;

    for idx in (0..=3).rev() {
        let digit_idx = ((freq / powers_of_ten[idx]) % 10).floor().to_num();

        if leading_zero && digit_idx == 0 {
            continue;
        } else {
            leading_zero = false;
        }

        if let Some(c) = digit_to_char(digit_idx) {
            result.push(c).unwrap();
        }
    }

    result.push('.').unwrap();

    for idx in 1..=3 {
        let digit_idx = ((freq * powers_of_ten[idx]) % 10).floor().to_num();
        if let Some(c) = digit_to_char(digit_idx) {
            result.push(c).unwrap();
        }
    }

    result
}
