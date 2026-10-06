mod arena;
mod calc;
mod drawer;
mod init;
mod json_response;
mod models;
mod polygon;
mod polyline;
mod strings;

use axum::{Json, Router, extract::Query, response::Response, routing::get};
use drawer::{build, build_blazing, build_iter, build_blazing_ptr};
use init::init_data;
use models::{DrawProperties1, LayerResultBlazing, Legend, ObrazResultBlazing, Rect, ObrazResultBlazingPtr, LayerResultBlazingPtr};
// use tokio::time::sleep;
use std::sync::OnceLock;
use widestring::{U16Str, U16String, u16str};
// use utoipa::{OpenApi, ToSchema, path};

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

/// Эндпоинт для вычисления числа Фибоначчи (оптимизировано)
// #[utoipa::path(
//     get,
//     path = "/fibonacci",
//     responses((status = 200, description = "Фибоначчи", body = String))
// )]
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

/// Получение преобразованных геоданных (без реального ответа)
fn map_work(query: MapQuery) -> i32 {
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
    build_iter(legends, &pr1, &rect1).count() as i32
}

/// Эндпоинт для получения преобразованных геоданных (без реального ответа)
// #[utoipa::path(
//     get,
//     path = "/map",
//     responses((status = 200, description = "Получение преобразованных геоданных (тест без реального ответа)", body = JsonLayers))
// )]
async fn map_query(Query(query): Query<MapQuery>) -> Json<i32> {
    let result = run_on_cpu_pool(move || map_work(query)).await;

    Json(result)
}

/// Получение преобразованных геоданных (без реального ответа) Blazing Версия
fn map_blazing_work(query: MapQuery) -> i32 {
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

    let mut allocator_f64 = arena::ArenaAllocator::<f64>::get();
    let mut allocator_obrazes = arena::ArenaAllocator::<ObrazResultBlazing>::get();
    let mut allocator_layers = arena::ArenaAllocator::<LayerResultBlazing>::get();

    build_blazing(
        legends,
        &mut allocator_f64,
        &mut allocator_obrazes,
        &mut allocator_layers,
        &pr1,
        &rect1,
    ).len() as i32
}

/// Эндпоинт для получения преобразованных геоданных (без реального ответа)
async fn map_blazing_query(Query(query): Query<MapQuery>) -> Json<i32> {
    let result = run_on_cpu_pool(move || map_blazing_work(query)).await;

    Json(result)
}

/// Получение преобразованных геоданных (без реального ответа) Blazing Версия
fn map_blazing_ptr_work(query: MapQuery) -> i32 {
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

    let mut allocator_f64 = arena::ArenaAllocator::<f64>::get();
    let mut allocator_obrazes = arena::ArenaAllocator::<ObrazResultBlazingPtr>::get();
    let mut allocator_layers = arena::ArenaAllocator::<LayerResultBlazingPtr>::get();

    build_blazing_ptr(
        legends,
        &mut allocator_f64,
        &mut allocator_obrazes,
        &mut allocator_layers,
        &pr1,
        &rect1,
    ).len() as i32
}

/// Эндпоинт для получения преобразованных геоданных (без реального ответа)
async fn map_blazing_ptr_query(Query(query): Query<MapQuery>) -> Json<i32> {
    let result = run_on_cpu_pool(move || map_blazing_ptr_work(query)).await;

    Json(result)
}

/// Получение преобразованных геоданных с JSON ответом
fn map_json_work(query: MapQuery) -> Vec<u8> {
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
    json_response::serialize(limited_result)
}

/// Эндпоинт для получения преобразованных геоданных с JSON ответом
async fn map_json_query(Query(query): Query<MapQuery>) -> Response {
    let body = run_on_cpu_pool(move || map_json_work(query)).await;

    json_response::from_bytes(body)
}

/// Получение преобразованных геоданных с JSON ответом Blazing версия
fn map_json_blazing_work(query: MapQuery) -> Vec<u8> {
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

    let mut allocator_f64 = arena::ArenaAllocator::<f64>::get();
    let mut allocator_obrazes = arena::ArenaAllocator::<ObrazResultBlazing>::get();
    let mut allocator_layers = arena::ArenaAllocator::<LayerResultBlazing>::get();

    let result = build_blazing(
        legends,
        &mut allocator_f64,
        &mut allocator_obrazes,
        &mut allocator_layers,
        &pr1,
        &rect1,
    );

    // Как и C# Take(5): вычисляем все слои и сохраняем исходный массив живым
    // до конца сериализации, но в JSON передаём только первые пять
    let limited_result = result.sub(0..5.min(result.len()));

    json_response::serialize(limited_result.as_slice())
}

/// Эндпоинт для получения преобразованных геоданных с JSON ответом Blazing версия
async fn map_json_blazing_query(Query(query): Query<MapQuery>) -> Response {
    let body = run_on_cpu_pool(move || map_json_blazing_work(query)).await;

    json_response::from_bytes(body)
}


