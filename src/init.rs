use crate::models::{Legend, Rect};
use std::fs;

/// Инициализация данных из JSON файла
pub fn init_data() -> (Vec<Legend>, Rect) {
    // Читаем данные из JSON файла
    let json = fs::read_to_string("data.json").expect("Не удалось прочитать файл data.json");

    let ls: Vec<Legend> = serde_json::from_str(&json).expect("Не удалось распарсить JSON");

    // Прямоугольник по умолчанию
    let r = Rect {
        left: 1200.0,
        bottom: 50.0,
        right: 4000.0,
        top: 2850.0,
    };

    (ls, r)
}

#[cfg(test)]
mod tests {
    #[test]
    fn serde_json_parses_double_like_csharp() {
        let value: f64 = serde_json::from_str("853.89239788765656").unwrap();

        assert_eq!(value.to_bits(), 0x408a_af23_a180_f409);
    }
}
