use crate::calc::{optimize, translate};
use crate::models::{DrawProperties1, GrType, ILayer, IObraz, Legend, Rect};
use crate::polygon::clip_polygon;
use crate::polyline::clip_polyline;

/// Отсечение графических образов по прямоугольнику (оптимизировано)
fn clip_primitives(l: &Legend, rect: &Rect) -> Vec<IObraz> {
    let mut result = Vec::with_capacity(l.primitives.len());

    for g in &l.primitives {
        // Целиком лежит внутри прямоугольника
        if g.rect.left >= rect.left
            && g.rect.bottom >= rect.bottom
            && g.rect.right <= rect.right
            && g.rect.top <= rect.top
        {
            result.push(IObraz {
                coords: g.coords.clone(),
                name: g.name.clone(),
            });
        } else if g.rect.left < rect.right
            && g.rect.bottom < rect.top
            && g.rect.right > rect.left
            && g.rect.top > rect.bottom
        {
            // Необходимо отсекать
            match l.gr_type {
                GrType::Line => {
                    for cs in clip_polyline(g, rect) {
                        result.push(IObraz {
                            coords: cs,
                            name: g.name.clone(),
                        });
                    }
                }
                GrType::Polygon => {
                    let cs = clip_polygon(g, rect);
                    if !cs.is_empty() {
                        result.push(IObraz {
                            coords: cs,
                            name: g.name.clone(),
                        });
                    }
                }
                _ => {}
            }
        }
    }

    result
}

/// Подготовка данных для отрисовки (оптимизировано)
pub fn build(ls: &[Legend], pr: &mut DrawProperties1, rect: &mut Rect) -> Vec<ILayer> {
    let mut result = Vec::with_capacity(ls.len());
    let mashtab = 1.0 / pr.scale;

    for l in ls {
        // Проверка диапазона масштаба
        if l.mashtab_range.min > pr.mashtab || l.mashtab_range.max < pr.mashtab {
            continue;
        }

        let mut mas = Vec::with_capacity(l.primitives.len());

        for obraz in clip_primitives(l, rect) {
            let mut cs_opt = optimize(&obraz.coords, mashtab);
            translate(&mut cs_opt, pr);

            mas.push(IObraz {
                name: obraz.name,
                coords: cs_opt,
            });
        }

        result.push(ILayer {
            legend_id: l.id,
            obrazes: mas,
        });
    }

    result
}
