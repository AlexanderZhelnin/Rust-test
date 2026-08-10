use axum::{
    body::{Body, Bytes},
    http::{StatusCode, header},
    response::Response,
};
use serde::Serialize;
use std::{mem, sync::Mutex};

// Тело `/mapJSON` при x=0,y=0 занимает около 393 КБ. Запас исключает рост
// Vec во время сериализации, а пул возвращает уже выделенную память после
// фактической отправки ответа клиенту
const INITIAL_CAPACITY: usize = 512 * 1024;
const MAX_POOLED_BODIES: usize = 32;
const MAX_POOLED_CAPACITY: usize = INITIAL_CAPACITY * 2;

static BODY_POOL: Mutex<Vec<Vec<u8>>> = Mutex::new(Vec::new());

fn take_buffer() -> Vec<u8> {
    let mut body = BODY_POOL
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .pop()
        .unwrap_or_else(|| Vec::with_capacity(INITIAL_CAPACITY));
    body.clear();
    body
}

// `Bytes::from_owner` не копирует Vec. Когда Hyper освобождает последние
// байты ответа, Drop возвращает исходный буфер в пул
struct PooledBody(Option<Vec<u8>>);

impl AsRef<[u8]> for PooledBody {
    fn as_ref(&self) -> &[u8] {
        self.0.as_deref().unwrap_or_default()
    }
}

impl Drop for PooledBody {
    fn drop(&mut self) {
        let Some(mut body) = self.0.take() else {
            return;
        };

        body.clear();
        if body.capacity() > MAX_POOLED_CAPACITY {
            return;
        }

        let mut pool = BODY_POOL
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if pool.len() < MAX_POOLED_BODIES {
            pool.push(mem::take(&mut body));
        }
    }
}

pub fn serialize<T>(value: &T) -> Vec<u8>
where
    T: ?Sized + Serialize,
{
    let mut body = take_buffer();
    serde_json::to_writer(&mut body, value).expect("Не удалось сериализовать JSON");
    body
}

pub fn from_bytes(body: Vec<u8>) -> Response {
    let body = Bytes::from_owner(PooledBody(Some(body)));

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
        let value = " !\"#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\]^_`abcdefghijklmnopqrstuvwxyz{|}~";
        let expected = r##"" !\"#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\]^_`abcdefghijklmnopqrstuvwxyz{|}~""##;
        assert_eq!(serialize(value), expected.as_bytes());

        assert_eq!(
            serialize("\0\u{0008}\t\n\u{000c}\r\u{001f}\\"),
            br#""\u0000\b\t\n\f\r\u001f\\""#
        );
    }
}
