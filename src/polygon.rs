use bumpalo::{Bump, collections::Vec as BumpVec};

use crate::models::{Primitive, Rect};

#[inline]
fn get_next_index(index: usize, len: usize) -> usize {
    let next = index + 2;
    if next >= len { 0 } else { next }
}

fn clip_left<'a>(bump: &'a Bump, coords: &[f64], left: f64) -> &'a mut [f64] {
    if coords.is_empty() {
        return bump.alloc_slice_copy(coords);
    }

    let mut polygon = BumpVec::with_capacity_in(coords.len() * 2, bump);
    let mut index = 0;
    let (mut px1, mut py1) = (coords[0], coords[1]);

    if px1 >= left {
        polygon.push(px1);
        polygon.push(py1);
    }

    for _ in 1..=coords.len() / 2 {
        index = get_next_index(index, coords.len());
        let (px2, py2) = (coords[index], coords[index + 1]);

        if px1 >= left && px2 >= left {
            polygon.push(px2);
            polygon.push(py2);
        } else if px1 < left && px2 > left {
            let y = (left - px1) * (py2 - py1) / (px2 - px1) + py1;
            polygon.push(left);
            polygon.push(y);
            polygon.push(px2);
            polygon.push(py2);
        } else if px1 > left && px2 < left {
            let y = (left - px1) * (py2 - py1) / (px2 - px1) + py1;
            polygon.push(left);
            polygon.push(y);
        }

        // Как в C#: сдвигаем начало ребра в его конец, чтобы сохранить тот же
        // последовательный обход контура и порядок вычисления пересечений
        (px1, py1) = (px2, py2);
    }

    polygon.into_bump_slice_mut()
}

fn clip_right<'a>(bump: &'a Bump, coords: &[f64], right: f64) -> &'a mut [f64] {
    if coords.is_empty() {
        return bump.alloc_slice_copy(coords);
    }

    let mut polygon = BumpVec::with_capacity_in(coords.len() * 2, bump);
    let mut index = 0;
    let (mut px1, mut py1) = (coords[0], coords[1]);

    if px1 <= right {
        polygon.push(px1);
        polygon.push(py1);
    }

    for _ in 0..coords.len() / 2 {
        index = get_next_index(index, coords.len());
        let (px2, py2) = (coords[index], coords[index + 1]);

        if px1 <= right && px2 <= right {
            polygon.push(px2);
            polygon.push(py2);
        } else if px1 > right && px2 < right {
            let y = (right - px1) * (py2 - py1) / (px2 - px1) + py1;
            polygon.push(right);
            polygon.push(y);
            polygon.push(px2);
            polygon.push(py2);
        } else if px1 < right && px2 > right {
            let y = (right - px1) * (py2 - py1) / (px2 - px1) + py1;
            polygon.push(right);
            polygon.push(y);
        }

        (px1, py1) = (px2, py2);
    }

    polygon.into_bump_slice_mut()
}

fn clip_bottom<'a>(bump: &'a Bump, coords: &[f64], bottom: f64) -> &'a mut [f64] {
    if coords.is_empty() {
        return bump.alloc_slice_copy(coords);
    }

    let mut polygon = BumpVec::with_capacity_in(coords.len() * 2, bump);
    let mut index = 0;
    let (mut px1, mut py1) = (coords[0], coords[1]);

    if py1 >= bottom {
        polygon.push(px1);
        polygon.push(py1);
    }

    for _ in 0..coords.len() / 2 {
        index = get_next_index(index, coords.len());
        let (px2, py2) = (coords[index], coords[index + 1]);

        if py1 >= bottom && py2 >= bottom {
            polygon.push(px2);
            polygon.push(py2);
        } else if py1 < bottom && py2 > bottom {
            let x = (bottom - py1) * (px2 - px1) / (py2 - py1) + px1;
            polygon.push(x);
            polygon.push(bottom);
            polygon.push(px2);
            polygon.push(py2);
        } else if py1 > bottom && py2 < bottom {
            let x = (bottom - py1) * (px2 - px1) / (py2 - py1) + px1;
            polygon.push(x);
            polygon.push(bottom);
        }

        (px1, py1) = (px2, py2);
    }

    polygon.into_bump_slice_mut()
}

fn clip_top<'a>(bump: &'a Bump, coords: &[f64], top: f64) -> &'a mut [f64] {
    if coords.is_empty() {
        return bump.alloc_slice_copy(coords);
    }

    let mut polygon = BumpVec::with_capacity_in(coords.len() * 2, bump);
    let mut index = 0;
    let (mut px1, mut py1) = (coords[0], coords[1]);

    if py1 <= top {
        polygon.push(px1);
        polygon.push(py1);
    }

    for _ in 0..coords.len() / 2 {
        index = get_next_index(index, coords.len());
        let (px2, py2) = (coords[index], coords[index + 1]);

        if py1 <= top && py2 <= top {
            polygon.push(px2);
            polygon.push(py2);
        } else if py1 > top && py2 < top {
            let x = (top - py1) * (px2 - px1) / (py2 - py1) + px1;
            polygon.push(x);
            polygon.push(top);
            polygon.push(px2);
            polygon.push(py2);
        } else if py1 < top && py2 > top {
            let x = (top - py1) * (px2 - px1) / (py2 - py1) + px1;
            polygon.push(x);
            polygon.push(top);
        }

        (px1, py1) = (px2, py2);
    }

    polygon.into_bump_slice_mut()
}

pub fn clip_polygon<'a>(
    bump: &'a Bump,
    primitive: &'static Primitive,
    rect: &Rect,
) -> &'a mut [f64] {
    let mut result: &'a mut [f64] = if primitive.rect.left < rect.left {
        clip_left(bump, &primitive.coords, rect.left)
    } else {
        // C# делает `(double[])g.Coords.Clone()` до последующих отсечений
        bump.alloc_slice_copy(&primitive.coords)
    };

    if primitive.rect.bottom < rect.bottom {
        result = clip_bottom(bump, result, rect.bottom);
    }

    if primitive.rect.right > rect.right {
        result = clip_right(bump, result, rect.right);
    }

    if primitive.rect.top > rect.top {
        result = clip_top(bump, result, rect.top);
    }

    result
}
