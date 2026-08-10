mod calc;
mod drawer;
mod init;
mod json_response;
mod models;
mod polygon;
mod polyline;
mod strings;

use axum::{Json, Router, extract::Query, http::StatusCode, response::Response, routing::get};
use drawer::{build, build_iter};
use init::init_data;
use models::{DrawProperties1, Legend, Rect};
use std::sync::OnceLock;
use widestring::{U16Str, U16String, u16str};

const SORT_PREFIX_1: &U16Str = u16str!("asrgfsadf12421");
const SORT_PREFIX_2: &U16Str = u16str!("asrgfsadf12321");
const SORT_STACK_CAPACITY: usize = 128;

// все выделения памяти Rust выполняются через mimalloc
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[inline]
fn write_sort_value(value: &mut U16String, prefix: &U16Str, suffix: &str) {
    value.clear();
    value.push(prefix);

    // `itoa` возвращает только ASCII. Расширяем каждый байт напрямую до u16,
    // как числовой форматтер C# записывает цифры в UTF-16 `char`, не запуская
    // декодирование UTF-8
    debug_assert!(suffix.is_ascii());
    let units = value.as_mut_vec();
    for &byte in suffix.as_bytes() {
        units.push(u16::from(byte));
    }
}

/// Аналог `ValueStringBuilder(stackalloc char[128])`: буфер целиком на стеке,
/// а ASCII-цифры из `itoa` расширяются до UTF-16 без heap-аллокаций
#[inline]
fn write_sort_stack(
    value: &mut [u16; SORT_STACK_CAPACITY],
    prefix: &U16Str,
    suffix: &str,
) -> usize {
    debug_assert!(suffix.is_ascii());
    let prefix = prefix.as_slice();
    let suffix = suffix.as_bytes();
    let len = prefix.len() + suffix.len();
    debug_assert!(len <= value.len());

    value[..prefix.len()].copy_from_slice(prefix);
    for (target, &source) in value[prefix.len()..len].iter_mut().zip(suffix) {
        *target = u16::from(source);
    }

    len
}

// Как в C# `(double x = 0, double y = 0)`: отсутствующие query-параметры
// независимо получают значение 0.0
#[derive(Default, serde::Deserialize)]
#[serde(default)]
struct MapQuery {
    x: f64,
    y: f64,
}

// Как в C#: файл читается и разбирается ровно один раз,
// а готовые данные живут до завершения процесса
static DATA: OnceLock<(Vec<Legend>, Rect)> = OnceLock::new();

fn get_data() -> (&'static [Legend], &'static Rect) {
    let (legends, rect) = DATA.get_or_init(init_data);
    // Возвращаем ссылки на общие данные без клонирования всей карты
    (legends.as_slice(), rect)
}

/// Эндпоинт для чтения файла
async fn read_file() -> (StatusCode, String) {
    // Как `File.ReadAllTextAsync` в C#: асинхронное чтение не блокирует
    // рабочий поток HTTP-сервера на время файловой операции
    match tokio::fs::read_to_string("data.txt").await {
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
        b = a.wrapping_add(b);
        a = temp;
    }

    a.to_string()
}

/// Эндпоинт для получения преобразованных геоданных (без реального ответа)
/// `Json<i32>` повторяет C#: возвращаемый `int` сериализуется как JSON,
/// а не как строка с типом `text/plain`
async fn map_query(Query(query): Query<MapQuery>) -> Json<i32> {
    let x = query.x / 100.0;
    let y = query.y / 100.0;

    let (legends, rect) = get_data();

    let pr1 = DrawProperties1 {
        left_top: [rect.left + x, rect.top + y],
        scale: 0.37037037037037035,
        mashtab: 100.0,
    };

    let rect1 = Rect {
        left: rect.left + x,
        top: rect.top + y,
        bottom: rect.bottom,
        right: rect.right,
    };

    // Как C# `BuildGenerator(...).Count()`: общий `Vec<ILayer>` не создаётся,
    // а уже посчитанный слой освобождается перед построением следующего
    Json(build_iter(legends, &pr1, &rect1).count() as i32)
}

