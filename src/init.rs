use crate::models::{Legend, Rect};
use std::fs;

/// Инициализация данных из JSON файла
pub fn init_data() -> (Vec<Legend>, Rect) {
    // Читаем данные из JSON файла
    let json = fs::read_to_string("data.json")
        .expect("Не удалось прочитать файл data.json");

    let ls: Vec<Legend> = serde_json::from_str(&json)
        .expect("Не удалось распарсить JSON");

    // Прямоугольник по умолчанию
    let r = Rect {
        left: 1200.0,
        bottom: 50.0,
        right: 4000.0,
        top: 2850.0,
    };

    (ls, r)
}
