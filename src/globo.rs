//! El globo de diálogo de claudio, como en los juegos: sale encima de él, el
//! texto se va escribiendo letra por letra y abajo vienen las opciones con
//! su flechita ▶. Todo en pixeles (unidades que luego se escalan), con una
//! fuentecita de 9 de alto hecha a mano: nomás las letras que se usan.

/// alto de una letra (fila 0: acentos; 1-7: mayúsculas y palos; 3-7: la
/// x; 8: lo que cuelga, como la p y la y)
pub const ALTO_LETRA: usize = 9;
/// aire entre el marco y el texto
const PAD: usize = 5;
/// entre el texto y las opciones
const ENTRE: usize = 4;
/// entre una opción y otra
const SEP: usize = 10;
/// la colita que apunta a claudio
pub const COLA: usize = 5;
/// letras por segundo
const VEL: f32 = 28.0;

pub const FONDO: u8 = 1;
pub const BORDE: u8 = 2;
pub const MARCO: u8 = 3;
pub const LETRA: u8 = 4;
pub const ACENTO: u8 = 5;
pub const COLORES: usize = 6;

fn glifo(c: char) -> &'static [&'static str; ALTO_LETRA] {
    const N: &str = "";
    match c {
        'a' => &[N, N, N, ".###.", "....#", ".####", "#...#", ".####", N],
        'b' => &[
            N, "#....", "#....", "####.", "#...#", "#...#", "#...#", "####.", N,
        ],
        'f' => &[N, "..##", ".#..", "####", ".#..", ".#..", ".#..", ".#..", N],
        'g' => &[
            N, N, N, ".####", "#...#", "#...#", ".####", "....#", ".###.",
        ],
        'j' => &[N, "..#", N, "..#", "..#", "..#", "..#", "..#", "##."],
        'k' => &[N, "#...", "#...", "#..#", "#.#.", "##..", "#.#.", "#..#", N],
        'm' => &[N, N, N, "##.#.", "#.#.#", "#.#.#", "#.#.#", "#.#.#", N],
        'q' => &[
            N, N, N, ".####", "#...#", "#...#", ".####", "....#", "....#",
        ],
        'r' => &[N, N, N, "#.##", "##..", "#...", "#...", "#...", N],
        'w' => &[N, N, N, "#...#", "#...#", "#.#.#", "#.#.#", ".#.#.", N],
        'x' => &[N, N, N, "#...#", ".#.#.", "..#..", ".#.#.", "#...#", N],
        'z' => &[N, N, N, "#####", "...#.", "..#..", ".#...", "#####", N],
        '.' => &[N, N, N, N, N, N, N, "#", N],
        ',' => &[N, N, N, N, N, N, N, ".#", "#."],
        'c' => &[N, N, N, ".####", "#....", "#....", "#....", ".####", N],
        'd' => &[
            N, "....#", "....#", ".####", "#...#", "#...#", "#...#", ".####", N,
        ],
        'e' => &[N, N, N, ".###.", "#...#", "#####", "#....", ".###.", N],
        'h' => &[
            N, "#....", "#....", "####.", "#...#", "#...#", "#...#", "#...#", N,
        ],
        'i' => &[N, "#", N, "#", "#", "#", "#", "#", N],
        'í' => &[".#", "#.", N, "#.", "#.", "#.", "#.", "#.", N],
        'l' => &[N, "#", "#", "#", "#", "#", "#", "#", N],
        'n' => &[N, N, N, "####.", "#...#", "#...#", "#...#", "#...#", N],
        'o' => &[N, N, N, ".###.", "#...#", "#...#", "#...#", ".###.", N],
        'p' => &[
            N, N, N, "####.", "#...#", "#...#", "####.", "#....", "#....",
        ],
        's' => &[N, N, N, ".####", "#....", ".###.", "....#", "####.", N],
        't' => &[N, ".#..", ".#..", "####", ".#..", ".#..", ".#..", "..##", N],
        'u' => &[N, N, N, "#...#", "#...#", "#...#", "#...#", ".####", N],
        'v' => &[N, N, N, "#...#", "#...#", "#...#", ".#.#.", "..#..", N],
        'y' => &[
            N, N, N, "#...#", "#...#", "#...#", ".####", "....#", "####.",
        ],
        '?' => &[
            N, ".###.", "#...#", "....#", "..##.", "..#..", ".....", "..#..", N,
        ],
        '¿' => &[
            N, N, "..#..", ".....", "..#..", ".##..", "#....", "#...#", ".###.",
        ],
        '!' => &[N, "#", "#", "#", "#", "#", N, "#", N],
        '¡' => &[N, N, "#", N, "#", "#", "#", "#", "#"],
        ':' => &[N, N, N, N, "#", N, N, "#", N],
        ')' => &[N, "#..", ".#.", "..#", "..#", "..#", ".#.", "#..", N],
        '▶' => &[N, "#...", "##..", "###.", "####", "###.", "##..", "#...", N],
        _ => &[N, "...", N, N, N, N, N, N, N],
    }
}

