use crate::models::{Primitive, Rect};

/// Отсечение слева (для полилинии, оптимизировано)
fn clip_left(coords: &[f64], left: f64) -> Vec<Vec<f64>> {
    let mut res = Vec::new();
    if coords.is_empty() {
        return res;
    }

    // Предварительно вычисляем значения
    let (mut px1, mut py1) = (coords[0], coords[1]);

    let mut pl = Vec::with_capacity(coords.len() * 2);

    if px1 >= left {
        pl.push(px1);
        pl.push(py1);
    }

    for i in (2..coords.len()).step_by(2) {
        let (px2, py2) = (coords[i], coords[i + 1]);

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

            res.push(pl);
            pl = Vec::with_capacity(coords.len());
        }
        (px1, py1) = (px2, py2);
    }

    if !pl.is_empty() {
        res.push(pl);
    }

    res
}

/// Отсечение справа (для полилинии, оптимизировано)
fn clip_right(coords: &[f64], right: f64) -> Vec<Vec<f64>> {
    let mut res = Vec::new();
    if coords.is_empty() {
        return res;
    }

    let (mut px1, mut py1) = (coords[0], coords[1]);

    let mut pl = Vec::with_capacity(coords.len());

    if px1 <= right {
        pl.push(px1);
        pl.push(py1);
    }

    for i in (2..coords.len()).step_by(2) {
        let (px2, py2) = (coords[i], coords[i + 1]);

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

            res.push(pl);
            pl = Vec::with_capacity(coords.len());
        }

        (px1, py1) = (px2, py2);
    }

    if !pl.is_empty() {
        res.push(pl);
    }

    res
}

/// Отсечение снизу (для полилинии, оптимизировано)
fn clip_bottom(coords: &[f64], bottom: f64) -> Vec<Vec<f64>> {
    let mut res = Vec::new();
    if coords.is_empty() {
        return res;
    }

    let (mut px1, mut py1) = (coords[0], coords[1]);

    let mut pl = Vec::with_capacity(coords.len() * 2);

    if py1 >= bottom {
        pl.push(px1);
        pl.push(py1);
    }

    for i in (2..coords.len()).step_by(2) {
        let (px2, py2) = (coords[i], coords[i + 1]);

        if py1 >= bottom && py2 >= bottom {
            pl.push(px2);
            pl.push(py2);
        } else if py1 < bottom && py2 > bottom {
            let intersect_x = (bottom - py1) * (px2 - px1) / (py2 - py1) + px1;
            pl.push(intersect_x);
            pl.push(bottom);
            pl.push(px2);
            pl.push(py2);
        } else if py1 > bottom && py2 < bottom {
            let intersect_x = (bottom - py1) * (px2 - px1) / (py2 - py1) + px1;
            pl.push(intersect_x);
            pl.push(bottom);

            res.push(pl);
            pl = Vec::with_capacity(coords.len());
        }

        (px1, py1) = (px2, py2);
    }

    if !pl.is_empty() {
        res.push(pl);
    }

    res
}

/// Отсечение сверху (для полилинии, оптимизировано)
fn clip_top(coords: &[f64], top: f64) -> Vec<Vec<f64>> {
    let mut res = Vec::new();
    if coords.is_empty() {
        return res;
    }

    let (mut px1, mut py1) = (coords[0], coords[1]);

    let mut pl = Vec::with_capacity(coords.len() * 2);

    if py1 <= top {
        pl.push(px1);
        pl.push(py1);
    }

    for i in (2..coords.len()).step_by(2) {
        let (px2, py2) = (coords[i], coords[i + 1]);

        if py1 <= top && py2 <= top {
            pl.push(px2);
            pl.push(py2);
        } else if py1 > top && py2 < top {
            let intersect_x = (top - py1) * (px2 - px1) / (py2 - py1) + px1;
            pl.push(intersect_x);
            pl.push(top);
            pl.push(px2);
            pl.push(py2);
        } else if py1 < top && py2 > top {
            let intersect_x = (top - py1) * (px2 - px1) / (py2 - py1) + px1;
            pl.push(intersect_x);
            pl.push(top);

            res.push(pl);
            pl = Vec::with_capacity(coords.len());
        }

        (px1, py1) = (px2, py2);
    }

    if !pl.is_empty() {
        res.push(pl);
    }

    res
}

/// Отсечение полилинии по прямоугольнику (оптимизировано)
pub fn clip_polyline(g: &Primitive, rect: &Rect) -> Vec<Vec<f64>> {
    let mut res = if g.rect.left < rect.left {
        clip_left(&g.coords, rect.left)
    } else {
        vec![g.coords.clone()]
    };

    if g.rect.bottom < rect.bottom {
        let mut tmp = Vec::with_capacity(res.len() * 2);
        for cs in &res {
            tmp.extend(clip_bottom(cs, rect.bottom));
        }
        res = tmp;
    }

    if g.rect.right > rect.right {
        let mut tmp = Vec::with_capacity(res.len() * 2);
        for cs in &res {
            tmp.extend(clip_right(cs, rect.right));
        }
        res = tmp;
    }

    if g.rect.top > rect.top {
        let mut tmp = Vec::with_capacity(res.len() * 2);
        for cs in &res {
            tmp.extend(clip_top(cs, rect.top));
        }
        res = tmp;
    }

    res
}
