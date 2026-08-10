mod calc;
mod drawer;
mod init;
mod json_response;
mod models;
mod polygon;
mod polyline;
mod strings;

use axum::{Json, Router, extract::Query, http::StatusCode, response::Response, routing::get};
use bumpalo::Bump;
use drawer::{build, build_count_reusing};
use init::init_data;
use models::{DrawProperties1, Legend, Rect};
use std::{cell::RefCell, sync::OnceLock};
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
// независимо получают значение 0.0.
#[derive(Default, serde::Deserialize)]
#[serde(default)]
struct MapQuery {
    x: f64,
    y: f64,
}

// Как в C#: файл читается и разбирается ровно один раз,
// а готовые данные живут до завершения процесса
static DATA: OnceLock<(Vec<Legend>, Rect)> = OnceLock::new();

thread_local! {
    // После прогрева каждый поток повторно использует один и тот же буфер
    static BUMP: RefCell<Bump> = RefCell::new(Bump::with_capacity(2_000_000));
}

fn get_data() -> (&'static [Legend], &'static Rect) {
    let (legends, rect) = DATA.get_or_init(init_data);
    // Возвращаем ссылки на общие данные без клонирования всей карты
    (legends.as_slice(), rect)
}

fn draw_input(x: f64, y: f64) -> (&'static [Legend], DrawProperties1, Rect) {
    let x = x / 100.0;
    let y = y / 100.0;
    let (legends, rect) = get_data();

    let properties = DrawProperties1 {
        left_top: [rect.left + x, rect.top + y],
        scale: 0.37037037037037035,
        mashtab: 100.0,
    };

    let clip_rect = Rect {
        left: rect.left + x,
        top: rect.top + y,
        bottom: rect.bottom,
        right: rect.right,
    };

    (legends, properties, clip_rect)
}

/// Эндпоинт для чтения файла.
async fn read_file() -> (StatusCode, String) {
    // Как `File.ReadAllTextAsync` в C#: асинхронное чтение не блокирует
    // рабочий поток HTTP-сервера на время файловой операции
    match tokio::fs::read_to_string("data.txt").await {
        Ok(content) => (StatusCode::OK, content),
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Ошибка: {error}"),
        ),
    }
}

/// Эндпоинт для вычисления числа Фибоначчи
async fn fibonacci() -> String {
    let mut a: u64 = 0;
    let mut b: u64 = 1;

    for _ in 2..2_000_000 {
        let next = a.wrapping_add(b);
        a = b;
        b = next;
    }

    a.to_string()
}

/// Эндпоинт для построения всех слоёв без передачи геометрии клиенту.
/// `Json<i32>` повторяет C#: возвращаемый `int` сериализуется как JSON,
/// а не как строка с типом `text/plain`
fn map_work(query: MapQuery) -> i32 {
    let (legends, properties, rect) = draw_input(query.x, query.y);

    BUMP.with(|cell| {
        let mut arena = cell.borrow_mut();
        let count = build_count_reusing(&mut arena, legends, &properties, &rect);

        count as i32
    })
}

async fn map_query(Query(query): Query<MapQuery>) -> Json<i32> {
    Json(run_on_cpu_pool(move || map_work(query)).await)
}

/// Эндпоинт для JSON-ответа. Все ссылки на арену уничтожаются до `reset()`
fn map_json_work(query: MapQuery) -> Vec<u8> {
    let (legends, properties, rect) = draw_input(query.x, query.y);

    BUMP.with(|cell| {
        let mut arena = cell.borrow_mut();
        let body = {
            let bump: &Bump = &arena;
            let layers = build(bump, legends, &properties, &rect);
            // Как и C# Take(5): вычисляем все слои, но передаём в сериализацию только первые пять
            let limited_layers = layers.get(..5).unwrap_or(layers);
            json_response::serialize(limited_layers)
        };
        arena.reset();

        body
    })
}

async fn map_json_query(Query(query): Query<MapQuery>) -> Response {
    let body = run_on_cpu_pool(move || map_json_work(query)).await;
    json_response::from_bytes(body)
}

