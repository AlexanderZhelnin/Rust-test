use crate::arena::allocator::ArenaAllocator;
use crate::arena::memory::ArenaSlice;
use crate::calc::{optimize, optimize_blazing, translate};
use crate::models::{
    DrawProperties1, GrType, Layer, LayerResultBlazing, LayerResultBlazingPtr, Legend, Obraz,
    ObrazResultBlazing, ObrazResultBlazingPtr, PtrString, Rect,
};
use crate::polygon::clip_polygon;
use crate::polyline::clip_polyline;

use std::sync::Arc;

/// Лениво передаёт каждый отсечённый образ потребителю, как C# `yield return`.
/// `Arc<str>` остаётся owning-типом, поэтому модели результата не получают
/// lifetime-параметров, а generic callback мономорфизуется компилятором
#[inline]
fn visit_clipped_primitives(l: &Legend, rect: &Rect, mut emit: impl FnMut(Vec<f64>, Arc<str>)) {
    for g in &l.primitives {
        // Целиком лежит внутри прямоугольника
        if g.rect.left >= rect.left
            && g.rect.bottom >= rect.bottom
            && g.rect.right <= rect.right
            && g.rect.top <= rect.top
        {
            emit(g.coords.clone(), g.name.clone());
        } else if g.rect.left < rect.right
            && g.rect.bottom < rect.top
            && g.rect.right > rect.left
            && g.rect.top > rect.bottom
        {
            // Необходимо отсекать
            match l.gr_type {
                GrType::Line => {
                    for cs in clip_polyline(g, rect) {
                        emit(cs, g.name.clone());

                    }
                }
                GrType::Polygon => {
                    let cs = clip_polygon(g, rect);
                    if !cs.is_empty() {
                        emit(cs, g.name.clone());
                    }
                }
                _ => {}
            }
        }
    }
}

/// Ленивый аналог C# `BuildGenerator`: каждый слой строится только тогда,
/// когда потребитель запрашивает следующий элемент
pub fn build_iter(ls: &[Legend], pr: &DrawProperties1, rect: &Rect) -> impl Iterator<Item = Layer> {
    let mashtab = 1.0 / pr.scale;

    ls.iter().filter_map(move |l| {
        // Проверка диапазона масштаба
        if l.mashtab_range.min > pr.mashtab || l.mashtab_range.max < pr.mashtab {
            return None;
        }

        let mut mas = Vec::with_capacity(l.primitives.len());

        visit_clipped_primitives(l, rect, |coords, name| {
            // Для короткой геометрии `Optimize` возвращает входной Vec,
            // как C#, без второй копии
            let mut cs_opt = optimize(coords, mashtab);
            translate(&mut cs_opt, pr);

            mas.push(Obraz {
                name,
                coords: cs_opt,
            });
        });

        Some(Layer {
            legend_id: l.id,
            obrazes: mas,
        })
    })
}

/// Ленивый аналог C# `BuildGenerator`: каждый слой строится только тогда,
/// когда потребитель запрашивает следующий элемент
pub fn build_blazing(
    ls: &[Legend],
    allocator_f64: &mut ArenaAllocator<f64>,
    allocator_obrazes: &mut ArenaAllocator<ObrazResultBlazing>,
    allocator_layers: &mut ArenaAllocator<LayerResultBlazing>,
    pr: &DrawProperties1,
    rect: &Rect,
) -> ArenaSlice<LayerResultBlazing> {
    let distance = pr.scale;

    let mut result = allocator_layers.alloc(ls.len());
    let sp = result.as_mut_slice();
    let mut count = 0;

    let (left, top, right, bottom) = (rect.left, rect.top, rect.right, rect.bottom);

    for l in ls {
        if l.mashtab_range.min > pr.mashtab || l.mashtab_range.max < pr.mashtab {
            continue;
        }

        let mut mas = allocator_obrazes.alloc(l.primitives.len());
        let g_sp = mas.as_mut_slice();

        let mut index = 0;

        for g in &l.primitives {
            let r = g.rect;

            if r.left >= left && r.bottom >= bottom && r.right <= right && r.top <= top {
                // Целиком лежит внутри прямоугольника
                let mut coords = optimize_blazing(&g.coords, allocator_f64, distance);
                translate(coords.as_mut_slice(), pr);
                g_sp[index] = ObrazResultBlazing {
                    name: g.name.clone(),
                    coords,
                };
                index += 1;
            } else {
                // Необходимо отсекать
                match l.gr_type {
                    GrType::Line => {
                        for cs in clip_polyline(g, rect) {
                            let mut coords = optimize_blazing(&cs, allocator_f64, distance);
                            translate(coords.as_mut_slice(), pr);
                            g_sp[index] = ObrazResultBlazing {
                                name: g.name.clone(),
                                coords,
                            };
                            index += 1;
                        }
                    }
                    GrType::Polygon => {
                        let cs = clip_polygon(g, rect);
                        if !cs.is_empty() {
                            let mut coords = optimize_blazing(&cs, allocator_f64, distance);
                            translate(coords.as_mut_slice(), pr);
                            g_sp[index] = ObrazResultBlazing {
                                name: g.name.clone(),
                                coords,
                            };
                            index += 1;
                        }
                    }
                    _ => {}
                }
            }
        }

        sp[count] = LayerResultBlazing {
            legend_id: l.id,
            obrazes: mas.sub(0..index),
        };
        count += 1;
    }

    result.sub(0..count)
}

