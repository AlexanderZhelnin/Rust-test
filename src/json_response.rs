use axum::{
    body::Body,
    http::{StatusCode, header},
    response::Response,
};
use serde::Serialize;

const INITIAL_CAPACITY: usize = 512 * 1024;

/// Стандартный JSON-форматтер без отдельного
/// пула и без пользовательской логики экранирования
pub fn serialize<T>(value: &T) -> Vec<u8>
where
    T: ?Sized + Serialize,
{
    let mut body = Vec::with_capacity(INITIAL_CAPACITY);
    serde_json::to_writer(&mut body, value).expect("Не удалось сериализовать JSON");
    body
}

pub fn from_bytes(body: Vec<u8>) -> Response {
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/json; charset=utf-8")
        .body(Body::from(body))
        .expect("Не удалось создать HTTP-ответ")
}
