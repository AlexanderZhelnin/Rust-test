use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::mem::MaybeUninit;
use std::ops::{Deref, DerefMut};
use std::sync::{Mutex, OnceLock};

use super::memory::ArenaMemory;

/// Начальный размер буфера
const INITIAL_CAPACITY: usize = 256;

/// Аллокатор арены
pub struct ArenaAllocator<T> {
    /// Текущий буфер
    buffer: Box<[MaybeUninit<T>]>,
    /// Старые буферы, удерживаемые для живых видов `ArenaMemory<T>`
    old_buffers: Vec<Box<[MaybeUninit<T>]>>,
    /// Сколько элементов выделено в текущем буфере
    count: usize,
}

/// При выходе из области видимости аллокатор возвращается в пул.
pub struct ArenaHandle<T: Send + 'static>(std::mem::ManuallyDrop<ArenaAllocator<T>>);

impl<T: Send + 'static> Deref for ArenaHandle<T> {
    type Target = ArenaAllocator<T>;
    #[inline]
    fn deref(&self) -> &ArenaAllocator<T> {
        &self.0
    }
}

impl<T: Send + 'static> DerefMut for ArenaHandle<T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut ArenaAllocator<T> {
        &mut self.0
    }
}

/// Пул Аллокаторов
fn pools() -> &'static Mutex<HashMap<TypeId, Box<dyn Any + Send>>> {
    static POOLS: OnceLock<Mutex<HashMap<TypeId, Box<dyn Any + Send>>>> = OnceLock::new();
    POOLS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn pop_from_pool<T: Send + 'static>() -> Option<ArenaAllocator<T>> {
    let mut guard = pools().lock().unwrap();
    let id = TypeId::of::<ArenaAllocator<T>>();
    let v = guard.get_mut(&id)?.downcast_mut::<Vec<Box<ArenaAllocator<T>>>>()?;
    v.pop().map(|b| *b)
}

fn push_to_pool<T: Send + 'static>(alloc: Box<ArenaAllocator<T>>) {
    let mut guard = pools().lock().unwrap();
    let id = TypeId::of::<ArenaAllocator<T>>();
    let v = guard.entry(id).or_insert_with(|| Box::new(Vec::<Box<ArenaAllocator<T>>>::new()));
    v.downcast_mut::<Vec<Box<ArenaAllocator<T>>>>()
        .expect("pool type mismatch")
        .push(alloc);
}

/// Имплементация методов аллокатора
impl<T> ArenaAllocator<T> {
    fn new() -> Self {
        let mut v = Vec::with_capacity(INITIAL_CAPACITY);
        v.resize_with(INITIAL_CAPACITY, MaybeUninit::uninit);
        Self {
            buffer: v.into_boxed_slice(),
            old_buffers: Vec::new(),
            count: 0,
        }
    }

    /// Возвращает вид и стартовое смещение в буфере.
    #[inline]
    pub fn alloc_with_start(&mut self, length: usize) -> (ArenaMemory<T>, usize) {
        let new_count = self.count + length;
        if new_count > self.buffer.len() {
            let new_cap = grow_cap(self.buffer.len(), new_count);
            let mut v = Vec::with_capacity(new_cap);
            v.resize_with(new_cap, MaybeUninit::uninit);
            // Старый буфер сохраняем: на него могут ссылаться живые виды
            self.old_buffers
                .push(std::mem::replace(&mut self.buffer, v.into_boxed_slice()));
        }
        let start = self.count;
        self.count = new_count;
        let mem = unsafe {
            ArenaMemory::from_raw(self.buffer.as_mut_ptr().cast::<T>().add(start), length)
        };
        (mem, start)
    }

    #[inline]
    pub fn alloc(&mut self, length: usize) -> ArenaMemory<T> {
        self.alloc_with_start(length).0
    }

    // #[inline]
    // pub fn alloc_list(&mut self, capacity: usize) -> super::collection::ArenaList<'_, T>
    // where
    //     T: Copy,
    // {
    //     let items = self.alloc(capacity);
    //     super::collection::ArenaList::new(items, self)
    // }
}

#[inline]
fn grow_cap(old_cap: usize, need: usize) -> usize {
    const MIN_GROW: usize = 256;
    let mut new_cap = old_cap.max(MIN_GROW);
    while new_cap < need {
        new_cap *= 2;
    }
    new_cap
}

impl<T: Send + 'static> ArenaAllocator<T> {
    /// Получение аллокатора из пула
    pub fn get() -> ArenaHandle<T> {
        let alloc = pop_from_pool().unwrap_or_else(Self::new);
        ArenaHandle(std::mem::ManuallyDrop::new(alloc))
    }
}

impl<T: Send + 'static> Drop for ArenaHandle<T> {
    /// Возврат аллокатора в пул
    fn drop(&mut self) {
        self.0.count = 0;
        let alloc = unsafe { std::mem::ManuallyDrop::take(&mut self.0) };
        push_to_pool(Box::new(alloc));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alloc_and_grow() {
        let mut a = ArenaAllocator::<f64>::new();
        let mut m1 = a.alloc(10);
        m1.as_mut_slice().copy_from_slice(&[1.0; 10]);
        // Вызывает рост (256 -> больше)
        let mut m2 = a.alloc(300);
        m2.as_mut_slice().copy_from_slice(&[2.0; 300]);
        // Старый вид остаётся валидным (как в C# — старый массив жив)
        assert_eq!(m1.as_slice()[0], 1.0);
        assert_eq!(m2.as_slice()[0], 2.0);
        assert_eq!(m1.len(), 10);
        assert_eq!(m2.len(), 300);
    }

    #[test]
    fn pool_reuse() {
        {
            let mut h = ArenaAllocator::<f64>::get();
            let m = h.alloc(100);
            assert_eq!(m.len(), 100);
        } // возврат в пул
        {
            let mut h = ArenaAllocator::<f64>::get();
            // Счётчик сброшен, буфер переиспользуется
            let m = h.alloc(50);
            assert_eq!(m.len(), 50);
        }
    }

    #[test]
    fn grow_cap_values() {
        assert_eq!(grow_cap(0, 100), 256);
        assert_eq!(grow_cap(256, 300), 512);
        assert_eq!(grow_cap(256, 256), 256);
        // 1024 -> 2048 -> 4096 -> 8192 (4096 < 5000, поэтому ещё одно удвоение)
        assert_eq!(grow_cap(1024, 5000), 8192);
    }
}