fn ancho_letra(c: char) -> usize {
    glifo(c)
        .iter()
        .map(|f| f.chars().count())
        .max()
        .unwrap_or(0)
}

/// Lo que mide un texto en pixeles (con uno de aire entre letras).
pub fn ancho(s: &str) -> usize {
    let n = s.chars().count();
    s.chars().map(ancho_letra).sum::<usize>() + n.saturating_sub(1)
}

pub struct Globo {
    pub texto: String,
    pub opciones: Vec<String>,
    /// la opción que tiene la flechita
    pub hover: usize,
    /// cuántas letras ya se escribieron
    mostrado: f32,
    /// segundos desde que salió
    pub edad: f32,
    pub w: usize,
    pub h: usize,
    /// la colita va arriba (el globo quedó abajo de quien habla, porque
    /// arriba no cabía)
    pub cola_arriba: bool,
}

impl Globo {
    pub fn new(texto: &str, opciones: &[&str]) -> Globo {
        let opciones: Vec<String> = opciones.iter().map(|s| s.to_string()).collect();
        let ops = opciones
            .iter()
            .map(|o| ancho("▶") + 2 + ancho(o))
            .sum::<usize>()
            + SEP * opciones.len().saturating_sub(1);
        let w = ancho(texto).max(ops) + 2 * PAD + 4;
        let h = 4
            + 2 * PAD
            + ALTO_LETRA
            + if opciones.is_empty() {
                0
            } else {
                ENTRE + ALTO_LETRA
            }
            + COLA;
        Globo {
            texto: texto.into(),
            opciones,
            hover: 0,
            mostrado: 0.0,
            edad: 0.0,
            w,
            h,
            cola_arriba: false,
        }
    }

    /// Otro texto en el mismo globo (sin opciones), escribiéndose de nuevo.
    pub fn cambia(&mut self, texto: &str) {
        self.texto = texto.into();
        self.opciones.clear();
        self.mostrado = 0.0;
    }

    pub fn avanza(&mut self, dt: f32) {
        self.edad += dt;
        self.mostrado += dt * VEL;
    }

    pub fn completo(&self) -> bool {
        self.mostrado as usize >= self.texto.chars().count()
    }

    /// Clic mientras se escribe: que salga todo de una vez.
    pub fn termina(&mut self) {
        self.mostrado = self.texto.chars().count() as f32;
    }

    /// Dónde va cada opción: (x0, x1) en la fila de las opciones.
    fn lugares(&self) -> Vec<(usize, usize)> {
        let mut x = 2 + PAD;
        self.opciones
            .iter()
            .map(|o| {
                let w = ancho("▶") + 2 + ancho(o);
                let r = (x, x + w);
                x += w + SEP;
                r
            })
            .collect()
    }

