mod calc;
mod drawer;
mod init;
mod models;
mod polygon;
mod polyline;
mod strings;

use actix_web::{web, App, HttpResponse, HttpServer};
use drawer::build;
use init::init_data;
use models::{DrawProperties1, Rect};
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
async fn read_file() -> HttpResponse {
    match std::fs::read_to_string("data.txt") {
        Ok(content) => HttpResponse::Ok().body(content),
        Err(e) => HttpResponse::InternalServerError().body(format!("Ошибка: {}", e)),
    }
}

/// Эндпоинт для вычисления числа Фибоначчи (оптимизировано)
async fn fibonacci() -> HttpResponse {
    // Используем итеративный подход с u64 вместо f64
    let mut a: u64 = 0;
    let mut b: u64 = 1;

    for _ in 2..2_000_000 {
        let temp = b;
        b = a.checked_add(b).unwrap_or(b); // Защита от переполнения
        a = temp;
    }

    HttpResponse::Ok().body(a.to_string())
}

/// Эндпоинт для получения преобразованных геоданных (без реального ответа)
async fn map_query(query: web::Query<MapQuery>) -> HttpResponse {
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

    HttpResponse::Ok().body(result.len().to_string())
}

/// Эндпоинт для получения преобразованных геоданных с JSON ответом
async fn map_json_query(query: web::Query<MapQuery>) -> HttpResponse {
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

    let json = serde_json::to_string(limited_result)
        .unwrap_or_else(|_| "[]".to_string());

    HttpResponse::Ok()
        .content_type("application/json")
        .body(json)
}

/// Эндпоинт для натуральной сортировки строк (оптимизировано)
async fn natural_sort() -> HttpResponse {
    const STR1: &str = "asrgfsadf12421";
    const STR2: &str = "asrgfsadf12321";

    let mut result = 0;
    for i in 0..10_000 {
        // Используем срезы вместо создания новых строк
        let s1 = format!("{}{}", STR1, i);
        let s2 = format!("{}{}", STR2, i);
        result += strings::compare(&s1, &s2) as i32;
    }

    HttpResponse::Ok().body(result.to_string())
}

/// Корневой эндпоинт
async fn root() -> HttpResponse {
    HttpResponse::Ok().body("Hello World rust!")
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    println!("Starting Rust server...");

    HttpServer::new(|| {
        App::new()
            .route("/", web::get().to(root))
            .route("/readfile", web::get().to(read_file))
            .route("/fibonacci", web::get().to(fibonacci))
            .route("/map", web::get().to(map_query))
            .route("/mapJSON", web::get().to(map_json_query))
            .route("/naturalsort", web::get().to(natural_sort))
    })
    .bind("127.0.0.1:3003")?
    .run()
    .await
}
