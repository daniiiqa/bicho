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
    /// con un clic te pregunta si ocupas ayuda y abre a Claude (claudio)
    pub invoca: bool,
    /// de qué color del look es (o un "#rrggbb" fijo)
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
    invoca: false,
    // sin colores del look (fuera de discada): el suyo
    color: &["acc", "color12", "#7aa2f7"],
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
    invoca: false,
    color: &["color4", "color13", "color12", "#e0af68"],
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
    invoca: false,
    color: &["color10", "color2", "color11", "#9ece6a"],
};

/// claudio: el bichito de Claude, que lo hizo para acordarse de cuando
/// hicieron el bicho juntos. Cuadradito y terracota (color fijo, no cambia
/// con el look), sin contorno, con bracitos, cachetes y un foquito de
/// antena que se mece al caminar.
pub const CLAUDIO: Especie = Especie {
    nombre: "claudio",
    cuerpo: &[
        "........hh......",
        ".......hhhh.....",
        "........hh......",
        "........b.......",
        "...bbbbbbbbbb...",
        "..bbbbbbbbbbbb..",
        ".bbhhbbbbbbbbbb.",
        "bbbhbbwpbbwpbbbb",
        "bbbbbbwpbbwpbbbb",
        "bbbnnbbbbbbnnbbb",
        ".bbbbbbbbbbbbbb.",
        "..bbbbbbbbbbbb..",
    ],
    cuerpo_b: Some(&[
        ".........hh.....",
        "........hhhh....",
        ".........hh.....",
        "........b.......",
        "...bbbbbbbbbb...",
        "..bbbbbbbbbbbb..",
        ".bbhhbbbbbbbbbb.",
        "bbbhbbwpbbwpbbbb",
        "bbbbbbwpbbwpbbbb",
        "bbbnnbbbbbbnnbbb",
        ".bbbbbbbbbbbbbb.",
        "..bbbbbbbbbbbb..",
    ]),
    patas_a: &["....bb....bb....", "....bb....bb...."],
    patas_b: &["....bb....bb....", "..........bb...."],
    brinca: false,
    blanda: None,
    invoca: true,
    color: &["#d97757"],
};

pub const TODAS: [&Especie; 4] = [&BICHO, &MICHI, &BABA, &CLAUDIO];

pub fn especie(nombre: &str) -> Option<&'static Especie> {
    todas().iter().copied().find(|e| e.nombre == nombre)
}

/// Todas, ya con sus disfraces: si existe `~/.config/bicho/<especie>.txt`,
/// ese dibujo reemplaza al de la especie (nomás en tu máquina; el repo no se
/// entera). Lo blandito de la baba no se disfraza.
pub fn todas() -> &'static [&'static Especie] {
    static D: std::sync::OnceLock<Vec<&'static Especie>> = std::sync::OnceLock::new();
    D.get_or_init(|| {
        let dir =
            std::env::var_os("HOME").map(|h| std::path::PathBuf::from(h).join(".config/bicho"));
        TODAS
            .iter()
            .map(|&e| {
                let texto = dir.as_ref().and_then(|d| {
                    std::fs::read_to_string(d.join(format!("{}.txt", e.nombre))).ok()
                });
                match texto.and_then(|t| disfraz(e, &t)) {
                    Some(d) => &*Box::leak(Box::new(d)),
                    None => e,
                }
            })
            .collect()
    })
}

