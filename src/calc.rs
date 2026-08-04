use crate::models::DrawProperties1;
use std::arch::x86_64::*;

/// Преобразование в систему координат экрана
#[inline]
pub fn translate(cs: &mut [f64], pr: &DrawProperties1) {
    let count = cs.len() - (cs.len() % 4);

    // Развертывание цикла для уменьшения накладных расходов
    let left_top_0 = pr.left_top[0];
    let left_top_1 = pr.left_top[1];
    let scale = pr.scale;

    let mut i = 0;
    while i < count {
        cs[i] = (cs[i] - left_top_0) * scale;
        cs[i + 1] = -(cs[i + 1] - left_top_1) * scale;
        cs[i + 2] = (cs[i + 2] - left_top_0) * scale;
        cs[i + 3] = -(cs[i + 3] - left_top_1) * scale;

        i += 4;
    }

    if count >= cs.len() {
        return;
    }

    // Обработка оставшихся элементов
    let last_idx = cs.len().saturating_sub(2);
    cs[last_idx] = (cs[last_idx] - left_top_0) * scale;
    cs[last_idx + 1] = -(cs[last_idx + 1] - left_top_1) * scale;
}

/// Удаление точек которые не будут отображаться
pub fn optimize(mas: &[f64], l: f64) -> Vec<f64> {
    let count = mas.len();
    if count < 5 {
        return mas.to_vec();
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

    if len_sq == 0.0 {
        return false;
    }

    // param = (a*c + b*d) / lenSQ
    let param = (ab_x * cd_x + ab_y * cd_y) / len_sq;

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

        if len_sq_simd == 0.0 {
            return false;
        }

        // param = (a*c + b*d) / lenSQ
        let ab_cd_dot = _mm_dp_pd(ab, cd, 255);
        let param = _mm_cvtsd_f64(ab_cd_dot) / len_sq_simd;

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