    /// dónde empieza la caja (con la colita arriba, abajo de ella)
    fn y_caja(&self) -> usize {
        if self.cola_arriba { COLA } else { 0 }
    }

    fn y_opciones(&self) -> usize {
        self.y_caja() + 2 + PAD + ALTO_LETRA + ENTRE
    }

    /// La opción bajo (x, y), en pixeles del globo. Con tantito margen para
    /// que sea fácil atinarle.
    pub fn opcion_en(&self, x: f32, y: f32) -> Option<usize> {
        if !self.completo() {
            return None;
        }
        let y0 = self.y_opciones() as f32;
        if y < y0 - 3.0 || y > y0 + ALTO_LETRA as f32 + 3.0 {
            return None;
        }
        self.lugares()
            .iter()
            .position(|&(x0, x1)| x >= x0 as f32 - 3.0 && x <= x1 as f32 + 3.0)
    }

    /// El dibujo: w×h celdas (0: transparente). `cola` es la columna a la que
    /// apunta la colita (donde anda claudio).
    pub fn grid(&self, cola: usize) -> Vec<u8> {
        let (w, h) = (self.w, self.h);
        let hb = h - COLA;
        let oy = self.y_caja();
        let mut g = vec![0u8; w * h];
        let pon = |g: &mut Vec<u8>, x: usize, y: usize, c: u8| {
            if x < w && y < h {
                g[y * w + x] = c;
            }
        };
        // la caja, con las esquinas redondeadas
        for y in 0..hb {
            for x in 0..w {
                let esquina = |d: usize| (x == d || x == w - 1 - d) && (y == d || y == hb - 1 - d);
                let d = x.min(y).min(w - 1 - x).min(hb - 1 - y);
                let c = match d {
                    _ if esquina(0) => 0,
                    _ if esquina(1) => BORDE,
                    0 => BORDE,
                    1 => MARCO,
                    _ => FONDO,
                };
                pon(&mut g, x, oy + y, c);
            }
        }
        // la colita: se abre el marco del lado de quien habla y sale en pico
        // (abajo, o arriba si el globo quedó abajo)
        let cx = cola.clamp(6, w - 7);
        let (orilla, dentro) = if self.cola_arriba {
            (oy, oy + 1)
        } else {
            (hb - 1, hb - 2)
        };
        for x in cx - 2..=cx + 2 {
            pon(&mut g, x, dentro, FONDO);
            pon(&mut g, x, orilla, FONDO);
        }
        pon(&mut g, cx - 3, orilla, MARCO);
        pon(&mut g, cx + 3, orilla, MARCO);
        for k in 0..COLA {
            let y = if self.cola_arriba {
                COLA - 1 - k
            } else {
                hb + k
            };
            if k + 1 == COLA {
                pon(&mut g, cx, y, BORDE);
                continue;
            }
            let medio = 3 - k.min(3);
            for x in cx - medio..=cx + medio {
                pon(&mut g, x, y, FONDO);
            }
            pon(&mut g, cx - medio, y, MARCO);
            pon(&mut g, cx + medio, y, MARCO);
            pon(&mut g, cx - medio - 1, y, BORDE);
            pon(&mut g, cx + medio + 1, y, BORDE);
        }
        // el texto, hasta donde va
        let n = self.mostrado as usize;
        let mut x = 2 + PAD;
        for c in self.texto.chars().take(n) {
            letra(&mut g, w, c, x, oy + 2 + PAD, LETRA);
            x += ancho_letra(c) + 1;
        }
        // las opciones, ya que acabó de escribir
        if self.completo() {
            let y = self.y_opciones();
            for (i, (o, &(x0, _))) in self.opciones.iter().zip(&self.lugares()).enumerate() {
                let elegida = i == self.hover;
                // la flechita parpadea en la elegida
                if elegida && (self.edad * 2.5).fract() < 0.7 {
                    letra(&mut g, w, '▶', x0, y, ACENTO);
                }
                let mut x = x0 + ancho("▶") + 2;
                for c in o.chars() {
                    letra(&mut g, w, c, x, y, if elegida { ACENTO } else { LETRA });
                    x += ancho_letra(c) + 1;
                }
            }
        }
        g
    }
}

