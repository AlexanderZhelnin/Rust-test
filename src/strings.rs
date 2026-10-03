use widestring::U16Str;

const ZERO: u16 = b'0' as u16;

/// Натуральное сравнение строк
#[inline]
pub fn compare(s1: &[u16], s2: &[u16]) -> i32 {
    let mut p1 = 0;
    let mut p2 = 0;

    while p1 < s1.len() {
        if p2 >= s2.len() {
            return 1;
        }

        let char1 = s1[p1];
        let char2 = s2[p2];
        p1 += 1;
        p2 += 1;

        if is_ascii_digit(char1) && is_ascii_digit(char2) {
            let mut num1 = i32::from(char1 - ZERO);
            let mut num2 = i32::from(char2 - ZERO);

            while p1 < s1.len() {
                let digit = s1[p1];
                if !is_ascii_digit(digit) {
                    break;
                }
                num1 = num1.wrapping_mul(10).wrapping_add(i32::from(digit - ZERO));
                p1 += 1;
            }

            while p2 < s2.len() {
                let digit = s2[p2];
                if !is_ascii_digit(digit) {
                    break;
                }
                num2 = num2.wrapping_mul(10).wrapping_add(i32::from(digit - ZERO));
                p2 += 1;
            }

            if num1 != num2 {
                return if num1 > num2 { 1 } else { -1 };
            }
        } else if char1 != char2 {
            return if char1 > char2 { 1 } else { -1 };
        }
    }

    if p2 == s2.len() { 0 } else { -1 }
}

#[inline]
fn is_ascii_digit(value: u16) -> bool {
    value >= u16::from(b'0') && value <= u16::from(b'9')
}
