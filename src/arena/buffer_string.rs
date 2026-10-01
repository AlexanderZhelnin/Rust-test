//! Точный порт C# `apiTest.Arena.BufferString` (Arena/Buffer.String.cs).
//!
//! Строковый буфер на базе арены. В C# `char` — 2 байта (UTF-16), здесь —
//! `char` (4 байта, UTF-32). NUL-терминатор воспроизведён.
//!
//! `BufferString` не хранит ссылку на аллокатор (как в C# — класс-ссылка),
//! поэтому несколько `BufferString` могут одновременно использовать один аллокатор.
//! Аллокатор передаётся в методы, которые могут вызывать рост.

use super::allocator::ArenaAllocator;
use super::memory::ArenaMemory;

/// Строковый буфер на арене (аналог C# `BufferString`)
pub struct BufferString {
    items: ArenaMemory<char>,
    count: usize,
    capacity: usize,
}

impl BufferString {
    /// Аналог C# конструктора `BufferString(allocator, copacity = 32)`
    pub fn new(allocator: &mut ArenaAllocator<char>, capacity: usize) -> Self {
        let items = allocator.alloc(capacity + 1); // +1 под NUL
        Self {
            items,
            count: 0,
            capacity,
        }
    }

    /// Аналог C# `AsSpan()`
    #[inline]
    pub fn as_span(&self) -> &[char] {
        &self.items[..self.count]
    }

    /// Аналог C# `Count`
    #[inline]
    pub fn count(&self) -> usize {
        self.count
    }

    /// Аналог C# `Append(ReadOnlySpan<char>)` — пишет NUL-терминатор
    pub fn append_slice(&mut self, allocator: &mut ArenaAllocator<char>, item: &[char]) {
        let count = self.count + item.len();
        if count > self.capacity {
            self.ensure_capacity(allocator, count);
        }
        self.items[self.count..count].copy_from_slice(item);
        self.count = count;
        self.items[self.count] = '\0';
    }

    /// Аналог C# `Append(char)` — пишет NUL-терминатор
    pub fn append_char(&mut self, allocator: &mut ArenaAllocator<char>, item: char) {
        let count = self.count + 1;
        if count > self.capacity {
            self.ensure_capacity(allocator, count);
        }
        self.items[self.count] = item;
        self.count = count;
        self.items[self.count] = '\0';
    }

    /// Аналог C# `Append<T : ISpanFormattable>` — форматирование числа/строки.
    /// NUL-терминатор здесь НЕ пишется (как в C#).
    pub fn append_formatted(&mut self, allocator: &mut ArenaAllocator<char>, s: &str) {
        let chars: Vec<char> = s.chars().collect();
        let count = self.count + chars.len();
        if count > self.capacity {
            self.ensure_capacity(allocator, count);
        }
        self.items[self.count..count].copy_from_slice(&chars);
        self.count = count;
    }

    /// Аналог C# `EnsureCapacity(int)`
    fn ensure_capacity(&mut self, allocator: &mut ArenaAllocator<char>, capacity: usize) {
        self.capacity = capacity * 2 + 2;
        let old = self.items;
        let old_count = self.count;
        self.items = allocator.alloc(self.capacity + 1); // +1 под NUL
        self.items[..old_count].copy_from_slice(&old[..old_count]);
    }

    /// Аналог C# `ToString()`
    pub fn to_string(&self) -> String {
        self.as_span().iter().collect()
    }
}
