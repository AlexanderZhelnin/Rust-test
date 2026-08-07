mod calc;
mod drawer;
mod init;
mod models;
mod polygon;
mod polyline;
mod strings;

// use actix_web::{web, App, HttpResponse, HttpServer};
use axum::{Json, Router, extract::Query, http::StatusCode, routing::get};
use drawer::build;
use init::init_data;
use models::{DrawProperties1, ILayer, Rect};
use std::sync::OnceLock;

#[derive(serde::Deserialize)]
struct MapQuery {
    x: f64,
    y: f64,
}

// Глобальные данные для инициализации (без unsafe)
static LS: OnceLock<Vec<models::Legend>> = OnceLock::new();
static R: OnceLock<Rect> = OnceLock::new();

fn get_data() -> (&'static Vec<models::Legend>, &'static Rect) {
    let ls = LS.get_or_init(|| init_data().0);
    let r = R.get_or_init(|| init_data().1);
    (ls, r)
}

/// Эндпоинт для чтения файла
async fn read_file() -> (StatusCode, String) {
    match std::fs::read_to_string("data.txt") {
        Ok(content) => (StatusCode::OK, content),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("Ошибка: {}", e)),
    }
}

/// Эндпоинт для вычисления числа Фибоначчи (оптимизировано)
async fn fibonacci() -> String {
    let mut a: u64 = 0;
    let mut b: u64 = 1;

    for _ in 2..2_000_000 {
        let temp = b;
        b = a + b;
        a = temp;
    }

    a.to_string()
}

/// Эндпоинт для получения преобразованных геоданных (без реального ответа)
async fn map_query(Query(query): Query<MapQuery>) -> (StatusCode, String) {
    let x = query.x / 100.0;
    let y = query.y / 100.0;

    let (pr, rect) = get_data();

    let pr1 = DrawProperties1 {
        left_top: [rect.left + x, rect.top + y],
        scale: 0.37037037037037035,
        mashtab: 100.0,
    };

    let mut rect1 = Rect {
        left: rect.left + x,
        top: rect.top + y,
        bottom: rect.bottom,
        right: rect.right,
    };

    // Используем ссылки вместо клонирования
    let result = build(pr, &mut pr1.clone(), &mut rect1);

    (StatusCode::OK, result.len().to_string())
}

/// Эндпоинт для получения преобразованных геоданных с JSON ответом
async fn map_json_query(Query(query): Query<MapQuery>) -> (StatusCode, Json<Vec<ILayer>>) {
    let x = query.x / 100.0;
    let y = query.y / 100.0;

    let (pr, rect) = get_data();

    let pr1 = DrawProperties1 {
        left_top: [rect.left + x, rect.top + y],
        scale: 0.37037037037037035,
        mashtab: 100.0,
    };

    let mut rect1 = Rect {
        left: rect.left + x,
        top: rect.top + y,
        bottom: rect.bottom,
        right: rect.right,
    };

    // Используем ссылки вместо клонирования
    let result = build(pr, &mut pr1.clone(), &mut rect1);

    // Возвращаем первые 5 элементов
    let limited_result = if result.len() > 5 {
        &result[..5]
    } else {
        &result[..]
    };

    (StatusCode::OK, Json(limited_result.to_vec()))
}

/// Эндпоинт для натуральной сортировки строк
async fn natural_sort() -> (StatusCode, String) {
    const STR1: &str = "asrgfsadf12421";
    const STR2: &str = "asrgfsadf12321";
    let mut buf = itoa::Buffer::new();

    let str1_len = STR1.len();
    let str2_len = STR2.len();

    let mut result = 0;
    for i in 0..10_000 {
        // let s1 = format!("{}{}", STR1, i);
        // let s2 = format!("{}{}", STR2, i);

        let i_str = buf.format(i);
        let i_len = i_str.len();

        let mut s1 = String::with_capacity(str1_len + i_len);
        s1.push_str(STR1);
        s1.push_str(i_str);

        let mut s2 = String::with_capacity(str2_len + i_len);
        s2.push_str(STR2);
        s2.push_str(i_str);

        result += strings::compare(&s1, &s2);
    }

    (StatusCode::OK, result.to_string())
}

/// Эндпоинт для натуральной сортировки строк (Версия для прикола)
async fn natural_sort_hack() -> (StatusCode, String) {
    const STR1: &str = "asrgfsadf12421";
    const STR2: &str = "asrgfsadf12321";

    let max_len = "10000".len();
    let mut s1 = String::with_capacity(STR1.len() + max_len);
    let mut s2 = String::with_capacity(STR2.len() + max_len);
    let mut buf = itoa::Buffer::new();

    let mut result = 0;
    for i in 0..10_000 {
        let i_str = buf.format(i);

        s1.clear();
        s1.push_str(STR1);
        s1.push_str(i_str);

        s2.clear();
        s2.push_str(STR2);
        s2.push_str(i_str);

        result += strings::compare(&s1, &s2);
    }

    (StatusCode::OK, result.to_string())
}

/// Корневой эндпоинт
async fn root() -> &'static str {
    "Hello World rust!"
}

#[tokio::main]
async fn main() {
    let listen_addr = "127.0.0.1:3003";
    println!("Starting Rust server at {listen_addr}");

    let app = Router::new()
        .route("/", get(root))
        .route("/readfile", get(read_file))
        .route("/fibonacci", get(fibonacci))
        .route("/map", get(map_query))
        .route("/mapJSON", get(map_json_query))
        .route("/naturalsort", get(natural_sort))
        .route("/naturalsorthack", get(natural_sort_hack));

    let listener = tokio::net::TcpListener::bind(listen_addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