pub fn build_blazing_ptr(
    ls: &[Legend],
    allocator_f64: &mut ArenaAllocator<f64>,
    allocator_obrazes: &mut ArenaAllocator<ObrazResultBlazingPtr>,
    allocator_layers: &mut ArenaAllocator<LayerResultBlazingPtr>,
    pr: &DrawProperties1,
    rect: &Rect,
) -> ArenaSlice<LayerResultBlazingPtr> {
    let distance = pr.scale;

    let mut result = allocator_layers.alloc(ls.len());
    let sp = result.as_mut_slice();
    let mut count = 0;

    let (left, top, right, bottom) = (rect.left, rect.top, rect.right, rect.bottom);

    for l in ls {
        if l.mashtab_range.min > pr.mashtab || l.mashtab_range.max < pr.mashtab {
            continue;
        }

        let mut mas = allocator_obrazes.alloc(l.primitives.len());
        let g_sp = mas.as_mut_slice();

        let mut index = 0;

        for g in &l.primitives {
            let r = g.rect;

            if r.left >= left && r.bottom >= bottom && r.right <= right && r.top <= top {
                // Целиком лежит внутри прямоугольника
                let mut coords = optimize_blazing(&g.coords, allocator_f64, distance);
                translate(coords.as_mut_slice(), pr);
                g_sp[index] = ObrazResultBlazingPtr {
                    name: unsafe { PtrString::from_raw(g.name.as_ptr(), g.name.len()) },
                    coords,
                };
                index += 1;
            } else {
                // Необходимо отсекать
                match l.gr_type {
                    GrType::Line => {
                        for cs in clip_polyline(g, rect) {
                            let mut coords = optimize_blazing(&cs, allocator_f64, distance);
                            translate(coords.as_mut_slice(), pr);
                            g_sp[index] = ObrazResultBlazingPtr {
                                name: unsafe { PtrString::from_raw(g.name.as_ptr(), g.name.len()) },
                                coords,
                            };
                            index += 1;
                        }
                    }
                    GrType::Polygon => {
                        let cs = clip_polygon(g, rect);
                        if !cs.is_empty() {
                            let mut coords = optimize_blazing(&cs, allocator_f64, distance);
                            translate(coords.as_mut_slice(), pr);
                            g_sp[index] = ObrazResultBlazingPtr {
                                name: unsafe { PtrString::from_raw(g.name.as_ptr(), g.name.len()) },
                                coords,
                            };
                            index += 1;
                        }
                    }
                    _ => {}
                }
            }
        }

        sp[count] = LayerResultBlazingPtr {
            legend_id: l.id,
            obrazes: mas.sub(0..index),
        };
        count += 1;
    }

    result.sub(0..count)
}

/// Eager-вариант для `/mapJSON`, соответствующий C# `Drawer.Build`
pub fn build(ls: &[Legend], pr: &DrawProperties1, rect: &Rect) -> Vec<Layer> {
    let mut result = Vec::with_capacity(ls.len());
    result.extend(build_iter(ls, pr, rect));
    result
}