/// Эндпоинт для получения преобразованных геоданных с JSON ответом
async fn map_json_query(Query(query): Query<MapQuery>) -> Response {
    let x = query.x / 100.0;
    let y = query.y / 100.0;

    let (legends, rect) = get_data();

    let pr1 = DrawProperties1 {
        left_top: [rect.left + x, rect.top + y],
        scale: 0.37037037037037035,
        mashtab: 100.0,
    };

    let rect1 = Rect {
        left: rect.left + x,
        top: rect.top + y,
        bottom: rect.bottom,
        right: rect.right,
    };

    let result = build(legends, &pr1, &rect1);

    // Как и C# Take(5): вычисляем все слои и сохраняем исходный массив живым
    // до конца сериализации, но в JSON передаём только первые пять
    let limited_result = result.get(..5).unwrap_or(&result);
    let body = json_response::serialize(limited_result);

    json_response::from_bytes(body)
}

/// Эндпоинт для натуральной сортировки строк
/// Как в C#, результат `int` возвращается числом JSON
async fn natural_sort() -> Json<i32> {
    let mut buf = itoa::Buffer::new();
    let mut result = 0;

    for i in 0..10_000 {
        // Как `STR1 + i` и `STR2 + i` в C#: на каждой итерации создаются
        // две строки и число форматируется отдельно для каждой из них
        let digits = buf.format(i);
        let mut s1 = U16String::with_capacity(SORT_PREFIX_1.len() + digits.len());
        write_sort_value(&mut s1, SORT_PREFIX_1, digits);

        let digits = buf.format(i);
        let mut s2 = U16String::with_capacity(SORT_PREFIX_2.len() + digits.len());
        write_sort_value(&mut s2, SORT_PREFIX_2, digits);

        result += strings::compare(s1.as_ustr(), s2.as_ustr());
    }

    Json(result)
}

/// Эндпоинт для натуральной сортировки строк (Версия для прикола)
/// Как в C#, результат `int` возвращается числом JSON
async fn natural_sort_hack() -> Json<i32> {
    let mut s1 = [0_u16; SORT_STACK_CAPACITY];
    let mut s2 = [0_u16; SORT_STACK_CAPACITY];
    let mut buf = itoa::Buffer::new();

    let mut result = 0;
    for i in 0..10_000 {
        let digits = buf.format(i);
        let s1_len = write_sort_stack(&mut s1, SORT_PREFIX_1, digits);

        let digits = buf.format(i);
        let s2_len = write_sort_stack(&mut s2, SORT_PREFIX_2, digits);

        result += strings::compare(
            U16Str::from_slice(&s1[..s1_len]),
            U16Str::from_slice(&s2[..s2_len]),
        );
    }

    Json(result)
}

/// Корневой эндпоинт
async fn root() -> &'static str {
    "Hello World rust!"
}

// Инструментированная PGO-сборка должна завершиться штатно, чтобы LLVM
// записал профиль. В обычном запуске эта ветка не выполняется, и сервер
// по-прежнему работает через прямой `serve(...).await`
async fn wait_for_pgo_training_shutdown() {
    let _ = tokio::task::spawn_blocking(|| {
        let mut input = String::new();
        let _ = std::io::stdin().read_line(&mut input);
    })
    .await;
}

#[tokio::main]
async fn main() {
    let listen_addr = "127.0.0.1:3003";
    get_data();
    println!("Starting Rust server at {listen_addr}");

    let app = Router::new()
        .route("/", get(root))
        .route("/readfile", get(read_file))
        .route("/fibonacci", get(fibonacci))
        .route("/map", get(map_query))
        .route("/mapJSON", get(map_json_query))
        .route("/naturalsort", get(natural_sort))
        .route("/naturalsortHack", get(natural_sort_hack));

    let listener = tokio::net::TcpListener::bind(listen_addr).await.unwrap();
    if std::env::var_os("API_TEST_PGO_TRAINING").is_some() {
        tokio::select! {
            result = axum::serve(listener, app) => result.unwrap(),
            _ = wait_for_pgo_training_shutdown() => {}
        }
    } else {
        axum::serve(listener, app).await.unwrap();
    }
}
