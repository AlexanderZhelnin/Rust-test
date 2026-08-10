use widestring::U16Str;

const ZERO: u16 = b'0' as u16;

/// Натуральное сравнение готовых UTF-16 строк без перекодирования в горячем цикле.
/// `U16Str` хранит те же 16-битные code units, по которым идёт C# `char*`.
/// Границы проверяются условиями циклов, после которых LLVM устраняет повторные
/// panic-checks у индексирования. Создаваемые endpoint-ом строки не содержат `\0`
#[inline]
pub fn compare(s1: &U16Str, s2: &U16Str) -> i32 {
    let s1 = s1.as_slice();
    let s2 = s2.as_slice();
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

            // Как C# в unchecked-контексте: переполнение `int` оборачивается
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

// Как в C# : считаем цифрами только ASCII `0..9`,
// а не все цифровые символы Unicode
#[inline]
fn is_ascii_digit(value: u16) -> bool {
    value >= u16::from(b'0') && value <= u16::from(b'9')
}

#[cfg(test)]
mod tests {
    use super::compare;
    use widestring::{U16Str, u16str};

    #[test]
    fn compares_utf16_code_units_like_csharp() {
        assert_eq!(compare(u16str!("😀"), u16str!("\u{e000}")), -1);
    }

    #[test]
    fn compares_embedded_numbers_naturally() {
        assert_eq!(compare(u16str!("item9"), u16str!("item10")), -1);
        assert_eq!(compare(u16str!("item10"), u16str!("item9")), 1);
        assert_eq!(compare(u16str!("item001"), u16str!("item1")), 0);
    }

    #[test]
    fn compares_empty_strings_and_prefixes() {
        assert_eq!(compare(u16str!(""), u16str!("")), 0);
        assert_eq!(compare(u16str!(""), u16str!("a")), -1);
        assert_eq!(compare(u16str!("a"), u16str!("")), 1);
        assert_eq!(compare(u16str!("a"), u16str!("ab")), -1);
    }

    #[test]
    fn wraps_integer_overflow_like_csharp_unchecked_int() {
        assert_eq!(compare(u16str!("item2147483648"), u16str!("item0")), -1);
    }

    #[test]
    fn supports_unpaired_surrogates_like_csharp_string() {
        let first = [0xd83d];
        let second = [0xe000];

        assert_eq!(
            compare(U16Str::from_slice(&first), U16Str::from_slice(&second)),
            -1
        );
    }

    #[test]
    fn embedded_nul_is_a_code_unit_like_csharp_safe_comparer() {
        assert_eq!(compare(u16str!("a\0z"), u16str!("a\0a")), 1);
    }
}
