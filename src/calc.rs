use crate::arena::ArenaSlice;
use crate::arena::allocator::ArenaAllocator;
use crate::models::DrawProperties1;
use std::arch::x86_64::*;

/// Преобразование в систему координат экрана
#[inline]
pub fn translate(cs: &mut [f64], pr: &DrawProperties1) {
    let count = cs.len() - (cs.len() % 4);
    let left_top_0 = pr.left_top[0];
    let left_top_1 = pr.left_top[1];
    let scale = pr.scale;

    let (vectors, tail) = cs.split_at_mut(count);
    if !vectors.is_empty() {
        // C# `Calc.Translate` обрабатывает те же четыре `double` через аппаратно
        // ускоренный `Vector<double>`. Здесь эквивалентный SIMD-путь выбран
        // явно, а для процессоров без AVX сохранён скалярный fallback
        if std::arch::is_x86_feature_detected!("avx") {
            // SAFETY: поддержка AVX проверена выше, длина `vectors` кратна 4
            unsafe { translate_avx(vectors, left_top_0, left_top_1, scale) };
        } else {
            translate_scalar(vectors, left_top_0, left_top_1, scale);
        }
    }

    // Координаты всегда идут парами, после блоков по четыре может остаться
    // только одна пара
    debug_assert!(tail.is_empty() || tail.len() == 2);
    if tail.len() == 2 {
        tail[0] = (tail[0] - left_top_0) * scale;
        tail[1] = (left_top_1 - tail[1]) * scale;
    }
}

#[target_feature(enable = "avx")]
unsafe fn translate_avx(cs: &mut [f64], left: f64, top: f64, scale: f64) {
    let left_top = _mm256_setr_pd(left, top, left, top);
    let scale = _mm256_setr_pd(scale, -scale, scale, -scale);
    let pointer = cs.as_mut_ptr();

    for i in (0..cs.len()).step_by(4) {
        // SAFETY: вызывающий передаёт длину, кратную четырём, поэтому каждая
        // невыравненная загрузка и запись целиком находится внутри `cs`
        let value = unsafe { _mm256_loadu_pd(pointer.add(i)) };
        let value = _mm256_mul_pd(_mm256_sub_pd(value, left_top), scale);
        unsafe { _mm256_storeu_pd(pointer.add(i), value) };
    }
}

#[inline]
fn translate_scalar(cs: &mut [f64], left: f64, top: f64, scale: f64) {
    for point_pair in cs.chunks_exact_mut(4) {
        point_pair[0] = (point_pair[0] - left) * scale;
        point_pair[1] = -(point_pair[1] - top) * scale;
        point_pair[2] = (point_pair[2] - left) * scale;
        point_pair[3] = -(point_pair[3] - top) * scale;
    }
}

/// Удаление точек которые не будут отображаться
pub fn optimize(mas: Vec<f64>, l: f64) -> Vec<f64> {
    let count = mas.len();
    if count < 5 {
        return mas;
    }

    // Предварительно вычисляем квадрат допуска
    let l_sq = l * l;

    // Оптимизированный вектор с резервированием памяти
    let mut coords: Vec<f64> = Vec::with_capacity(count);

    // Добавляем первую точку
    coords.push(mas[0]);
    coords.push(mas[1]);

    let mut last_coord1 = [mas[0], mas[1]];
    let mut last_coord2 = [mas[2], mas[3]];

    for i in (4..count).step_by(2) {
        if !is_point_on_line_simd(&last_coord1, &last_coord2, &[mas[i], mas[i + 1]], l_sq) {
            last_coord1 = [mas[i - 2], mas[i - 1]];
            last_coord2 = [mas[i], mas[i + 1]];

            coords.push(last_coord1[0]);
            coords.push(last_coord1[1]);
        }
    }

    // Добавляем последнюю точку
    coords.push(mas[count - 2]);
    coords.push(mas[count - 1]);

    coords
}

