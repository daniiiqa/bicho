//! Los animalitos en pixeles: una cuadrícula de 16×20 celdas (cada celda son
//! `escala` pixeles). Las 16×16 de abajo son el cuerpo (se puede girar para
//! trepar); las 4 de arriba quedan para el corazón, los zzz y las notas.

pub const ANCHO: usize = 16;
pub const ALTO: usize = 20;
/// filas libres arriba del cuerpo
const AIRE: usize = ALTO - ANCHO;

pub type Grid = [[u8; ANCHO]; ALTO];
type Bloque = [[u8; ANCHO]; ANCHO];

pub const CONTORNO: u8 = 1;
pub const CUERPO: u8 = 2;
pub const BRILLO: u8 = 3;
pub const OJO: u8 = 4;
pub const PUPILA: u8 = 5;
pub const CORAZON: u8 = 6;
pub const ZZZ: u8 = 7;
pub const NARIZ: u8 = 8;
pub const COLORES: usize = 9;

/// Un animalito: su cuerpo mirando a la derecha (o = contorno, b = cuerpo,
/// h = brillo, w = ojo, p = pupila, n = nariz, a/l = antenas y patas) y sus
/// patas. Sentado o dormido se dibuja sin patas, pegado al piso.
pub struct Especie {
    pub nombre: &'static str,
    cuerpo: &'static [&'static str],
    /// el otro cuadro al caminar (antenas, cola); None: el mismo
    cuerpo_b: Option<&'static [&'static str]>,
    patas_a: &'static [&'static str],
    patas_b: &'static [&'static str],
    /// camina a brincos (la baba)
    pub brinca: bool,
    /// las formas de algo blandito (squash & stretch); None: no se deforma
    blanda: Option<&'static Blanda>,
    /// de qué color del look es
    pub color: &'static [&'static str],
}

/// Cómo se deforma algo blandito: se aplasta antes de brincar, se estira en
/// el aire, se embarra al caer y tiembla como gelatina.
#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub enum Forma {
    #[default]
    Normal,
    /// más ancha y bajita (temblando, respirando)
    Ancha,
    /// agachada, cargando el brinco
    Aplastada,
    /// embarrada en el piso (al caer, o justo antes de despegar)
    Plasta,
    /// un poco más alta y angosta (el rebote)
    Alta,
    /// estirada en el aire
    Estirada,
}

struct Blanda {
    ancha: &'static [&'static str],
    aplastada: &'static [&'static str],
    plasta: &'static [&'static str],
    alta: &'static [&'static str],
    estirada: &'static [&'static str],
}

// las de la baba: todas centradas, con los ojos en las mismas columnas
const BABA_BLANDA: Blanda = Blanda {
    ancha: &[
        ".....oooooo.....",
        "...oobbbbbboo...",
        "..obhhbbbbbbbo..",
        ".obhbbwpbbwpbbo.",
        "obbbbbwpbbwpbbbo",
        "obbbbbbbbbbbbbbo",
        "obbbbbbbbbbbbbbo",
        ".oooooooooooooo.",
    ],
    aplastada: &[
        "....oooooooo....",
        "..oobbbbbbbboo..",
        ".obhhbbbbbbbbbo.",
        ".obbbbwpbbwpbbo.",
        "obbbbbwpbbwpbbbo",
        "obbbbbbbbbbbbbbo",
        ".oooooooooooooo.",
    ],
    plasta: &[
        "...oooooooooo...",
        ".oobhhbbbbbbboo.",
        "obbbbbwpbbwpbbbo",
        "obbbbbbbbbbbbbbo",
        ".oooooooooooooo.",
    ],
    alta: &[
        "......oooo......",
        ".....obbbbo.....",
        "....obhbbbbo....",
        "...obhbbbbbbo...",
        "...obbwpbbwpo...",
        "...obbwpbbwpo...",
        "...obbbbbbbbo...",
        "..obbbbbbbbbbo..",
        "..obbbbbbbbbbo..",
        "...oooooooooo...",
    ],
    estirada: &[
        ".......oo.......",
        "......obbo......",
        ".....obhbbo.....",
        ".....ohbbbo.....",
        "....owpbbwpo....",
        "....owpbbwpo....",
        "....obbbbbbo....",
        "....obbbbbbo....",
        "....obbbbbbo....",
        "...obbbbbbbbo...",
        "...obbbbbbbbo...",
        "....oooooooo....",
    ],
};

