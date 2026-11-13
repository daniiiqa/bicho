//! Las letras de las terminales como repisas: si un bicho anda metido en una
//! terminal, un hilo aparte le saca foto a la ventana (grim) cada ratito,
//! encuentra las líneas de texto y en cada una los tramos de letras donde
//! cabe parado (arriba tiene que haber hueco del alto de su cuerpo). Los
//! animalitos y el globo se tapan en la foto para no confundirlos con
//! letras.

use std::collections::HashMap;
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Cada cuánto vuelve a ver las terminales.
const CADA: Duration = Duration::from_millis(400);
/// Qué tan distinto del fondo tiene que ser un pixel para ser letra
/// (suma de las diferencias de r, g y b).
const TINTA: i32 = 90;
/// Huecos entre letras más chicos que esto (px) siguen siendo la misma repisa
/// (los espacios entre palabras no la cortan).
const HUECO: usize = 14;
/// El fondo se calcula por cuadritos de este tamaño (px), no de toda la foto.
const CUADRITO: usize = 32;

/// Un rectángulo (x, y, ancho, alto).
pub type Rect = (f32, f32, f32, f32);

/// Lo que el hilo tiene que ver: una terminal y quiénes andan ahí.
#[derive(Clone, Debug)]
pub struct Pedido {
    /// el address de la ventana
    pub id: String,
    /// la ventana, en coordenadas globales (lógicas)
    pub rect: Rect,
    /// lo que se fotografía: nomás alrededor de los animalitos (hasta donde
    /// les alcanza un brinco), dentro de la ventana
    pub zona: Rect,
    /// lo que se tapa en la foto (animalitos, globo), en globales
    pub tapar: Vec<Rect>,
    /// el lienzo de un animalito (ancho, alto)
    pub lienzo: (f32, f32),
    /// dónde va su cuerpo dentro del lienzo: columnas (x0, x1) y alto
    pub cuerpo: (f32, f32, f32),
}

/// Donde se puede parar: `y` es la del lienzo parado ahí, y `x0..x1` las x
/// del lienzo que caben, contando desde la esquina de la ventana (así, si la
/// mueves, las repisas se van con ella).
#[derive(Clone, Debug, PartialEq)]
pub struct Repisa {
    pub y: f32,
    pub x0: f32,
    pub x1: f32,
}

#[derive(Default)]
pub struct Estado {
    pub pedidos: Vec<Pedido>,
    pub repisas: HashMap<String, Vec<Repisa>>,
}

pub type Compartido = Arc<Mutex<Estado>>;

/// Arranca el hilo que ve las terminales.
pub fn arranca() -> Compartido {
    let c: Compartido = Arc::default();
    let hilo = c.clone();
    std::thread::spawn(move || {
        let mut antes: HashMap<String, (Huella, Vec<Repisa>)> = HashMap::new();
        loop {
            std::thread::sleep(CADA);
            let pedidos = hilo.lock().map(|e| e.pedidos.clone()).unwrap_or_default();
            let mut nuevas = HashMap::new();
            for p in &pedidos {
                let Some(img) = foto(p.zona) else { continue };
                // la misma foto que la vez pasada: las mismas repisas
                let huella = (
                    huella(&img.rgb),
                    p.zona.0.to_bits(),
                    p.zona.1.to_bits(),
                    p.rect.0.to_bits(),
                    p.rect.1.to_bits(),
                );
                let r = match antes.remove(&p.id) {
                    Some((h, r)) if h == huella => r,
                    _ => repisas(&img, p),
                };
                nuevas.insert(p.id.clone(), (huella, r));
            }
            antes = nuevas;
            if let Ok(mut e) = hilo.lock() {
                e.repisas = antes
                    .iter()
                    .map(|(id, (_, r))| (id.clone(), r.clone()))
                    .collect();
            }
        }
    });
    c
}

/// Lo que identifica una foto: su contenido y dónde se tomó (zona y ventana).
type Huella = (u64, u32, u32, u32, u32);

/// Una huella rápida de la foto (FNV-1a).
fn huella(d: &[u8]) -> u64 {
    d.iter().fold(0xcbf2_9ce4_8422_2325u64, |h, &b| {
        (h ^ b as u64).wrapping_mul(0x0100_0000_01b3)
    })
}

/// Una foto en RGB.
pub struct Imagen {
    pub w: usize,
    pub h: usize,
    pub rgb: Vec<u8>,
}