/// Lee un disfraz:
///
/// ```text
/// // comentarios así
/// color = #d97757
/// [cuerpo]
/// ..bbbbbbbbbbbb..
/// [cuerpo_b]   (opcional: el otro cuadro al caminar)
/// [patas_a]    (opcionales)
/// [patas_b]
/// ```
///
/// Las letras son las de siempre (o contorno, b cuerpo, h brillo, w ojo,
/// p pupila, n nariz/cachete). Renglones de 16; si algo está mal, None (y se
/// queda el dibujo normal).
fn disfraz(base: &Especie, texto: &str) -> Option<Especie> {
    fn fija(v: Vec<String>) -> &'static [&'static str] {
        let v: Vec<&'static str> = v
            .into_iter()
            .map(|s| &*Box::leak(s.into_boxed_str()))
            .collect();
        Box::leak(v.into_boxed_slice())
    }
    let mut secciones: std::collections::HashMap<String, Vec<String>> = Default::default();
    let mut color = None;
    let mut actual = None;
    for l in texto.lines().map(str::trim_end) {
        let l2 = l.trim();
        if l2.is_empty() || l2.starts_with("//") {
            continue;
        }
        if let Some(c) = l2
            .strip_prefix("color")
            .map(str::trim)
            .and_then(|c| c.strip_prefix('='))
        {
            let c = c.trim();
            let ok =
                c.len() == 7 && c.starts_with('#') && c[1..].chars().all(|d| d.is_ascii_hexdigit());
            color = ok.then(|| &*Box::leak(c.to_string().into_boxed_str()));
            continue;
        }
        if let Some(s) = l2.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
            actual = Some(s.to_string());
            secciones.entry(s.to_string()).or_default();
            continue;
        }
        if l.chars().count() != ANCHO || !l.chars().all(|c| ".obhwpnal".contains(c)) {
            return None;
        }
        secciones.get_mut(actual.as_ref()?)?.push(l.to_string());
    }
    let cuerpo = secciones.remove("cuerpo").filter(|c| !c.is_empty())?;
    let patas_a = secciones.remove("patas_a").unwrap_or_default();
    let patas_b = secciones
        .remove("patas_b")
        .unwrap_or_else(|| patas_a.clone());
    let cuerpo_b = secciones
        .remove("cuerpo_b")
        .filter(|c| c.len() == cuerpo.len());
    if cuerpo.len() + patas_a.len().max(patas_b.len()) > ANCHO {
        return None;
    }
    Some(Especie {
        nombre: base.nombre,
        cuerpo: fija(cuerpo),
        cuerpo_b: cuerpo_b.map(fija),
        patas_a: fija(patas_a),
        patas_b: fija(patas_b),
        brinca: base.brinca,
        blanda: base.blanda,
        invoca: base.invoca,
        color: color.map_or(base.color, |c| Box::leak(Box::new([c])).as_slice()),
    })
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
    Trepa {
        paso: bool,
        pared_der: bool,
    },
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
    /// fase de la laptop (claudio sentado, mientras Claude trabaja)
    pub piensa: Option<f32>,
    /// 0..1 mientras le dura el "!" (susto, aviso)
    pub alerta: Option<f32>,
    /// fase del relojito (esperando a que acabe de compilar)
    pub reloj: Option<f32>,
    /// su casita arriba: es de casa (lo metiste tú a su ventana) y le pasaste
    /// el mouse encima, o acabas de meterlo
    pub casa: bool,
    /// cómo va deformado (solo los blanditos)
    pub forma: Forma,
}

/// Cierra los ojos: con ojos de dos renglones el de arriba se vuelve cuerpo
/// y el de abajo una rayita; con uno, rayita. Los ojos sin blanco (pura
/// pupila, como en algún disfraz) se hacen de un renglón.
fn cierra(filas: &mut [Vec<char>]) {
    let ojo = if filas.iter().flatten().any(|&c| c == 'w') {
        'w'
    } else {
        'p'
    };
    let con_ojo: Vec<usize> = (0..filas.len())
        .filter(|&i| filas[i].contains(&ojo))
        .collect();
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
        Pose::Camina(paso) | Pose::Trepa { paso, .. } => {
            (paso, Some(if paso { e.patas_b } else { e.patas_a }))
        }
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
    let mut filas: Vec<Vec<char>> = base
        .iter()
        .chain(patas.unwrap_or(&[]))
        .map(|f| f.chars().collect())
        .collect();
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
            if c != '.'
                && (0..ANCHO as i32).contains(&x)
                && (0..ALTO as i32).contains(&y)
                && g[y as usize][x as usize] == 0
            {
                g[y as usize][x as usize] = color;
            }
        }
    }
}