pub const BICHO: Especie = Especie {
    nombre: "bicho",
    cuerpo: &[
        "......a...a.....",
        ".......a.a......",
        ".....ooooooo....",
        "....obbbbbbbo...",
        "...obhhbbbbbbo..",
        "..obhbbbbwpbwpo.",
        "..obbbbbbwpbwpo.",
        "..obbbbbbbbbbbo.",
        "..obbbbbbbbbbbo.",
        "...obbbbbbbbbo..",
        "....ooooooooo...",
    ],
    cuerpo_b: Some(&[
        ".......a...a....",
        ".......a.a......",
        ".....ooooooo....",
        "....obbbbbbbo...",
        "...obhhbbbbbbo..",
        "..obhbbbbwpbwpo.",
        "..obbbbbbwpbwpo.",
        "..obbbbbbbbbbbo.",
        "..obbbbbbbbbbbo.",
        "...obbbbbbbbbo..",
        "....ooooooooo...",
    ]),
    patas_a: &["....l.l...l.l...", "...l...l.l...l.."],
    patas_b: &["....l.l...l.l...", "....l.l...l.l..."],
    brinca: false,
    blanda: None,
    color: &["acc", "color12"],
};

pub const MICHI: Especie = Especie {
    nombre: "michi",
    cuerpo: &[
        ".........o...o..",
        "........obo.obo.",
        ".o......obbobbo.",
        "obo.....obbbbbo.",
        "obo.....owpbwpo.",
        "obooooooobbnbbo.",
        "obbbhhhbbbbbbbo.",
        ".obbbbbbbbbbbo..",
        ".obbbbbbbbbbbo..",
        "..ooooooooooo...",
    ],
    cuerpo_b: Some(&[
        ".........o...o..",
        "........obo.obo.",
        "..o.....obbobbo.",
        ".obo....obbbbbo.",
        ".obo....owpbwpo.",
        ".oboooooobbnbbo.",
        "obbbhhhbbbbbbbo.",
        ".obbbbbbbbbbbo..",
        ".obbbbbbbbbbbo..",
        "..ooooooooooo...",
    ]),
    patas_a: &["...l.l....l.l...", "..l...l..l...l.."],
    patas_b: &["...l.l....l.l...", "...l.l....l.l..."],
    brinca: false,
    blanda: None,
    color: &["color4", "color13", "color12"],
};

pub const BABA: Especie = Especie {
    nombre: "baba",
    cuerpo: &[
        "......oooo......",
        "....oobbbboo....",
        "...obhhbbbbbo...",
        "..obhbbbbbbbbo..",
        "..obbbwpbbwpbo..",
        ".obbbbwpbbwpbbo.",
        ".obbbbbbbbbbbbo.",
        ".obbbbbbbbbbbbo.",
        "..oooooooooooo..",
    ],
    // lo demás lo hacen sus formas (BABA_BLANDA)
    cuerpo_b: None,
    patas_a: &[],
    patas_b: &[],
    brinca: true,
    blanda: Some(&BABA_BLANDA),
    color: &["color10", "color2", "color11"],
};

pub const TODAS: [&Especie; 3] = [&BICHO, &MICHI, &BABA];

pub fn especie(nombre: &str) -> Option<&'static Especie> {
    TODAS.into_iter().find(|e| e.nombre == nombre)
}