/// Получение преобразованных геоданных с JSON ответом Blazing версия
fn map_json_blazing_ptr_work(query: MapQuery) -> Vec<u8> {
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

    let mut allocator_f64 = arena::ArenaAllocator::<f64>::get();
    let mut allocator_obrazes = arena::ArenaAllocator::<ObrazResultBlazingPtr>::get();
    let mut allocator_layers = arena::ArenaAllocator::<LayerResultBlazingPtr>::get();

    let result = build_blazing_ptr(
        legends,
        &mut allocator_f64,
        &mut allocator_obrazes,
        &mut allocator_layers,
        &pr1,
        &rect1,
    );

    // Как и C# Take(5): вычисляем все слои и сохраняем исходный массив живым
    // до конца сериализации, но в JSON передаём только первые пять
    let limited_result = result.sub(0..5.min(result.len()));

    json_response::serialize(limited_result.as_slice())
}

/// Эндпоинт для получения преобразованных геоданных с JSON ответом Blazing версия
async fn map_json_blazing_ptr_query(Query(query): Query<MapQuery>) -> Response {
    let body = run_on_cpu_pool(move || map_json_blazing_ptr_work(query)).await;

    json_response::from_bytes(body)
}

/// Тестовые вычисления натурального сравнения
/// Как в C#, результат `int` возвращается числом JSON
fn natural_sort_work() -> i32 {
    let mut buf = itoa::Buffer::new();
    let mut result = 0;

    for i in 0..10_000 {
        let digits = buf.format(i);
        let mut s1 = U16String::with_capacity(SORT_PREFIX_1.len() + digits.len());
        write_sort_value(&mut s1, SORT_PREFIX_1, digits);

        let digits = buf.format(i);
        let mut s2 = U16String::with_capacity(SORT_PREFIX_2.len() + digits.len());
        write_sort_value(&mut s2, SORT_PREFIX_2, digits);

        result += strings::compare(s1.as_slice(), s2.as_slice());
    }

    result
}

/// Эндпоинт для натуральной сортировки строк
// #[utoipa::path(
//     get,
//     path = "/naturalsort",
//     responses((status = 200, description = "Натуральное сравнение 10000 пар строк", body = String))
// )]
async fn natural_sort() -> Json<i32> {
    let result = run_on_cpu_pool(natural_sort_work).await;

    Json(result)
}

fn natural_sort_blazing_work() -> i32 {
    let mut buf = itoa::Buffer::new();
    let mut result = 0;

    let (l1, l2) = (SORT_PREFIX_1.len() + 5, SORT_PREFIX_2.len() + 5);

    let mut allocator = arena::ArenaAllocator::<u16>::get();

    for i in 0..10_000 {
        let digits = buf.format(i);

        let mut s1 = allocator.alloc(l1);
        let mut index1 = SORT_PREFIX_1.len();
        let s1_slice = s1.as_mut_slice();

        s1_slice[..SORT_PREFIX_1.len()].copy_from_slice(SORT_PREFIX_1.as_slice());

        for &byte in digits.as_bytes() {
            s1_slice[index1] = u16::from(byte);
            index1 += 1;
        }

        let digits = buf.format(i);
        let mut s2 = allocator.alloc(l2);
        let mut index2 = SORT_PREFIX_2.len();
        let s2_slice = s2.as_mut_slice();

        s2_slice[..SORT_PREFIX_2.len()].copy_from_slice(SORT_PREFIX_2.as_slice());

        for &byte in digits.as_bytes() {
            s2_slice[index2] = u16::from(byte);
            index2 += 1;
        }

        result += strings::compare(s1.sub(0..index1).as_slice(), s2.sub(0..index2).as_slice());
    }

    result
}

async fn natural_sort_blazing_query() -> Json<i32> {
    let result = run_on_cpu_pool(natural_sort_blazing_work).await;

    Json(result)
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

        result += strings::compare(&s1[..s1_len], &s2[..s2_len]);
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
    let cpu_threads = cpu_parallelism();
    init_cpu_executor(cpu_threads);
    get_data();
    println!("Rayon CPU pool: {cpu_threads} threads");
    println!("Starting Rust server at {listen_addr}");

    let app = Router::new()
        .route("/", get(root))
        .route("/fibonacci", get(fibonacci))
        .route("/map", get(map_query))
        .route("/mapBlazing", get(map_blazing_query))
        .route("/mapBlazingPtr", get(map_blazing_ptr_query))
        .route("/mapJSON", get(map_json_query))
        .route("/mapJSONBlazing", get(map_json_blazing_query))
        .route("/mapJSONBlazingPtr", get(map_json_blazing_ptr_query))
        .route("/naturalsort", get(natural_sort))
        .route("/naturalsortBlazing", get(natural_sort_blazing_query))
        .route("/naturalsortHack", get(natural_sort_hack));

    let listener = tokio::net::TcpListener::bind(listen_addr).await.unwrap();
    // if std::env::var_os("API_TEST_PGO_TRAINING").is_some() {
    //     tokio::select! {
    //         result = axum::serve(listener, app) => result.unwrap(),
    //         _ = wait_for_pgo_training_shutdown() => {}
    //     }
    // } else {
    axum::serve(listener, app).await.unwrap();
    // }
}