/// Находится ли следующая точка на линии с определённым допуском
#[inline]
#[allow(unused)]
pub fn is_point_on_line(p1: &[f64], p2: &[f64], p: &[f64], l_sq: f64) -> bool {
    // ab = p - p1
    let ab_x = p[0] - p1[0];
    let ab_y = p[1] - p1[1];

    // cd = p2 - p1
    let cd_x = p2[0] - p1[0];
    let cd_y = p2[1] - p1[1];

    // lenSQ = c*c + d*d
    let len_sq = cd_x * cd_x + cd_y * cd_y;

    // C# использует param = -1 для вырожденного отрезка и затем
    // проверяет расстояние до p1
    let param = if len_sq != 0.0 {
        (ab_x * cd_x + ab_y * cd_y) / len_sq
    } else {
        -1.0
    };

    // Вычисляем ближайшую точку на линии
    let (xx, yy) = if param < 0.0 {
        (p1[0], p1[1])
    } else if param > 1.0 {
        (p2[0], p2[1])
    } else {
        (p1[0] + param * cd_x, p1[1] + param * cd_y)
    };

    // dP = p - xy
    let dx = p[0] - xx;
    let dy = p[1] - yy;

    // distance_sq < l_sq
    (dx * dx + dy * dy) < l_sq
}

/// Находится ли следующая точка на линии с определённым допуском (SIMD версия)
/// Использует SSE4.1 для ускорения вычислений
#[cfg(target_arch = "x86_64")]
#[inline]
pub fn is_point_on_line_simd(p1: &[f64; 2], p2: &[f64; 2], p: &[f64; 2], l_sq: f64) -> bool {
    unsafe {
        // Загружаем значения в Vector128
        let v_p = _mm_loadu_pd(p.as_ptr());
        let v_p1 = _mm_loadu_pd(p1.as_ptr());
        let v_p2 = _mm_loadu_pd(p2.as_ptr());

        // ab = p - p1
        let ab = _mm_sub_pd(v_p, v_p1);

        // cd = p2 - p1
        let cd = _mm_sub_pd(v_p2, v_p1);

        // lenSQ = c*c + d*d (используем dot product с маской 255 для суммирования всех элементов)
        let len_sq_vec = _mm_dp_pd(cd, cd, 255);
        let len_sq_simd = _mm_cvtsd_f64(len_sq_vec);

        // Как в C#: вырожденный отрезок означает param = -1,
        // после чего расстояние считается до p1
        let param = if len_sq_simd != 0.0 {
            let ab_cd_dot = _mm_dp_pd(ab, cd, 255);
            _mm_cvtsd_f64(ab_cd_dot) / len_sq_simd
        } else {
            -1.0
        };

        // Вычисляем ближайшую точку на линии
        let xy = if param < 0.0 {
            v_p1
        } else if param > 1.0 {
            v_p2
        } else {
            // xy = p1 + cd * param
            let param_vec = _mm_set1_pd(param);
            let cd_param = _mm_mul_pd(cd, param_vec);
            _mm_add_pd(v_p1, cd_param)
        };

        // dP = p - xy
        let dp = _mm_sub_pd(v_p, xy);

        // distance_sq < l_sq
        let dp_dot = _mm_dp_pd(dp, dp, 255);
        _mm_cvtsd_f64(dp_dot) < l_sq
    }
}

pub fn optimize_blazing(
    mas: &[f64],
    allocator: &mut ArenaAllocator<f64>,
    l: f64,
) -> ArenaSlice<f64> {
    let count = mas.len();
    if count < 5 {
        let mut coords = allocator.alloc(count);
        coords.as_mut_slice().copy_from_slice(mas);
        return coords;
    }

    let mut result = allocator.alloc(count);

    let l_sq = l * l;

    let dest = result.as_mut_slice();
    dest[0] = mas[0];
    dest[1] = mas[1];
    let mut p2 = 2usize;

    let mut last_coord1 = [mas[0], mas[1]];
    let mut last_coord2 = [mas[2], mas[3]];

    for i in (4..count).step_by(2) {
        if !is_point_on_line_simd(&last_coord1, &last_coord2, &[mas[i], mas[i + 1]], l_sq) {
            last_coord1 = [mas[i - 2], mas[i - 1]];
            last_coord2 = [mas[i], mas[i + 1]];

            dest[p2] = last_coord1[0];
            p2 += 1;
            dest[p2] = last_coord1[1];
            p2 += 1;
        }
    }

    dest[p2] = mas[count - 2];
    p2 += 1;
    dest[p2] = mas[count - 1];
    p2 += 1;

    result.sub(0..p2)
}
