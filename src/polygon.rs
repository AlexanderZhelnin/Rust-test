use crate::models::{Primitive, Rect};

/// Получить следующий индекс (циклически)
fn get_next_index(cur_index: usize, len: usize) -> usize {
    let next = cur_index + 2;
    if next >= len { 0 } else { next }
}

/// Отсечение слева (оптимизировано)
fn clip_left(coords: &[f64], left: f64) -> Vec<f64> {
    if coords.is_empty() {
        return coords.to_vec();
    }

    let mut pl = Vec::with_capacity(coords.len() * 2);
    let mut cur_index = 0;

    let (px1, py1) = (coords[0], coords[1]);

    if px1 >= left {
        pl.push(px1);
        pl.push(py1);
    }

    let len = coords.len() / 2;
    for _ in 1..=len {
        cur_index = get_next_index(cur_index, coords.len());
        let (px2, py2) = (coords[cur_index], coords[cur_index + 1]);

        if px1 >= left && px2 >= left {
            pl.push(px2);
            pl.push(py2);
        } else if px1 < left && px2 > left {
            // Вычисляем точку пересечения один раз
            let intersect_y = (left - px1) * (py2 - py1) / (px2 - px1) + py1;
            pl.push(left);
            pl.push(intersect_y);
            pl.push(px2);
            pl.push(py2);
        } else if px1 > left && px2 < left {
            let intersect_y = (left - px1) * (py2 - py1) / (px2 - px1) + py1;
            pl.push(left);
            pl.push(intersect_y);
        }
    }

    pl
}

/// Отсечение справа (оптимизировано)
fn clip_right(coords: &[f64], right: f64) -> Vec<f64> {
    if coords.is_empty() {
        return coords.to_vec();
    }

    let mut pl = Vec::with_capacity(coords.len() * 2);
    let mut cur_index = 0;

    let (px1, py1) = (coords[0], coords[1]);

    if px1 <= right {
        pl.push(px1);
        pl.push(py1);
    }

    let len = coords.len() / 2;
    for _ in 0..len {
        cur_index = get_next_index(cur_index, coords.len());

        let px2 = coords[cur_index];
        let py2 = coords[cur_index + 1];

        if px1 <= right && px2 <= right {
            pl.push(px2);
            pl.push(py2);
        } else if px1 > right && px2 < right {
            let intersect_y = (right - px1) * (py2 - py1) / (px2 - px1) + py1;
            pl.push(right);
            pl.push(intersect_y);
            pl.push(px2);
            pl.push(py2);
        } else if px1 < right && px2 > right {
            let intersect_y = (right - px1) * (py2 - py1) / (px2 - px1) + py1;
            pl.push(right);
            pl.push(intersect_y);
        }
    }

    pl
}

/// Отсечение снизу (оптимизировано)
fn clip_bottom(coords: &[f64], bottom: f64) -> Vec<f64> {
    if coords.is_empty() {
        return coords.to_vec();
    }

    let mut pl = Vec::with_capacity(coords.len() * 2);
    let mut cur_index = 0;

    let (px1, py1) = (coords[0], coords[1]);

    if py1 >= bottom {
        pl.push(px1);
        pl.push(py1);
    }

    let len = coords.len() / 2;
    for _ in 0..len {
        cur_index = get_next_index(cur_index, coords.len());
        let (px2, py2) = (coords[cur_index], coords[cur_index + 1]);

        if py1 >= bottom && py2 >= bottom {
            pl.push(px2);
            pl.push(py2);
        } else if py1 < bottom && py2 > bottom {
            // Вычисляем точку пересечения один раз
            let intersect_x = (bottom - py1) * (px2 - px1) / (py2 - py1) + px1;
            pl.push(intersect_x);
            pl.push(bottom);
            pl.push(px2);
            pl.push(py2);
        } else if py1 > bottom && py2 < bottom {
            let intersect_x = (bottom - py1) * (px2 - px1) / (py2 - py1) + px1;
            pl.push(intersect_x);
            pl.push(bottom);
        }
    }

    pl
}

/// Отсечение сверху (оптимизировано)
fn clip_top(coords: &[f64], top: f64) -> Vec<f64> {
    if coords.is_empty() {
        return coords.to_vec();
    }

    let mut pl = Vec::with_capacity(coords.len() * 2);
    let mut cur_index = 0;

    let (px1, py1) = (coords[0], coords[1]);

    if py1 <= top {
        pl.push(px1);
        pl.push(py1);
    }

    let len = coords.len() / 2;
    for _ in 0..len {
        cur_index = get_next_index(cur_index, coords.len());
        let (px2, py2) = (coords[cur_index], coords[cur_index + 1]);

        if py1 <= top && py2 <= top {
            pl.push(px2);
            pl.push(py2);
        } else if py1 > top && py2 < top {
            // Вычисляем точку пересечения один раз
            let intersect_x = (top - py1) * (px2 - px1) / (py2 - py1) + px1;
            pl.push(intersect_x);
            pl.push(top);
            pl.push(px2);
            pl.push(py2);
        } else if py1 < top && py2 > top {
            let intersect_x = (top - py1) * (px2 - px1) / (py2 - py1) + px1;
            pl.push(intersect_x);
            pl.push(top);
        }
    }

    pl
}

/// Отсечение полигона по прямоугольнику (оптимизировано)
pub fn clip_polygon(g: &Primitive, rect: &Rect) -> Vec<f64> {
    let mut res = if g.rect.left < rect.left {
        clip_left(&g.coords, rect.left)
    } else {
        g.coords.clone()
    };

    if g.rect.bottom < rect.bottom {
        res = clip_bottom(&res, rect.bottom);
    }

    if g.rect.right > rect.right {
        res = clip_right(&res, rect.right);
    }

    if g.rect.top > rect.top {
        res = clip_top(&res, rect.top);
    }

    res
}
