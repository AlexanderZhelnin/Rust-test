use bumpalo::{Bump, collections::Vec as BumpVec};

use crate::models::{Primitive, Rect};

type Segments<'a> = BumpVec<'a, &'a mut [f64]>;

fn clip_left<'a>(bump: &'a Bump, coords: &[f64], left: f64) -> Segments<'a> {
    let mut result = Segments::new_in(bump);
    if coords.is_empty() {
        return result;
    }

    let (mut px1, mut py1) = (coords[0], coords[1]);
    let mut polyline = BumpVec::with_capacity_in(coords.len() * 2, bump);

    if px1 >= left {
        polyline.push(px1);
        polyline.push(py1);
    }

    for i in (2..coords.len()).step_by(2) {
        let (px2, py2) = (coords[i], coords[i + 1]);

        if px1 >= left && px2 >= left {
            polyline.push(px2);
            polyline.push(py2);
        } else if px1 < left && px2 > left {
            let y = (left - px1) * (py2 - py1) / (px2 - px1) + py1;
            polyline.push(left);
            polyline.push(y);
            polyline.push(px2);
            polyline.push(py2);
        } else if px1 > left && px2 < left {
            let y = (left - px1) * (py2 - py1) / (px2 - px1) + py1;
            polyline.push(left);
            polyline.push(y);
            result.push(polyline.into_bump_slice_mut());
            polyline = BumpVec::with_capacity_in(coords.len() * 2, bump);
        }

        (px1, py1) = (px2, py2);
    }

    if !polyline.is_empty() {
        result.push(polyline.into_bump_slice_mut());
    }

    result
}

fn clip_right<'a>(bump: &'a Bump, coords: &[f64], right: f64) -> Segments<'a> {
    let mut result = Segments::new_in(bump);
    if coords.is_empty() {
        return result;
    }

    let (mut px1, mut py1) = (coords[0], coords[1]);
    let mut polyline = BumpVec::with_capacity_in(coords.len() * 2, bump);

    if px1 <= right {
        polyline.push(px1);
        polyline.push(py1);
    }

    for i in (2..coords.len()).step_by(2) {
        let (px2, py2) = (coords[i], coords[i + 1]);

        if px1 <= right && px2 <= right {
            polyline.push(px2);
            polyline.push(py2);
        } else if px1 > right && px2 < right {
            let y = (right - px1) * (py2 - py1) / (px2 - px1) + py1;
            polyline.push(right);
            polyline.push(y);
            polyline.push(px2);
            polyline.push(py2);
        } else if px1 < right && px2 > right {
            let y = (right - px1) * (py2 - py1) / (px2 - px1) + py1;
            polyline.push(right);
            polyline.push(y);
            result.push(polyline.into_bump_slice_mut());
            polyline = BumpVec::with_capacity_in(coords.len() * 2, bump);
        }

        (px1, py1) = (px2, py2);
    }

    if !polyline.is_empty() {
        result.push(polyline.into_bump_slice_mut());
    }

    result
}

fn clip_bottom<'a>(bump: &'a Bump, coords: &[f64], bottom: f64) -> Segments<'a> {
    let mut result = Segments::new_in(bump);
    if coords.is_empty() {
        return result;
    }

    let (mut px1, mut py1) = (coords[0], coords[1]);
    let mut polyline = BumpVec::with_capacity_in(coords.len() * 2, bump);

    if py1 >= bottom {
        polyline.push(px1);
        polyline.push(py1);
    }

    for i in (2..coords.len()).step_by(2) {
        let (px2, py2) = (coords[i], coords[i + 1]);

        if py1 >= bottom && py2 >= bottom {
            polyline.push(px2);
            polyline.push(py2);
        } else if py1 < bottom && py2 > bottom {
            let x = (bottom - py1) * (px2 - px1) / (py2 - py1) + px1;
            polyline.push(x);
            polyline.push(bottom);
            polyline.push(px2);
            polyline.push(py2);
        } else if py1 > bottom && py2 < bottom {
            let x = (bottom - py1) * (px2 - px1) / (py2 - py1) + px1;
            // Как в C#: перед точкой пересечения повторно добавляем предыдущую
            // точку, сохраняя тот же состав и порядок координат сегмента
            polyline.push(px1);
            polyline.push(py1);
            polyline.push(x);
            polyline.push(bottom);
            result.push(polyline.into_bump_slice_mut());
            polyline = BumpVec::with_capacity_in(coords.len() * 2, bump);
        }

        (px1, py1) = (px2, py2);
    }

    if !polyline.is_empty() {
        result.push(polyline.into_bump_slice_mut());
    }

    result
}

fn clip_top<'a>(bump: &'a Bump, coords: &[f64], top: f64) -> Segments<'a> {
    let mut result = Segments::new_in(bump);
    if coords.is_empty() {
        return result;
    }

    let (mut px1, mut py1) = (coords[0], coords[1]);
    let mut polyline = BumpVec::with_capacity_in(coords.len() * 2, bump);

    if py1 <= top {
        polyline.push(px1);
        polyline.push(py1);
    }

    for i in (2..coords.len()).step_by(2) {
        let (px2, py2) = (coords[i], coords[i + 1]);

        if py1 <= top && py2 <= top {
            polyline.push(px2);
            polyline.push(py2);
        } else if py1 > top && py2 < top {
            let x = (top - py1) * (px2 - px1) / (py2 - py1) + px1;
            polyline.push(x);
            polyline.push(top);
            polyline.push(px2);
            polyline.push(py2);
        } else if py1 < top && py2 > top {
            let x = (top - py1) * (px2 - px1) / (py2 - py1) + px1;
            polyline.push(px1);
            polyline.push(py1);
            polyline.push(x);
            polyline.push(top);
            result.push(polyline.into_bump_slice_mut());
            polyline = BumpVec::with_capacity_in(coords.len() * 2, bump);
        }

        (px1, py1) = (px2, py2);
    }

    if !polyline.is_empty() {
        result.push(polyline.into_bump_slice_mut());
    }

    result
}

pub fn clip_polyline<'a>(
    bump: &'a Bump,
    primitive: &'static Primitive,
    rect: &Rect,
) -> Segments<'a> {
    let mut result = if primitive.rect.left < rect.left {
        clip_left(bump, &primitive.coords, rect.left)
    } else {
        let mut segments = Segments::with_capacity_in(1, bump);
        // C# делает `(double[])g.Coords.Clone()` до последующих отсечений
        let coords: &'a mut [f64] = bump.alloc_slice_copy(&primitive.coords);
        segments.push(coords);
        segments
    };

    if primitive.rect.bottom < rect.bottom {
        let mut clipped = Segments::with_capacity_in(result.len() * 2, bump);
        for segment in result {
            for next in clip_bottom(bump, segment, rect.bottom) {
                clipped.push(next);
            }
        }
        result = clipped;
    }

    if primitive.rect.right > rect.right {
        let mut clipped = Segments::with_capacity_in(result.len() * 2, bump);
        for segment in result {
            for next in clip_right(bump, segment, rect.right) {
                clipped.push(next);
            }
        }
        result = clipped;
    }

    if primitive.rect.top > rect.top {
        let mut clipped = Segments::with_capacity_in(result.len() * 2, bump);
        for segment in result {
            for next in clip_top(bump, segment, rect.top) {
                clipped.push(next);
            }
        }
        result = clipped;
    }

    result
}
