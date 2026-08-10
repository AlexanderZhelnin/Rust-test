use bumpalo::{Bump, collections::Vec as BumpVec};

use crate::calc::{optimize_bump, translate};
use crate::models::{DrawProperties1, GrType, ILayer, IObraz, Legend, Primitive, Rect};
use crate::polygon::clip_polygon;
use crate::polyline::clip_polyline;

/// Повторяет C#-ветку частичного пересечения из `ClipPrimitives`
/// Строгие `<` и `>` важны, касание границы пересечением не считается
#[inline]
fn intersects(primitive: &Primitive, rect: &Rect) -> bool {
    primitive.rect.left < rect.right
        && primitive.rect.bottom < rect.top
        && primitive.rect.right > rect.left
        && primitive.rect.top > rect.bottom
}

/// Повторяет C#-проверку полного попадания примитива в прямоугольник
/// Здесь границы включены, поэтому используются `>=` и `<=`
#[inline]
fn is_inside(primitive: &Primitive, rect: &Rect) -> bool {
    primitive.rect.left >= rect.left
        && primitive.rect.bottom >= rect.bottom
        && primitive.rect.right <= rect.right
        && primitive.rect.top <= rect.top
}

/// Та же проверка диапазона видимости, что используется в обоих C# `Build`
#[inline]
fn is_visible(legend: &Legend, properties: &DrawProperties1) -> bool {
    legend.mashtab_range.min <= properties.mashtab && legend.mashtab_range.max >= properties.mashtab
}

/// Как в C#: для каждого образа сначала выполняется `Calc.Optimize`, затем
/// `Calc.Translate`. Порядок важен, потому что Translate меняет координаты
#[inline]
fn optimize_and_translate<'a>(
    bump: &'a Bump,
    coords: &'a mut [f64],
    mashtab: f64,
    properties: &DrawProperties1,
) -> &'a [f64] {
    let coords = optimize_bump(bump, coords, mashtab);
    translate(coords, properties);
    coords
}

/// Строит ровно один слой. Эту функцию используют и полный JSON-результат,
/// и ленивый по слоям подсчёт, чтобы геометрическая логика не расходилась
#[inline]
fn build_layer<'a>(
    bump: &'a Bump,
    legend: &'static Legend,
    properties: &DrawProperties1,
    rect: &Rect,
    mashtab: f64,
) -> ILayer<'a> {
    let mut obrazes = BumpVec::with_capacity_in(legend.primitives.len(), bump);

    // Как C# `ClipPrimitives`: обходим примитивы в исходном порядке
    for primitive in &legend.primitives {
        if is_inside(primitive, rect) {
            // Соответствует C# `Coords = [.. g.Coords]`: создаём отдельный
            // рабочий массив и не меняем координаты в глобальном `Init.ls`
            let copied_coords = bump.alloc_slice_copy(&primitive.coords);
            let coords = optimize_and_translate(bump, copied_coords, mashtab, properties);
            obrazes.push(IObraz {
                // Как C# `Name = g.Name`: используем ту же неизменяемую
                // строку без копирования её содержимого
                name: primitive.name.as_str(),
                coords,
            });
        } else if intersects(primitive, rect) {
            // Как C# `switch`: при частичном пересечении обрабатываются
            // только линии и полигоны, остальные типы результата не дают
            match legend.gr_type {
                GrType::Line => {
                    // `ClipPolyline` может разделить одну линию на несколько
                    // образов, и C# добавляет каждый из них отдельно
                    for coords in clip_polyline(bump, primitive, rect) {
                        let coords = optimize_and_translate(bump, coords, mashtab, properties);
                        obrazes.push(IObraz {
                            name: primitive.name.as_str(),
                            coords,
                        });
                    }
                }
                GrType::Polygon => {
                    let clipped = clip_polygon(bump, primitive, rect);
                    // Как в C#: полностью отсечённый пустой полигон пропускаем
                    if !clipped.is_empty() {
                        let coords = optimize_and_translate(bump, clipped, mashtab, properties);
                        obrazes.push(IObraz {
                            name: primitive.name.as_str(),
                            coords,
                        });
                    }
                }
                _ => {}
            }
        }
    }

    // Как в C#, прошедший проверку масштаба слой существует даже при пустом
    // списке образов, поэтому сохраняются его ID и позиция
    ILayer {
        legend_id: legend.id,
        obrazes: obrazes.into_bump_slice(),
    }
}

/// Повторяет C# `Drawer.Build`, меняя только способ выделения памяти на bumpalo
/// Результат должен быть сериализован до `Bump::reset()`
pub fn build<'a>(
    bump: &'a Bump,
    legends: &'static [Legend],
    properties: &DrawProperties1,
    rect: &Rect,
) -> &'a [ILayer<'a>] {
    let mut layers = BumpVec::with_capacity_in(legends.len(), bump);
    let mashtab = 1.0 / properties.scale;

    for legend in legends {
        // Та же проверка диапазона видимости, что в C#. Исходный порядок слоёв
        // сохраняется: дополнительной сортировки по Priority нет
        if !is_visible(legend, properties) {
            continue;
        }

        layers.push(build_layer(bump, legend, properties, rect, mashtab));
    }

    layers.into_bump_slice()
}

/// Аналог C# `BuildGenerator(...).Count()` для bump-версии.
///
/// Слой строится полностью той же функцией, что и для `/mapJSON`, после чего
/// его память становится ненужной и arena переиспользуется следующим слоем.
/// Возвращать такой слой наружу нельзя: `reset()` инвалидировал бы его ссылки
pub fn build_count_reusing(
    bump: &mut Bump,
    legends: &'static [Legend],
    properties: &DrawProperties1,
    rect: &Rect,
) -> usize {
    bump.reset();

    let mashtab = 1.0 / properties.scale;
    let mut count = 0;

    for legend in legends {
        if !is_visible(legend, properties) {
            continue;
        }

        {
            let arena: &Bump = bump;
            let _layer = build_layer(arena, legend, properties, rect, mashtab);
        }

        count += 1;
        bump.reset();
    }

    count
}