fn celda(c: char) -> u8 {
    match c {
        'o' | 'a' | 'l' => CONTORNO,
        'b' => CUERPO,
        'h' => BRILLO,
        'w' => OJO,
        'p' => PUPILA,
        'n' => NARIZ,
        _ => 0,
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Pose {
    /// el paso alterna las patas (y antenas, cola…)
    Camina(bool),
    Parado,
    Sentado,
    Duerme,
    /// en el aire: patas colgando
    Brinca,
    /// pegado a la pared; `pared_der`: la pared le queda a la derecha
    Trepa { paso: bool, pared_der: bool },
}

pub struct Look {
    pub pose: Pose,
    pub derecha: bool,
    pub parpadeo: bool,
    /// 0..1 mientras sube el corazón
    pub corazon: Option<f32>,
    /// fase de los zzz (segundos)
    pub zzz: Option<f32>,
    /// fase de las notas (bailando)
    pub notas: Option<f32>,
    /// cómo va deformado (solo los blanditos)
    pub forma: Forma,
}

/// Cierra los ojos: con ojos de dos renglones el de arriba se vuelve cuerpo
/// y el de abajo una rayita; con uno, rayita.
fn cierra(filas: &mut [Vec<char>]) {
    let con_ojo: Vec<usize> = (0..filas.len()).filter(|&i| filas[i].contains(&'w')).collect();
    for (k, &i) in con_ojo.iter().enumerate() {
        for c in filas[i].iter_mut() {
            *c = match *c {
                'w' | 'p' if k == 0 && con_ojo.len() > 1 => 'b',
                'w' => 'p',
                c => c,
            };
        }
    }
}

fn cuerpo(e: &Especie, pose: Pose, parpadeo: bool, forma: Forma) -> Bloque {
    let (otro, patas) = match pose {
        Pose::Camina(paso) | Pose::Trepa { paso, .. } => (paso, Some(if paso { e.patas_b } else { e.patas_a })),
        Pose::Parado => (false, Some(e.patas_a)),
        Pose::Brinca => (!e.brinca, Some(e.patas_b)),
        Pose::Sentado | Pose::Duerme => (false, None),
    };
    let base = match (e.blanda, forma, e.cuerpo_b) {
        (Some(bl), Forma::Ancha, _) => bl.ancha,
        (Some(bl), Forma::Aplastada, _) => bl.aplastada,
        (Some(bl), Forma::Plasta, _) => bl.plasta,
        (Some(bl), Forma::Alta, _) => bl.alta,
        (Some(bl), Forma::Estirada, _) => bl.estirada,
        (_, _, Some(b)) if otro => b,
        _ => e.cuerpo,
    };
    let mut filas: Vec<Vec<char>> = base.iter().chain(patas.unwrap_or(&[])).map(|f| f.chars().collect()).collect();
    if parpadeo || pose == Pose::Duerme {
        cierra(&mut filas);
    }
    let mut b: Bloque = [[0; ANCHO]; ANCHO];
    let arriba = ANCHO.saturating_sub(filas.len());
    for (i, f) in filas.iter().enumerate().take(ANCHO) {
        for (d, c) in b[arriba + i].iter_mut().zip(f) {
            *d = celda(*c);
        }
    }
    b
}

fn espejo<const H: usize>(g: &mut [[u8; ANCHO]; H]) {
    for f in g.iter_mut() {
        f.reverse();
    }
}

/// a la izquierda (contra reloj): lo que miraba a la derecha mira arriba y
/// las patas quedan a la derecha
fn gira_izq(b: &Bloque) -> Bloque {
    let mut n: Bloque = [[0; ANCHO]; ANCHO];
    for (r, f) in n.iter_mut().enumerate() {
        for (c, v) in f.iter_mut().enumerate() {
            *v = b[c][ANCHO - 1 - r];
        }
    }
    n
}

fn gira_der(b: &Bloque) -> Bloque {
    let mut n: Bloque = [[0; ANCHO]; ANCHO];
    for (r, f) in n.iter_mut().enumerate() {
        for (c, v) in f.iter_mut().enumerate() {
            *v = b[ANCHO - 1 - c][r];
        }
    }
    n
}

fn sello(g: &mut Grid, dibujo: &[&str], col: i32, renglon: i32, color: u8) {
    for (dy, f) in dibujo.iter().enumerate() {
        for (dx, c) in f.chars().enumerate() {
            let (x, y) = (col + dx as i32, renglon + dy as i32);
            if c != '.' && (0..ANCHO as i32).contains(&x) && (0..ALTO as i32).contains(&y) && g[y as usize][x as usize] == 0 {
                g[y as usize][x as usize] = color;
            }
        }
    }
}

const CORAZON_DIBUJO: [&str; 4] = [".r.r.", "rrrrr", ".rrr.", "..r.."];
const Z_DIBUJO: [&str; 3] = ["zzz", ".z.", "zzz"];
const NOTA_DIBUJO: [&str; 4] = ["..xx", "..x.", "xxx.", "xx.."];

/// Primer renglón con algo del cuerpo (para poner cosas encima).
fn tope(g: &Grid) -> i32 {
    g.iter().position(|f| f.iter().any(|&c| c != 0)).unwrap_or(AIRE) as i32
}

pub fn grid(e: &Especie, look: &Look) -> Grid {
    let mut b = cuerpo(e, look.pose, look.parpadeo, look.forma);
    if let Pose::Trepa { pared_der, .. } = look.pose {
        b = if pared_der {
            gira_izq(&b)
        } else {
            espejo(&mut b);
            gira_der(&b)
        };
    } else if !look.derecha {
        espejo(&mut b);
    }
    let mut g: Grid = [[0; ANCHO]; ALTO];
    g[AIRE..].copy_from_slice(&b);
    let tope = tope(&g);

    // lo de arriba se pinta del lado de la cabeza
    let lado = |col: i32, w: i32| if look.derecha { col } else { ANCHO as i32 - col - w };
    if let Some(p) = look.corazon {
        let sube = (p * 4.0) as i32;
        sello(&mut g, &CORAZON_DIBUJO, lado(9, 5), (tope - 4).min(3) - sube, CORAZON);
    }
    if let Some(t) = look.zzz {
        // dos zetas que suben y se van, desfasadas
        for (k, col) in [(0.0, 11), (0.5, 8)] {
            let f = (t * 0.6 + k) % 1.0;
            sello(&mut g, &Z_DIBUJO, lado(col + (f * 3.0) as i32, 3), tope.min(7) - (f * 8.0) as i32, ZZZ);
        }
    }
    if let Some(t) = look.notas {
        for (k, col) in [(0.0, 1), (0.5, 11)] {
            let f = (t * 0.5 + k) % 1.0;
            sello(&mut g, &NOTA_DIBUJO, col + ((f * 6.0).sin() * 1.5) as i32, tope.min(6) - (f * 7.0) as i32, ZZZ);
        }
    }
    g
}

/// Pinta la cuadrícula en un buffer ARGB (premultiplicado) de
/// `ANCHO*px × ALTO*px`.
pub fn pinta(g: &Grid, colores: &[u32; COLORES], px: usize, buf: &mut [u8]) {
    let w = ANCHO * px;
    for (y, f) in g.iter().enumerate() {
        for (x, &c) in f.iter().enumerate() {
            let argb = if c == 0 { 0 } else { colores[c as usize] };
            let bytes = argb.to_le_bytes();
            for py in 0..px {
                let fila = (y * px + py) * w;
                for pxx in 0..px {
                    let i = (fila + x * px + pxx) * 4;
                    buf[i..i + 4].copy_from_slice(&bytes);
                }
            }
        }
    }
}

/// Para `bicho ver`: la cuadrícula en la terminal, con medios bloques.
pub fn terminal(g: &Grid, colores: &[u32; COLORES]) -> String {
    let mut s = String::new();
    let rgb = |c: u8| {
        let v = colores[c as usize];
        ((v >> 16) & 0xff, (v >> 8) & 0xff, v & 0xff)
    };
    for y in (0..ALTO).step_by(2) {
        for (x, &a) in g[y].iter().enumerate() {
            let b = if y + 1 < ALTO { g[y + 1][x] } else { 0 };
            match (a, b) {
                (0, 0) => s.push(' '),
                (a, 0) => {
                    let (r, gg, bb) = rgb(a);
                    s += &format!("\x1b[38;2;{r};{gg};{bb}m▀\x1b[0m");
                }
                (0, b) => {
                    let (r, gg, bb) = rgb(b);
                    s += &format!("\x1b[38;2;{r};{gg};{bb}m▄\x1b[0m");
                }
                (a, b) => {
                    let (r, gg, bb) = rgb(a);
                    let (r2, g2, b2) = rgb(b);
                    s += &format!("\x1b[38;2;{r};{gg};{bb};48;2;{r2};{g2};{b2}m▀\x1b[0m");
                }
            }
        }
        s.push('\n');
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dibujos_de_16() {
        for e in TODAS {
            let bl = e.blanda.map(|b| [b.ancha, b.aplastada, b.plasta, b.alta, b.estirada].concat()).unwrap_or_default();
            let todas = e.cuerpo.iter().chain(e.cuerpo_b.unwrap_or(&[])).chain(e.patas_a).chain(e.patas_b).chain(&bl);
            for f in todas {
                assert_eq!(f.chars().count(), ANCHO, "{}: {f}", e.nombre);
            }
            assert!(e.cuerpo.len() + e.patas_a.len() <= ANCHO, "{}", e.nombre);
        }
    }

    #[test]
    fn trepando_las_patas_tocan_la_pared() {
        let look = |pared_der| Look { pose: Pose::Trepa { paso: false, pared_der }, derecha: true, parpadeo: false, corazon: None, zzz: None, notas: None, forma: Forma::Normal };
        for e in TODAS {
            let g = grid(e, &look(true));
            assert!(g.iter().any(|f| f[ANCHO - 1] != 0), "{}", e.nombre);
            let g = grid(e, &look(false));
            assert!(g.iter().any(|f| f[0] != 0), "{}", e.nombre);
        }
    }

    #[test]
    fn dormido_cierra_los_ojos() {
        for e in TODAS {
            let look = Look { pose: Pose::Duerme, derecha: true, parpadeo: false, corazon: None, zzz: None, notas: None, forma: Forma::Normal };
            assert!(!grid(e, &look).iter().flatten().any(|&c| c == OJO), "{}", e.nombre);
        }
    }
}
