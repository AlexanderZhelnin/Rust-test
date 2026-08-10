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

#[cfg(test)]
mod tests {
    use super::serialize;

    #[test]
    fn writes_printable_unicode_as_utf8() {
        assert_eq!(serialize("Ж😀\u{007f}"), "\"Ж😀\u{007f}\"".as_bytes());
    }

    #[test]
    fn uses_standard_json_escapes() {
        assert_eq!(
            serialize("\0\u{0008}\t\n\u{000c}\r\u{001f}\\"),
            br#""\u0000\b\t\n\f\r\u001f\\""#
        );
    }
}