fn foto((x, y, w, h): Rect) -> Option<Imagen> {
    let g = format!(
        "{},{} {}x{}",
        x.round() as i32,
        y.round() as i32,
        w.round() as i32,
        h.round() as i32
    );
    let out = Command::new("grim")
        .args(["-g", &g, "-t", "ppm", "-"])
        .stderr(Stdio::null())
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    ppm(&out.stdout)
}

/// Lee un PPM binario (P6, 255).
fn ppm(d: &[u8]) -> Option<Imagen> {
    let mut campos = Vec::new();
    let mut i = 0;
    while campos.len() < 4 {
        while d.get(i)?.is_ascii_whitespace() {
            i += 1;
        }
        let ini = i;
        while !d.get(i)?.is_ascii_whitespace() {
            i += 1;
        }
        campos.push(std::str::from_utf8(&d[ini..i]).ok()?.to_string());
    }
    let (w, h): (usize, usize) = (campos[1].parse().ok()?, campos[2].parse().ok()?);
    if campos[0] != "P6" || campos[3] != "255" {
        return None;
    }
    let rgb = d.get(i + 1..i + 1 + w * h * 3)?.to_vec();
    Some(Imagen { w, h, rgb })
}

/// Las repisas de una foto de la terminal.
pub fn repisas(img: &Imagen, p: &Pedido) -> Vec<Repisa> {
    // se trabaja en la zona de la foto; las repisas salen contando desde la
    // esquina de la ventana
    let (rx, ry, rw, rh) = p.zona;
    let (dx, dy) = (rx - p.rect.0, ry - p.rect.1);
    let (w, h) = (rw.round().max(1.0) as usize, rh.round().max(1.0) as usize);
    // pixeles de la foto por pixel lógico (con escala de monitor)
    let k = img.w as f32 / w as f32;
    let px = |x: usize, y: usize| {
        let (ix, iy) = (
            ((x as f32 + 0.5) * k) as usize,
            ((y as f32 + 0.5) * k) as usize,
        );
        let i = (iy.min(img.h - 1) * img.w + ix.min(img.w - 1)) * 3;
        (
            img.rgb[i] as i32,
            img.rgb[i + 1] as i32,
            img.rgb[i + 2] as i32,
        )
    };
    // el fondo, por cuadritos: el color que más sale alrededor de cada uno
    // (en el cuadrito y sus vecinos, para que una línea de letras gorda no le
    // gane). Con la terminal transparente el wallpaper de atrás cambia de un
    // lado a otro, pero despacito; las letras sí contrastan con lo de junto
    let (cx, cy) = (w.div_ceil(CUADRITO), h.div_ceil(CUADRITO));
    let mut fondos = vec![(0, 0, 0); cx * cy];
    // conteo por color (5 bits por canal), reusado: nomás se limpia lo que
    // se tocó
    let mut cuenta = vec![0u16; 1 << 15];
    let mut tocados: Vec<usize> = Vec::new();
    for ty in 0..cy {
        for tx in 0..cx {
            for &i in &tocados {
                cuenta[i] = 0;
            }
            tocados.clear();
            let (y0, x0) = (
                ty.saturating_sub(1) * CUADRITO,
                tx.saturating_sub(1) * CUADRITO,
            );
            let (y1, x1) = (((ty + 2) * CUADRITO).min(h), ((tx + 2) * CUADRITO).min(w));
            for y in (y0..y1).step_by(3) {
                for x in (x0..x1).step_by(3) {
                    let (r, g, b) = px(x, y);
                    let i = ((r >> 3) << 10 | (g >> 3) << 5 | (b >> 3)) as usize;
                    if cuenta[i] == 0 {
                        tocados.push(i);
                    }
                    cuenta[i] += 1;
                }
            }
            if let Some(&i) = tocados.iter().max_by_key(|&&i| cuenta[i]) {
                let (r, g, b) = ((i >> 10) as i32, ((i >> 5) & 31) as i32, (i & 31) as i32);
                fondos[ty * cx + tx] = ((r << 3) + 4, (g << 3) + 4, (b << 3) + 4);
            }
        }
    }
    let mut tinta = vec![false; w * h];
    for y in 0..h {
        for x in 0..w {
            let (r, g, b) = px(x, y);
            let f = fondos[(y / CUADRITO) * cx + x / CUADRITO];
            tinta[y * w + x] = (r - f.0).abs() + (g - f.1).abs() + (b - f.2).abs() > TINTA;
        }
    }
    // lo que no es letra: los animalitos y el globo
    for &(tx, ty, tw, th) in &p.tapar {
        let (x0, y0) = ((tx - rx).max(0.0) as usize, (ty - ry).max(0.0) as usize);
        let (x1, y1) = (
            ((tx + tw - rx).max(0.0) as usize).min(w),
            ((ty + th - ry).max(0.0) as usize).min(h),
        );
        for y in y0..y1 {
            tinta[y * w + x0.min(w)..y * w + x1.max(x0.min(w))].fill(false);
        }
    }
    let fila_con = |y: usize| tinta[y * w..(y + 1) * w].iter().any(|&t| t);

    let (cw, ch) = p.lienzo;
    let (bx0, bx1, alto) = (
        p.cuerpo.0 as usize,
        p.cuerpo.1 as usize,
        p.cuerpo.2.ceil() as usize,
    );
    let mut out = Vec::new();
    let mut y = 0;
    while y < h {
        // el renglón de arriba de una línea de texto
        if !fila_con(y) || (y > 0 && fila_con(y - 1)) {
            y += 1;
            continue;
        }
        let y0 = y;
        let mut y1 = y;
        while y1 + 1 < h && fila_con(y1 + 1) {
            y1 += 1;
        }
        y = y1 + 1;
        if y0 < alto {
            continue;
        }
        // por columna: ¿hay letras en la línea? ¿cuánto hueco hay arriba?
        let con: Vec<bool> = (0..w)
            .map(|x| (y0..=y1).any(|yy| tinta[yy * w + x]))
            .collect();
        let libre: Vec<bool> = (0..w)
            .map(|x| (y0 - alto..y0).all(|yy| !tinta[yy * w + x]))
            .collect();
        // los tramos de letras (los huecos chicos no cortan)
        let mut tramos: Vec<(usize, usize)> = Vec::new();
        for x in (0..w).filter(|&x| con[x]) {
            match tramos.last_mut() {
                Some(t) if x - t.1 <= HUECO => t.1 = x,
                _ => tramos.push((x, x)),
            }
        }
        for (s0, s1) in tramos {
            // dónde puede ir el lienzo: con el centro sobre las letras y el
            // cuerpo sin chocar con nada
            let medio = cw / 2.0;
            let desde = (s0 as f32 - medio).max(0.0) as usize;
            let hasta = ((s1 as f32 - medio).max(0.0) as usize).min(w.saturating_sub(cw as usize));
            let mut corrida: Option<(usize, usize)> = None;
            let cierra = |c: &mut Option<(usize, usize)>, out: &mut Vec<Repisa>| {
                if let Some((a, b)) = c.take() {
                    out.push(Repisa {
                        y: dy + y0 as f32 - ch,
                        x0: dx + a as f32,
                        x1: dx + b as f32,
                    });
                }
            };
            for lx in desde..=hasta {
                let cabe = (lx + bx0..=(lx + bx1).min(w - 1)).all(|c| libre[c]);
                if cabe {
                    corrida = Some(corrida.map_or((lx, lx), |(a, _)| (a, lx)));
                } else {
                    cierra(&mut corrida, &mut out);
                }
            }
            cierra(&mut corrida, &mut out);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Una "terminal" de mentiras: fondo oscuro y bloques claros donde hay
    /// letras.
    fn terminal(w: usize, h: usize, letras: &[(usize, usize, usize, usize)]) -> Imagen {
        let mut rgb = vec![20u8; w * h * 3];
        for &(x, y, lw, lh) in letras {
            for yy in y..y + lh {
                for xx in x..x + lw {
                    rgb[(yy * w + xx) * 3..(yy * w + xx) * 3 + 3].fill(230);
                }
            }
        }
        Imagen { w, h, rgb }
    }

    fn pedido(w: f32, h: f32) -> Pedido {
        Pedido {
            id: "a".into(),
            rect: (100.0, 50.0, w, h),
            zona: (100.0, 50.0, w, h),
            tapar: vec![],
            lienzo: (48.0, 60.0),
            cuerpo: (6.0, 41.0, 30.0),
        }
    }

    #[test]
    fn se_para_en_la_linea_si_cabe_arriba() {
        // una línea larga abajo (y=200) con todo libre arriba
        let img = terminal(600, 300, &[(20, 200, 400, 16)]);
        let r = repisas(&img, &pedido(600.0, 300.0));
        assert_eq!(r.len(), 1, "{r:?}");
        assert_eq!(r[0].y, 200.0 - 60.0);
        assert!(r[0].x0 >= 0.0 && r[0].x1 <= 420.0);
    }

    #[test]
    fn no_cabe_si_la_linea_de_arriba_tiene_letras() {
        // dos líneas pegadas: en la de abajo no cabe (la de arriba estorba),
        // en la de arriba sí
        let img = terminal(600, 300, &[(20, 180, 400, 16), (20, 200, 400, 16)]);
        let r = repisas(&img, &pedido(600.0, 300.0));
        assert!(r.iter().all(|r| r.y == 180.0 - 60.0), "{r:?}");
        // pero si la de abajo es más larga, en lo que sobra sí cabe
        let img = terminal(600, 300, &[(20, 180, 100, 16), (20, 200, 500, 16)]);
        let r = repisas(&img, &pedido(600.0, 300.0));
        assert!(
            r.iter().any(|r| r.y == 200.0 - 60.0 && r.x0 > 120.0 - 10.0),
            "{r:?}"
        );
    }

    #[test]
    fn con_zona_las_repisas_cuentan_desde_la_ventana() {
        // la foto es nomás un pedazo (desde x=200, y=100 de la ventana)
        let img = terminal(300, 200, &[(20, 150, 200, 16)]);
        let mut p = pedido(1000.0, 800.0);
        p.zona = (100.0 + 200.0, 50.0 + 100.0, 300.0, 200.0);
        let r = repisas(&img, &p);
        assert_eq!(r.len(), 1, "{r:?}");
        assert_eq!(r[0].y, 100.0 + 150.0 - 60.0);
        assert!(r[0].x0 >= 200.0 && r[0].x1 <= 200.0 + 220.0, "{r:?}");
    }

    #[test]
    fn con_la_terminal_transparente_el_wallpaper_no_son_letras() {
        // fondo que va de oscuro a claro de lado a lado (un wallpaper
        // difuminado atrás) y una línea de letras que contrasta con lo suyo
        let (w, h) = (600, 300);
        let mut rgb = vec![0u8; w * h * 3];
        for y in 0..h {
            for x in 0..w {
                let v = (20 + x * 180 / w) as u8;
                rgb[(y * w + x) * 3..(y * w + x) * 3 + 3].copy_from_slice(&[v, v / 2, v]);
            }
        }
        for y in 200..216 {
            for x in 20..420 {
                let v = rgb[(y * w + x) * 3];
                let letra = if v > 120 { v - 110 } else { v + 110 };
                rgb[(y * w + x) * 3..(y * w + x) * 3 + 3].fill(letra);
            }
        }
        let r = repisas(&Imagen { w, h, rgb }, &pedido(600.0, 300.0));
        assert!(
            !r.is_empty() && r.iter().all(|r| r.y == 200.0 - 60.0),
            "{r:?}"
        );
    }

    #[test]
    fn los_animalitos_no_son_letras() {
        let img = terminal(600, 300, &[(300, 100, 40, 40)]);
        let mut p = pedido(600.0, 300.0);
        p.tapar = vec![(100.0 + 290.0, 50.0 + 95.0, 60.0, 60.0)];
        assert!(repisas(&img, &p).is_empty());
    }

    #[test]
    fn lee_ppm() {
        let mut d = b"P6\n2 1\n255\n".to_vec();
        d.extend([1, 2, 3, 4, 5, 6]);
        let img = ppm(&d).unwrap();
        assert_eq!((img.w, img.h, img.rgb.len()), (2, 1, 6));
    }
}

/// Con las terminales de verdad: `cargo test en_vivo -- --ignored --nocapture`
#[cfg(test)]
#[test]
#[ignore]
fn en_vivo() {
    let m = crate::hypr::mundo(&crate::hypr::monitor_enfocado().unwrap(), 2.0);
    for v in m.ventanas.iter().filter(|v| m.clis.contains(&v.id)) {
        let rect = (v.x + m.origen.0, v.y + m.origen.1, v.w, v.h);
        // como en la vida real: nomás alrededor de un animalito
        let zona = (
            rect.0 + rect.2 / 2.0 - 345.0,
            rect.1 + rect.3 / 2.0 - 270.0,
            690.0f32.min(rect.2),
            540.0f32.min(rect.3),
        );
        let p = Pedido {
            id: v.id.clone(),
            rect,
            zona,
            tapar: vec![],
            lienzo: (48.0, 60.0),
            cuerpo: (6.0, 41.0, 30.0),
        };
        let t = std::time::Instant::now();
        let img = foto(p.zona).expect("grim");
        let tf = t.elapsed();
        let r = repisas(&img, &p);
        println!(
            "{} {:?}: {} repisas; foto {:?}, análisis {:?}",
            v.id,
            p.rect,
            r.len(),
            tf,
            t.elapsed() - tf
        );
        for x in r.iter().take(8) {
            println!("   y={} x={}..{}", x.y, x.x0, x.x1);
        }
    }
}
