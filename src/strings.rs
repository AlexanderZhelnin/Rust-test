/// Натуральное сравнение строк
pub fn compare(s1: &str, s2: &str) -> i32 {
    // Быстрые проверки
    if s1.is_empty() && s2.is_empty() {
        return 0;
    }
    if s1.is_empty() {
        return -1;
    }
    if s2.is_empty() {
        return 1;
    }

    let mut p1 = s1.chars();
    let mut p2 = s2.chars();

    let mut char1 = p1.next();
    let mut char2 = p2.next();

    loop {
        match (char1, char2) {
            (Some(c1), Some(c2)) => {
                // Проверяем, являются ли оба символа цифрами
                if c1.is_ascii_digit() && c2.is_ascii_digit() {
                    let mut num1 = (c1 as u8 - b'0') as i32;
                    let mut num2 = (c2 as u8 - b'0') as i32;

                    char1 = p1.next();
                    char2 = p2.next();

                    // Читаем остальные цифры первого числа
                    while let Some(c11) = char1 {
                        if c11.is_ascii_digit() {
                            num1 = num1 * 10 + (c11 as u8 - b'0') as i32;
                            char1 = p1.next();
                        } else {
                            break;
                        }
                    }

                    // Читаем остальные цифры второго числа
                    while let Some(c22) = char2 {
                        if c22.is_ascii_digit() {
                            num2 = num2 * 10 + (c22 as u8 - b'0') as i32;
                            char2 = p2.next();
                        } else {
                            break;
                        }
                    }

                    if num1 != num2 {
                        return if num1 > num2 { 1 } else { -1 };
                    }
                } else {
                    // Сравниваем как символы
                    if c1 != c2 {
                        return if c1 > c2 { 1 } else { -1 };
                    }

                    char1 = p1.next();
                    char2 = p2.next();
                }
            }
            (None, None) => return 0,
            (Some(_), None) => return 1,
            (None, Some(_)) => return -1,
        }
    }

    // Проверяем, закончились ли обе строки
    // if p2.next().is_none() { 0 } else { -1 }
}
