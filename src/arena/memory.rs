use std::ops::{Deref, DerefMut, Range};

/// Вид на область памяти арены (аналог C# `Memory<T>`)
#[derive(Clone, Copy)]
pub struct ArenaMemory<T> {
    ptr: *mut T,
    len: usize,
}

// Память принадлежит ArenaAllocator<T>, который живёт в пуле
// и никогда не освобождает свои буферы. Точки не разделяются небезопасно
// между потоками (арена используется в рамках одного запроса).
unsafe impl<T: Send> Send for ArenaMemory<T> {}
unsafe impl<T: Send> Sync for ArenaMemory<T> {}

impl<T> ArenaMemory<T> {
    pub const unsafe fn from_raw(ptr: *mut T, len: usize) -> Self {
        Self { ptr, len }
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.len
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    #[inline]
    pub fn as_slice(&self) -> &[T] {
        unsafe { std::slice::from_raw_parts(self.ptr, self.len) }
    }

    #[inline]
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        unsafe { std::slice::from_raw_parts_mut(self.ptr, self.len) }
    }

    #[inline]
    pub fn sub(&self, range: Range<usize>) -> ArenaMemory<T> {
        let start = range.start.min(self.len);
        let end = range.end.min(self.len);
        debug_assert!(start <= end, "invalid range for ArenaMemory::sub");
        unsafe { ArenaMemory::from_raw(self.ptr.add(start), end - start) }
    }

    #[inline]
    pub fn copy_to(&self, dest: &mut ArenaMemory<T>)
    where
        T: Copy,
    {
        dest.as_mut_slice().copy_from_slice(self.as_slice());
    }
}

impl<T> Deref for ArenaMemory<T> {
    type Target = [T];
    #[inline]
    fn deref(&self) -> &[T] {
        self.as_slice()
    }
}

impl<T> DerefMut for ArenaMemory<T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut [T] {
        self.as_mut_slice()
    }
}

impl<T> std::fmt::Debug for ArenaMemory<T>
where
    [T]: std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ArenaMemory")
            .field("len", &self.len)
            .finish()
    }
}