const CORAZON_DIBUJO: [&str; 4] = [".r.r.", "rrrrr", ".rrr.", "..r.."];
const Z_DIBUJO: [&str; 3] = ["zzz", ".z.", "zzz"];
const ALERTA_DIBUJO: [&str; 5] = ["r", "r", "r", ".", "r"];
/// la casita de los que son de casa
const CASA_DIBUJO: [&str; 6] = [
    "...z...", "..zzz..", ".zzzzz.", "zzzzzzz", ".zz.zz.", ".zz.zz.",
];
/// el relojito de esperar: la manecilla da la vuelta
const RELOJ_DIBUJO: [[&str; 5]; 4] = [
    [".zzz.", "z.z.z", "z.z.z", "z...z", ".zzz."],
    [".zzz.", "z...z", "z.zzz", "z...z", ".zzz."],
    [".zzz.", "z...z", "z.z.z", "z.z.z", ".zzz."],
    [".zzz.", "z...z", "zzz.z", "z...z", ".zzz."],
];
/// la luz de la pantalla de la laptop de claudio, cuadro por cuadro
const LUZ_LAPTOP: [&str; 4] = ["hhhhhhhh", "hhh.hhhh", "hhhhh.hh", "h.hhhhhh"];
const NOTA_DIBUJO: [&str; 4] = ["..xx", "..x.", "xxx.", "xx.."];

/// Primer renglón con algo del cuerpo (para poner cosas encima).
fn tope(g: &Grid) -> i32 {
    g.iter()
        .position(|f| f.iter().any(|&c| c != 0))
        .unwrap_or(AIRE) as i32
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
    let lado = |col: i32, w: i32| {
        if look.derecha {
            col
        } else {
            ANCHO as i32 - col - w
        }
    };
    if let Some(p) = look.corazon {
        let sube = (p * 4.0) as i32;
        sello(
            &mut g,
            &CORAZON_DIBUJO,
            lado(9, 5),
            (tope - 4).min(3) - sube,
            CORAZON,
        );
    }
    if let Some(t) = look.zzz {
        // dos zetas que suben y se van, desfasadas
        for (k, col) in [(0.0, 11), (0.5, 8)] {
            let f = (t * 0.6 + k) % 1.0;
            sello(
                &mut g,
                &Z_DIBUJO,
                lado(col + (f * 3.0) as i32, 3),
                tope.min(7) - (f * 8.0) as i32,
                ZZZ,
            );
        }
    }
    if let Some(t) = look.piensa {
        // su laptop, vista desde atrás: tapa oscura con su logo y la luz de
        // la pantalla en la orilla, que cambia como texto que se recorre
        let luz = LUZ_LAPTOP[(t * 5.0) as usize % LUZ_LAPTOP.len()];
        let tapa = [luz, "oooooooo", "oooohooo", "oooooooo"];
        for (dy, f) in tapa.iter().enumerate() {
            for (dx, c) in f.chars().enumerate() {
                let (x, y) = (4 + dx, ALTO - tapa.len() + dy);
                if c != '.' {
                    g[y][x] = celda(c);
                }
            }
        }
    }
    if let Some(p) = look.alerta {
        // el "!" brinca tantito al salir
        let sube = if p < 0.15 { 1 } else { 0 };
        sello(
            &mut g,
            &ALERTA_DIBUJO,
            lado(10, 1),
            (tope - 6).max(0) - sube,
            CORAZON,
        );
    }
    if let Some(t) = look.reloj {
        let cuadro = &RELOJ_DIBUJO[(t * 1.5) as usize % RELOJ_DIBUJO.len()];
        sello(&mut g, cuadro, lado(10, 5), (tope - 6).max(0), ZZZ);
    }
    if look.casa && look.alerta.is_none() && look.reloj.is_none() {
        sello(&mut g, &CASA_DIBUJO, lado(9, 7), (tope - 7).max(0), ZZZ);
    }
    if let Some(t) = look.notas {
        for (k, col) in [(0.0, 1), (0.5, 11)] {
            let f = (t * 0.5 + k) % 1.0;
            sello(
                &mut g,
                &NOTA_DIBUJO,
                col + ((f * 6.0).sin() * 1.5) as i32,
                tope.min(6) - (f * 7.0) as i32,
                ZZZ,
            );
        }
    }
    g
}

