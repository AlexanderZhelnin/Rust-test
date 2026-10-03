import io

path = r"c:\Project\Rust-test\src\drawer.rs"
with io.open(path, "r", encoding="utf-8", newline="") as f:
    lines = f.readlines()

le = "\r\n" if lines[0].endswith("\r\n") else "\n"

new_code = """        // Отсечение полилинии может разбить один примитив на несколько
        // сегментов, поэтому заранее неизвестно, сколько образов получится
        // в слое. Сначала собираем их во временный Vec, затем копируем в
        // арену точного размера
        let mut mas_vec: Vec<ObrazResultBlazing> = Vec::with_capacity(l.primitives.len());

        for g in &l.primitives {
            let r = g.rect;

            if r.left >= left && r.bottom >= bottom && r.right <= right && r.top <= top {
                // Целиком лежит внутри прямоугольника
                let mut coords = optimize_blazing(&g.coords, allocator_f64, distance);
                translate(coords.as_mut_slice(), pr);
                mas_vec.push(ObrazResultBlazing {
                    name: g.name.clone(),
                    coords,
                });
            } else {
                // Необходимо отсекать
                match l.gr_type {
                    GrType::Line => {
                        for cs in clip_polyline(g, rect) {
                            let mut coords = optimize_blazing(&cs, allocator_f64, distance);
                            translate(coords.as_mut_slice(), pr);
                            mas_vec.push(ObrazResultBlazing {
                                name: g.name.clone(),
                                coords,
                            });
                        }
                    }
                    GrType::Polygon => {
                        let cs = clip_polygon(g, rect);
                        if !cs.is_empty() {
                            let mut coords = optimize_blazing(&cs, allocator_f64, distance);
                            translate(coords.as_mut_slice(), pr);
                            mas_vec.push(ObrazResultBlazing {
                                name: g.name.clone(),
                                coords,
                            });
                        }
                    }
                    _ => {}
                }
            }
        }

        // Копируем собранные образы в арену точного размера
        let mas = allocator_obrazes.alloc(mas_vec.len());
        let mas_ptr = mas.as_mut_slice().as_mut_ptr();
        // SAFETY: `mas` выделен на `mas_vec.len()` элементов, слоты ещё не
        // инициализированы (арена хранит `MaybeUninit`), поэтому пишем через
        // `ptr::write`, не дропая старые значения
        for (i, item) in mas_vec.iter().enumerate() {
            unsafe { std::ptr::write(mas_ptr.add(i), item.clone()); }
        }

        sp[count] = LayerResultBlazing {
            legend_id: l.id,
            obrazes: mas,
        };
        count += 1;
"""

new_code_lines = new_code.split("\n")
if new_code_lines and new_code_lines[-1] == "":
    new_code_lines = new_code_lines[:-1]
new_code_lines = [l + le for l in new_code_lines]

new_lines = lines[:105] + new_code_lines + lines[160:]

with io.open(path, "w", encoding="utf-8", newline="") as f:
    f.writelines(new_lines)

print("Done, total lines:", len(new_lines))