fn letra(g: &mut [u8], w: usize, c: char, x: usize, y: usize, color: u8) {
    for (dy, f) in glifo(c).iter().enumerate() {
        for (dx, p) in f.chars().enumerate() {
            let i = (y + dy) * w + x + dx;
            if p == '#' && x + dx < w && i < g.len() {
                g[i] = color;
            }
        }
    }
}

/// Pinta el globo en un buffer ARGB de `w*px × h*px`.
pub fn pinta(g: &[u8], w: usize, colores: &[u32; COLORES], px: usize, buf: &mut [u8]) {
    let ancho = w * px;
    for (i, &c) in g.iter().enumerate() {
        let (x, y) = (i % w, i / w);
        let bytes = if c == 0 {
            [0; 4]
        } else {
            colores[c as usize].to_le_bytes()
        };
        for py in 0..px {
            let fila = (y * px + py) * ancho;
            for pxx in 0..px {
                let j = (fila + x * px + pxx) * 4;
                buf[j..j + 4].copy_from_slice(&bytes);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn letras_de_nueve_y_parejas() {
        for c in "abcdefghijklmnopqrstuvwxyzí¿?¡!:)▶.,".chars() {
            assert_ne!(glifo(c), glifo('\u{0}'), "falta la letra {c:?}");
            let gl = glifo(c);
            let w = ancho_letra(c);
            for f in gl.iter().filter(|f| !f.is_empty()) {
                assert_eq!(f.chars().count(), w, "{c:?}: {f}");
            }
        }
    }

    #[test]
    fn se_escribe_y_luego_salen_las_opciones() {
        let mut g = Globo::new("¿ocupas ayuda?", &["sí", "no"]);
        assert!(!g.completo());
        let y = (g.y_opciones() + 3) as f32;
        let (x0, _) = g.lugares()[1];
        assert_eq!(
            g.opcion_en(x0 as f32 + 2.0, y),
            None,
            "mientras escribe no hay opciones"
        );
        g.avanza(2.0);
        assert!(g.completo());
        assert_eq!(g.opcion_en(x0 as f32 + 2.0, y), Some(1));
        let celdas = g.grid(10);
        assert_eq!(celdas.len(), g.w * g.h);
        assert!(celdas.contains(&ACENTO), "la elegida va en color");
    }

    #[test]
    fn con_la_colita_arriba_todo_se_recorre() {
        let mut g = Globo::new("¿ocupas ayuda?", &["sí", "no"]);
        g.avanza(2.0);
        let abajo = g.grid(20);
        g.cola_arriba = true;
        let arriba = g.grid(20);
        // la caja es la misma, nomás recorrida COLA renglones
        let fila = |c: &[u8], y: usize| c[y * g.w..(y + 1) * g.w].to_vec();
        assert_eq!(fila(&arriba, COLA + 10), fila(&abajo, 10));
        // la punta de la colita: abajo en uno, arriba en el otro
        assert_eq!(arriba[20], BORDE);
        assert_eq!(abajo[(g.h - 1) * g.w + 20], BORDE);
        // y las opciones se atinan donde quedaron
        let y = (g.y_opciones() + 3) as f32;
        let (x0, _) = g.lugares()[1];
        assert_eq!(g.opcion_en(x0 as f32 + 2.0, y), Some(1));
    }

    #[test]
    fn la_cola_no_se_sale() {
        let g = Globo::new("¿ocupas ayuda?", &["sí", "no"]);
        for cola in [0, 3, g.w / 2, g.w - 1, g.w + 50] {
            let _ = g.grid(cola);
        }
    }
}