/// Las celdas del cuerpo (sin corazón ni zzz ni notas): (x0, y0, x1, y1),
/// inclusivas. Para saber dónde se le puede dar clic.
pub fn caja(g: &Grid) -> Option<(usize, usize, usize, usize)> {
    let mut c: Option<(usize, usize, usize, usize)> = None;
    for (y, f) in g.iter().enumerate() {
        for (x, &v) in f.iter().enumerate() {
            if v != 0 && v != CORAZON && v != ZZZ {
                c = Some(match c {
                    None => (x, y, x, y),
                    Some((x0, y0, x1, y1)) => (x0.min(x), y0.min(y), x1.max(x), y1.max(y)),
                });
            }
        }
    }
    c
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
            let bl = e
                .blanda
                .map(|b| [b.ancha, b.aplastada, b.plasta, b.alta, b.estirada].concat())
                .unwrap_or_default();
            let todas = e
                .cuerpo
                .iter()
                .chain(e.cuerpo_b.unwrap_or(&[]))
                .chain(e.patas_a)
                .chain(e.patas_b)
                .chain(&bl);
            for f in todas {
                assert_eq!(f.chars().count(), ANCHO, "{}: {f}", e.nombre);
            }
            assert!(e.cuerpo.len() + e.patas_a.len() <= ANCHO, "{}", e.nombre);
        }
    }

    #[test]
    fn trepando_las_patas_tocan_la_pared() {
        let look = |pared_der| Look {
            pose: Pose::Trepa {
                paso: false,
                pared_der,
            },
            derecha: true,
            parpadeo: false,
            corazon: None,
            zzz: None,
            notas: None,
            piensa: None,
            alerta: None,
            reloj: None,
            casa: false,
            forma: Forma::Normal,
        };
        for e in TODAS {
            let g = grid(e, &look(true));
            assert!(g.iter().any(|f| f[ANCHO - 1] != 0), "{}", e.nombre);
            let g = grid(e, &look(false));
            assert!(g.iter().any(|f| f[0] != 0), "{}", e.nombre);
        }
    }

    const ORIGINAL: &str = "// prueba: sin contorno y ojos de pura pupila
color = #112233
[cuerpo]
..bbbbbbbbbbbb..
..bbpbbbbbbpbb..
..bbpbbbbbbpbb..
bbbbbbbbbbbbbbbb
..bbbbbbbbbbbb..
[patas_a]
...b.b....b.b...
";

    #[test]
    fn un_disfraz_cambia_el_dibujo_y_el_color() {
        let d = disfraz(&CLAUDIO, ORIGINAL).expect("se lee");
        assert_eq!(d.cuerpo.len(), 5);
        assert_eq!(d.patas_b, d.patas_a, "sin patas_b usa las mismas");
        assert_eq!(d.color, &["#112233"]);
        assert!(d.invoca, "sigue siendo claudio");
        // parpadea aunque sus ojos sean pura pupila
        let pupilas = |parpadeo| {
            let look = Look {
                pose: Pose::Parado,
                derecha: true,
                parpadeo,
                corazon: None,
                zzz: None,
                notas: None,
                piensa: None,
                alerta: None,
                reloj: None,
                casa: false,
                forma: Forma::Normal,
            };
            grid(&d, &look)
                .iter()
                .flatten()
                .filter(|&&c| c == PUPILA)
                .count()
        };
        assert_eq!((pupilas(false), pupilas(true)), (4, 2));
    }

    #[test]
    fn un_disfraz_mal_hecho_no_cuenta() {
        assert!(
            disfraz(&CLAUDIO, "[cuerpo]\n..bbb..\n").is_none(),
            "renglón corto"
        );
        assert!(
            disfraz(&CLAUDIO, "[patas_a]\n...b.b....b.b...\n").is_none(),
            "sin cuerpo"
        );
        assert!(
            disfraz(
                &CLAUDIO,
                &format!("[cuerpo]\n{}", "bbbbbbbbbbbbbbbb\n".repeat(17))
            )
            .is_none(),
            "muy alto"
        );
    }

    #[test]
    fn dormido_cierra_los_ojos() {
        for e in TODAS {
            let look = Look {
                pose: Pose::Duerme,
                derecha: true,
                parpadeo: false,
                corazon: None,
                zzz: None,
                notas: None,
                piensa: None,
                alerta: None,
                reloj: None,
                casa: false,
                forma: Forma::Normal,
            };
            assert!(
                !grid(e, &look).iter().flatten().any(|&c| c == OJO),
                "{}",
                e.nombre
            );
        }
    }
}