fn natural_sort_result() -> i32 {
    let mut number = itoa::Buffer::new();
    let mut result = 0;

    for i in 0..10_000 {
        // Как `STR1 + i` и `STR2 + i` в C#: на каждой итерации создаются
        // две строки и число форматируется отдельно для каждой из них
        let digits = number.format(i);
        let mut first = U16String::with_capacity(SORT_PREFIX_1.len() + digits.len());
        write_sort_value(&mut first, SORT_PREFIX_1, digits);

        let digits = number.format(i);
        let mut second = U16String::with_capacity(SORT_PREFIX_2.len() + digits.len());
        write_sort_value(&mut second, SORT_PREFIX_2, digits);

        result += strings::compare(first.as_ustr(), second.as_ustr());
    }

    result
}

fn natural_sort_hack_result() -> i32 {
    let mut first = [0_u16; SORT_STACK_CAPACITY];
    let mut second = [0_u16; SORT_STACK_CAPACITY];
    let mut number = itoa::Buffer::new();

    let mut result = 0;
    for i in 0..10_000 {
        let digits = number.format(i);
        let first_len = write_sort_stack(&mut first, SORT_PREFIX_1, digits);

        let digits = number.format(i);
        let second_len = write_sort_stack(&mut second, SORT_PREFIX_2, digits);

        result += strings::compare(
            U16Str::from_slice(&first[..first_len]),
            U16Str::from_slice(&second[..second_len]),
        );
    }

    result
}

fn cpu_parallelism() -> usize {
    // Это уже выбранное самим Tokio количество worker'ов. Без явной настройки
    // Tokio получает его через std::thread::available_parallelism()
    tokio::runtime::Handle::current().metrics().num_workers()
}

struct CpuExecutor {
    permits: tokio::sync::Semaphore,
    pool: rayon::ThreadPool,
}

// В сравнения с C# это допустимая настройка runtime, .NET Core также
// выполняет синхронную CPU-работу обработчиков в пуле потоков.
// Rayon меняет только способ планирования, но не логику и объём вычислений.
// Сам Tokio также рекомендует выносить длительную CPU-работу из
// потоков async I/O в отдельный ограниченный executor
static CPU_EXECUTOR: OnceLock<CpuExecutor> = OnceLock::new();

fn init_cpu_executor(thread_count: usize) {
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(thread_count)
        .thread_name(|index| format!("api-cpu-{index}"))
        .build()
        .expect("Failed to create the Rayon thread pool");

    if CPU_EXECUTOR
        .set(CpuExecutor {
            permits: tokio::sync::Semaphore::new(thread_count),
            pool,
        })
        .is_err()
    {
        panic!("CPU executor has already been initialized");
    }
}

fn cpu_executor() -> &'static CpuExecutor {
    CPU_EXECUTOR
        .get()
        .expect("CPU executor has not been initialized")
}

async fn run_on_cpu_pool<T>(work: impl FnOnce() -> T + Send + 'static) -> T
where
    T: Send + 'static,
{
    let executor = cpu_executor();
    let _permit = executor
        .permits
        .acquire()
        .await
        .expect("CPU executor semaphore was closed");
    let (sender, receiver) = tokio::sync::oneshot::channel();
    executor.pool.spawn_fifo(move || {
        let _ = sender.send(work());
    });
    receiver.await.expect("Rayon task terminated unexpectedly")
}

/// Как в C#, результат `int` возвращается числом JSON
async fn natural_sort() -> Json<i32> {
    Json(run_on_cpu_pool(natural_sort_result).await)
}

/// Как в C#, результат `int` возвращается числом JSON
async fn natural_sort_hack() -> Json<i32> {
    Json(natural_sort_hack_result())
}

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
    let cpu_threads = cpu_parallelism();
    init_cpu_executor(cpu_threads);
    get_data();
    println!("Rayon CPU pool: {cpu_threads} threads");
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
