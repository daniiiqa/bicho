//! Qué hace el animalito: camina por el piso, trepa por los lados de las
//! ventanas, se pasea por arriba de ellas y se cae por la orilla. Se sienta,
//! se duerme (más de noche), baila si suena música, a veces persigue al
//! cursor, y si se lo acercas brinca con un corazón. La baba anda a brincos.
//!
//! Todo en pixeles lógicos del monitor; (x, y) es la esquina de arriba a la
//! izquierda de su lienzo (`cw × ch`), con las patas en la orilla de abajo.

use crate::arte::{Forma, Look, Pose};
use crate::hypr::{Mundo, Ventana};
use crate::texto::Repisa;

/// Por dónde puede caminar: el piso (`ventana: None`) o el techo de una
/// ventana. `x0..x1` son las x válidas del lienzo; `y` donde queda parado.
#[derive(Debug)]
struct Plataforma {
    ventana: Option<String>,
    x0: f32,
    x1: f32,
    y: f32,
}

#[derive(Clone, PartialEq, Debug)]
enum Hace {
    Camina,
    Para,
    Sienta,
    Duerme,
    /// camina hasta el lado de la ventana para treparla
    VaATrepar {
        ventana: String,
        pared_der: bool,
    },
    Trepa {
        ventana: String,
        pared_der: bool,
    },
    /// en el aire (cayendo o brincando)
    Vuela,
    Baila,
    /// va hacia el cursor
    Persigue,
    /// lo tienes agarrado con el mouse
    Agarrado,
}

pub struct Bicho {
    pub x: f32,
    pub y: f32,
    vx: f32,
    vy: f32,
    der: bool,
    /// la ventana sobre la que anda (None: el piso)
    sobre: Option<String>,
    hace: Hace,
    falta: f32,
    reloj: f32,
    parpadeo: f32,
    prox_parpadeo: f32,
    corazon: f32,
    cariño_espera: f32,
    /// dónde estaba el cursor y cuánto lleva quieto cerca
    cursor_antes: Option<(f32, f32)>,
    cursor_quieto: f32,
    rng: u64,
    /// tamaño de una celda del dibujo
    s: f32,
    pub cw: f32,
    pub ch: f32,
    pub velocidad: f32,
    pub trepa: bool,
    /// anda a brincos (la baba)
    pub salta: bool,
    /// brincando para caminar: al caer sigue con esto
    vuelve: Option<Hace>,
    /// lo que falta para el siguiente brinco
    carga: f32,
    /// segundos desde que cayó (la baba tiembla al caer)
    aterrizo: f32,
    /// suena música (playerctl)
    pub musica: bool,
    /// de noche le da más sueño
    pub noche: bool,
    cursor: Option<(f32, f32)>,
    /// cuánto sube el dibujo sin moverse de lugar (bailando)
    pub dy: f32,
    /// platicando contigo (el globo de claudio): se queda quieto
    pub platica: bool,
    /// Claude está trabajando (claudio): le salen puntitos
    pub piensa: bool,
    /// metido en una ventana (su address): ahí vive, se mueve con ella y se
    /// esconde si la ventana no se ve
    pub dentro: Option<String>,
    /// su ventana está en otro escritorio: no se pinta
    pub oculto: bool,
    /// si su ventana es una terminal: las letras donde cabe parado
    pub repisas: Vec<Repisa>,
    /// la ventana de la que anda colgado y su x (para irse con ella)
    ancla: Option<(String, f32)>,
}

const CORAZON_DURA: f32 = 1.3;
/// cuánto tiene que estar el cursor quieto cerca para que brinque de cariño
const CARIÑO_QUIETO: f32 = 0.7;
const GRAVEDAD: f32 = 1400.0;
/// cada que decide qué hacer: chance de meterse a una ventana solito
const METERSE: f32 = 0.1;
/// y de salirse si ya anda adentro
const SALIRSE: f32 = 0.06;
/// en una terminal: chance de brincar a otra línea de letras
const BRINCA_LETRAS: f32 = 0.35;
/// cayendo más rápido que esto rebota en vez de quedarse
const REBOTA: f32 = 900.0;
/// lo más rápido que se puede aventar
const AVIENTA_MAX: f32 = 2600.0;

impl Bicho {
    pub fn new(s: f32, cw: f32, ch: f32, semilla: u64) -> Bicho {
        Bicho {
            x: 0.0,
            y: 0.0,
            vx: 0.0,
            vy: 0.0,
            der: true,
            sobre: None,
            hace: Hace::Vuela,
            falta: 0.0,
            reloj: 0.0,
            parpadeo: 0.0,
            prox_parpadeo: 3.0,
            corazon: 0.0,
            cariño_espera: 0.0,
            cursor_antes: None,
            cursor_quieto: 0.0,
            rng: semilla | 1,
            s,
            cw,
            ch,
            velocidad: 1.0,
            trepa: true,
            salta: false,
            vuelve: None,
            carga: 0.0,
            aterrizo: 9.0,
            musica: false,
            noche: false,
            cursor: None,
            dy: 0.0,
            platica: false,
            piensa: false,
            dentro: None,
            oculto: false,
            repisas: Vec::new(),
            ancla: None,
        }
    }

    /// Aparece en algún lugar del piso, cayendo desde arriba.
    pub fn aparece(&mut self, ancho: f32) {
        self.x = self.azar() * (ancho - self.cw).max(0.0);
        self.y = -self.ch;
        self.der = self.azar() < 0.5;
        self.hace = Hace::Vuela;
    }

    fn azar(&mut self) -> f32 {
        // xorshift: no hace falta más
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 7;
        self.rng ^= self.rng << 17;
        (self.rng >> 40) as f32 / (1u64 << 24) as f32
    }

    fn entre(&mut self, a: f32, b: f32) -> f32 {
        a + self.azar() * (b - a)
    }

    fn paso(&self) -> f32 {
        14.0 * self.s * self.velocidad
    }

    fn plataformas(&self, m: &Mundo, ancho: f32, alto: f32) -> Vec<Plataforma> {
        // metido en una ventana: su piso es el fondo de la ventana, y nada más
        if let Some(v) = self
            .dentro
            .as_ref()
            .and_then(|id| m.ventanas.iter().find(|v| &v.id == id))
        {
            let x0 = (v.x).max(0.0);
            let x1 = (v.x + v.w - self.cw).min(ancho - self.cw).max(x0);
            let mut p = vec![Plataforma {
                ventana: None,
                x0,
                x1,
                y: v.y + v.h - self.ch,
            }];
            // las letras de la terminal (cada línea se llama por su altura:
            // si el texto se recorre, se le va el piso y cae)
            for r in &self.repisas {
                let (rx0, rx1) = ((v.x + r.x0).max(x0), (v.x + r.x1).min(x1));
                let y = v.y + r.y;
                if rx0 <= rx1 && y > v.y - self.ch && y < v.y + v.h - self.ch - 2.0 {
                    p.push(Plataforma {
                        ventana: Some(format!("{}#{}", v.id, r.y.round() as i32)),
                        x0: rx0,
                        x1: rx1,
                        y,
                    });
                }
            }
            return p;
        }
        let mut p = vec![Plataforma {
            ventana: None,
            x0: 0.0,
            x1: (ancho - self.cw).max(0.0),
            y: alto - m.abajo - self.ch,
        }];
        for v in &m.ventanas {
            let y = v.y - m.borde - self.ch;
            // con la cabeza fuera de la pantalla no; las antenas sí pueden
            if y + 4.0 * self.s < 0.0 {
                continue;
            }
            let x0 = (v.x - m.borde - self.cw / 2.0).max(0.0);
            let x1 = (v.x + v.w + m.borde - self.cw / 2.0).min(ancho - self.cw);
            // casi fuera de la pantalla no cabe (y clamp(x0, x1) truena si x0 > x1)
            if x1 < x0 {
                continue;
            }
            p.push(Plataforma {
                ventana: Some(v.id.clone()),
                x0,
                x1,
                y,
            });
        }
        p
    }

    /// La x del lienzo pegado al lado de `v` (`pared_der`: la ventana le queda
    /// a la derecha).
    fn junto_a(&self, v: &Ventana, m: &Mundo, pared_der: bool) -> f32 {
        if pared_der {
            v.x - m.borde - self.cw
        } else {
            v.x + v.w + m.borde
        }
    }

    /// Escoge qué hacer ahora que terminó lo anterior (estando parado en
    /// algo).
    fn decide(&mut self, m: &Mundo, ancho: f32, alto: f32) {
        let r = self.azar();
        let piso = alto - m.abajo - self.ch;
        if self.trepa && self.dentro.is_none() && self.sobre.is_none() && r < 0.22 {
            // una ventana que llegue al piso y con lado libre en la pantalla
            let mut opciones = Vec::new();
            for v in &m.ventanas {
                if (v.y + v.h + m.borde - (piso + self.ch)).abs() > 40.0
                    || v.y - m.borde - self.ch + 4.0 * self.s < 0.0
                {
                    continue;
                }
                for pared_der in [true, false] {
                    let x = self.junto_a(v, m, pared_der);
                    if x >= 0.0 && x <= ancho - self.cw {
                        opciones.push((v.id.clone(), pared_der));
                    }
                }
            }
            if !opciones.is_empty() {
                let (ventana, pared_der) = opciones
                    [(self.azar() * opciones.len() as f32) as usize % opciones.len()]
                .clone();
                self.hace = Hace::VaATrepar { ventana, pared_der };
                self.falta = 30.0;
                return;
            }
        }
        // a veces se mete a una ventana solito: si anda arriba de ella se
        // clava para adentro; si anda en el piso, a la que le quede enfrente
        if self.dentro.is_none() && self.azar() < METERSE {
            let enfrente = || {
                let centro = self.x + self.cw / 2.0;
                m.ventanas
                    .iter()
                    .find(|v| {
                        centro > v.x
                            && centro < v.x + v.w
                            && (v.y + v.h - (piso + self.ch)).abs() < 40.0
                            && v.h > self.ch
                    })
                    .map(|v| v.id.clone())
            };
            if let Some(id) = self.sobre.clone().or_else(enfrente) {
                self.dentro = Some(id);
                self.sobre = None;
                self.brinquito();
                return;
            }
        }
        // en una terminal: brinca a otra línea de letras que le quede cerca
        if self.dentro.is_some() && self.azar() < BRINCA_LETRAS {
            let plats = self.plataformas(m, ancho, alto);
            let cerca: Vec<&Plataforma> = plats
                .iter()
                .filter(|p| {
                    p.ventana != self.sobre
                        && (p.y - self.y).abs() <= 170.0
                        && (p.y - self.y).abs() > 4.0
                })
                .filter(|p| (self.x.clamp(p.x0, p.x1) - self.x).abs() <= 280.0)
                .collect();
            if !cerca.is_empty() {
                let p = cerca[(self.azar() * cerca.len() as f32) as usize % cerca.len()];
                // que caiga tantito adentro de la repisa, no en la orilla
                let adentro = ((p.x1 - p.x0) * 0.15).min(20.0);
                let tx = (self.x + self.entre(-60.0, 60.0)).clamp(p.x0 + adentro, p.x1 - adentro);
                self.salta_a(tx, p.y);
                return;
            }
        }
        // adentro: de vez en cuando se sale por abajo
        if self.dentro.is_some() && self.azar() < SALIRSE {
            self.dentro = None;
            self.brinquito();
            return;
        }
        if self.musica && self.azar() < 0.6 {
            (self.hace, self.falta) = (Hace::Baila, self.entre(4.0, 10.0));
            return;
        }
        if let Some((cx, _)) = self.cursor
            && cx >= 0.0
            && cx <= ancho
            && self.azar() < 0.1
        {
            self.der = cx > self.x + self.cw / 2.0;
            (self.hace, self.falta) = (Hace::Persigue, self.entre(3.0, 6.0));
            return;
        }
        // de noche: más sentado y dormido
        let r = if self.noche {
            self.azar() * 0.7 + 0.3
        } else {
            self.azar()
        };
        (self.hace, self.falta) = if r < 0.5 {
            if self.azar() < 0.35 {
                self.der = !self.der;
            }
            (Hace::Camina, self.entre(2.0, 7.0))
        } else if r < 0.72 {
            (Hace::Para, self.entre(1.0, 3.5))
        } else if r < 0.86 {
            (Hace::Sienta, self.entre(3.0, 8.0))
        } else if r < 0.96 {
            (Hace::Duerme, self.entre(12.0, 30.0))
        } else {
            self.vy = -380.0;
            self.vx = 0.0;
            (Hace::Vuela, 0.0)
        };
    }

    pub fn tick(
        &mut self,
        dt: f32,
        m: &Mundo,
        ancho: f32,
        alto: f32,
        cursor: Option<(f32, f32)>,
    ) -> Look {
        let dt = dt.min(0.1);
        self.reloj += dt;
        self.aterrizo += dt;
        self.falta -= dt;
        self.corazon = (self.corazon - dt).max(0.0);
        self.cariño_espera = (self.cariño_espera - dt).max(0.0);
        self.parpadeo = (self.parpadeo - dt).max(0.0);
        self.prox_parpadeo -= dt;
        if self.prox_parpadeo <= 0.0 {
            self.parpadeo = 0.14;
            self.prox_parpadeo = self.entre(2.0, 6.0);
        }
        self.cursor = cursor;
        self.dy = 0.0;
        // metido en una ventana que no se ve: o está en otro escritorio (se
        // esconde con ella) o ya la cerraron (se sale y cae)
        if let Some(id) = self.dentro.clone() {
            self.oculto = !m.ventanas.iter().any(|v| v.id == id);
            if self.oculto {
                if m.todas.is_empty() || m.todas.contains(&id) {
                    return self.look();
                }
                self.dentro = None;
                self.oculto = false;
                self.cae();
            }
        }
        // la ventana donde anda (adentro o encima) se movió: él también
        let suya = self.dentro.clone().or_else(|| {
            self.sobre
                .as_ref()
                .map(|s| s.split('#').next().unwrap_or(s).to_string())
        });
        let donde = suya.and_then(|id| m.ventanas.iter().find(|v| v.id == id).map(|v| (id, v.x)));
        if let (Some((id, x)), Some((ida, xa))) = (&donde, &self.ancla)
            && id == ida
            && !matches!(self.hace, Hace::Agarrado)
        {
            self.x += x - xa;
        }
        self.ancla = donde;
        let plats = self.plataformas(m, ancho, alto);
        let paso = self.paso();

        // ¿lo que pisa sigue ahí? (la ventana se cerró, se movió o cambiaste
        // de escritorio)
        if !matches!(self.hace, Hace::Vuela | Hace::Trepa { .. } | Hace::Agarrado) {
            match plats.iter().find(|p| p.ventana == self.sobre) {
                Some(p) if self.x >= p.x0 - 2.0 && self.x <= p.x1 + 2.0 => self.y = p.y,
                _ => self.cae(),
            }
        }

        // cariño: el cursor quieto cerca lo despierta y brinca (si nomás va
        // pasando, o vienes a agarrarlo, no: que no se te escape)
        if let Some((cx, cy)) = cursor {
            let (bx, by) = (self.x + self.cw / 2.0, self.y + self.ch * 0.65);
            let cerca = (cx - bx).hypot(cy - by) < self.cw * 1.4;
            let quieto = self
                .cursor_antes
                .is_some_and(|(ax, ay)| (cx - ax).hypot(cy - ay) < 6.0);
            self.cursor_quieto = if cerca && quieto {
                self.cursor_quieto + dt
            } else {
                0.0
            };
            self.cursor_antes = Some((cx, cy));
            if self.cursor_quieto > CARIÑO_QUIETO
                && !self.platica
                && self.cariño_espera <= 0.0
                && !matches!(self.hace, Hace::Vuela | Hace::Trepa { .. } | Hace::Agarrado)
            {
                self.cariño(cx > bx);
                if self.hace == Hace::Persigue {
                    self.falta = 0.0;
                }
            }
        }

        match self.hace.clone() {
            // lo mueve el mouse (arrastra); aquí nomás espera
            Hace::Agarrado => {}
            Hace::Camina => {
                self.anda(dt, &plats);
                if self.falta <= 0.0 && self.hace == Hace::Camina {
                    self.decide(m, ancho, alto);
                }
            }
            Hace::Persigue => {
                let meta = cursor.map(|(cx, _)| cx - self.cw / 2.0);
                match meta {
                    Some(meta) if (meta - self.x).abs() > self.cw / 3.0 && self.falta > 0.0 => {
                        self.der = meta > self.x;
                        self.anda(dt, &plats);
                    }
                    _ => {
                        self.hace = Hace::Para;
                        self.falta = self.entre(0.8, 2.0);
                    }
                }
            }
            Hace::Baila => {
                // de un lado al otro, al brinquito
                let t = self.reloj * 2.2;
                self.der = (t as u32).is_multiple_of(2);
                self.dy = (t.fract() * std::f32::consts::PI).sin() * 3.0 * self.s;
                if self.falta <= 0.0 || !self.musica {
                    self.decide(m, ancho, alto);
                }
            }
            Hace::Para | Hace::Sienta if self.platica => {}
            Hace::Para | Hace::Sienta => {
                if self.falta <= 0.0 || (self.musica && self.azar() < dt * 0.3) {
                    self.decide(m, ancho, alto);
                }
            }
            Hace::Duerme => {
                if self.falta <= 0.0 {
                    self.decide(m, ancho, alto);
                }
            }
            Hace::VaATrepar { ventana, pared_der } => {
                let Some(v) = m.ventanas.iter().find(|v| v.id == ventana) else {
                    self.hace = Hace::Camina;
                    self.falta = 1.0;
                    return self.look();
                };
                let meta = self.junto_a(v, m, pared_der);
                if (meta - self.x).abs() <= paso * dt || self.falta <= 0.0 {
                    self.x = meta;
                    self.hace = Hace::Trepa { ventana, pared_der };
                } else {
                    self.der = meta > self.x;
                    self.x += if self.der { paso } else { -paso } * dt;
                }
            }
            Hace::Trepa { ventana, pared_der } => {
                let Some(v) = m.ventanas.iter().find(|v| v.id == ventana) else {
                    self.cae();
                    return self.look();
                };
                if (self.junto_a(v, m, pared_der) - self.x).abs() > 4.0 {
                    self.cae();
                    return self.look();
                }
                self.y -= paso * 0.7 * dt;
                // la cabeza ya llegó a la orilla de arriba: se sube
                let techo = v.y - m.borde;
                if self.y + 4.0 * self.s <= techo {
                    if let Some(p) = plats
                        .iter()
                        .find(|p| p.ventana.as_deref() == Some(ventana.as_str()))
                    {
                        self.x = if pared_der { p.x0 } else { p.x1 };
                        self.y = p.y;
                        self.sobre = p.ventana.clone();
                        self.der = pared_der;
                        self.hace = Hace::Camina;
                        self.falta = self.entre(2.0, 6.0);
                    } else {
                        self.cae();
                    }
                }
            }
            Hace::Vuela => {
                let antes = self.y;
                self.vy += GRAVEDAD * dt;
                self.y += self.vy * dt;
                // las orillas de la pantalla: rebota (si lo aventaste)
                let tope_x = (ancho - self.cw).max(0.0);
                self.x += self.vx * dt;
                if self.dentro.is_some() {
                    // adentro no se sale por los lados
                    self.x = self.x.clamp(plats[0].x0, plats[0].x1);
                }
                if self.x < 0.0 || self.x > tope_x {
                    self.x = self.x.clamp(0.0, tope_x);
                    self.vx = -self.vx * 0.5;
                    self.der = self.vx > 0.0;
                }
                if self.y < -self.ch * 0.3 && self.vy < 0.0 {
                    self.y = -self.ch * 0.3;
                    self.vy = -self.vy * 0.3;
                }
                if self.vy > 0.0 {
                    // cae en lo más alto que cruzó
                    let aterriza = plats
                        .iter()
                        .filter(|p| {
                            antes <= p.y + 0.5 && self.y >= p.y && self.x >= p.x0 && self.x <= p.x1
                        })
                        .min_by(|a, b| a.y.total_cmp(&b.y));
                    let piso = &plats[0];
                    let aterriza = aterriza.or((self.y >= piso.y).then_some(piso));
                    if let Some(p) = aterriza
                        && self.vy > REBOTA
                    {
                        // cayó fuerte (lo aventaste): rebota y sigue volando
                        self.y = p.y;
                        self.aterrizo = 0.0;
                        self.vy = -self.vy * 0.35;
                        self.vx *= 0.6;
                    } else if let Some(p) = aterriza {
                        self.y = p.y;
                        self.sobre = p.ventana.clone();
                        // qué tan fuerte cayó: más alto, más se embarra
                        self.aterrizo = if self.vy > 500.0 { 0.0 } else { 0.07 };
                        self.vx = 0.0;
                        self.vy = 0.0;
                        match self.vuelve.take() {
                            Some(h) if self.falta > 0.0 => {
                                self.hace = h;
                                // que alcance a temblar antes del siguiente
                                self.carga = 0.55;
                            }
                            _ => {
                                self.hace = Hace::Para;
                                self.falta = self.entre(0.4, 1.2);
                            }
                        }
                    }
                }
            }
        }
        self.look()
    }

    /// Un paso hacia donde mira; en la orilla del techo de una ventana se
    /// avienta (o se da la vuelta), en la del piso se da la vuelta. La baba
    /// en vez de caminar se aplasta y brinca.
    fn anda(&mut self, dt: f32, plats: &[Plataforma]) {
        let paso = self.paso();
        let Some(p) = plats.iter().find(|p| p.ventana == self.sobre) else {
            return;
        };
        if self.salta {
            self.carga -= dt;
            if self.carga > 0.0 {
                return;
            }
            let lejos = if self.der {
                self.x + paso * 0.5 > p.x1
            } else {
                self.x - paso * 0.5 < p.x0
            };
            if lejos && (self.sobre.is_none() || self.azar() < 0.4) {
                self.der = !self.der;
            }
            self.carga = 0.3;
            self.vx = if self.der { paso * 1.6 } else { -paso * 1.6 };
            self.vy = -300.0;
            self.vuelve = Some(self.hace.clone());
            self.hace = Hace::Vuela;
            return;
        }
        self.x += if self.der { paso } else { -paso } * dt;
        if self.x < p.x0 || self.x > p.x1 {
            self.x = self.x.clamp(p.x0, p.x1);
            if self.sobre.is_some() && self.azar() < 0.6 {
                // se avienta
                self.vx = if self.der { paso } else { -paso };
                self.vy = -120.0;
                self.hace = Hace::Vuela;
                self.sobre = None;
            } else {
                self.der = !self.der;
            }
        }
    }

    /// Brinca contento con un corazón, mirando hacia `der`.
    pub fn cariño(&mut self, der: bool) {
        self.der = der;
        self.corazon = CORAZON_DURA;
        self.cariño_espera = 4.0;
        self.vy = -320.0;
        self.vx = 0.0;
        self.vuelve = None;
        self.hace = Hace::Vuela;
    }

    /// Lo agarras con el mouse: se queda colgando donde lo tengas.
    pub fn agarra(&mut self) {
        self.dentro = None;
        self.oculto = false;
        self.hace = Hace::Agarrado;
        self.sobre = None;
        self.vuelve = None;
        self.vx = 0.0;
        self.vy = 0.0;
    }

    /// Un brinco que cae en (tx, ty): sube un poco más alto que eso y cae
    /// justo ahí.
    fn salta_a(&mut self, tx: f32, ty: f32) {
        let sube = (self.y - ty).max(0.0) + 24.0;
        let vy = -(2.0 * GRAVEDAD * sube).sqrt();
        let cae = ty - (self.y - sube);
        let t = -vy / GRAVEDAD + (2.0 * cae.max(1.0) / GRAVEDAD).sqrt();
        self.vy = vy;
        self.vx = (tx - self.x) / t;
        if self.vx.abs() > 1.0 {
            self.der = self.vx > 0.0;
        }
        self.sobre = None;
        self.vuelve = None;
        (self.hace, self.falta) = (Hace::Vuela, 0.0);
    }

    fn brinquito(&mut self) {
        self.vy = -260.0;
        self.vx = 0.0;
        self.vuelve = None;
        (self.hace, self.falta) = (Hace::Vuela, 0.0);
    }

    /// ¿Va en el aire o agarrado? (se tiene que ver fluido)
    pub fn en_el_aire(&self) -> bool {
        matches!(self.hace, Hace::Vuela | Hace::Agarrado)
    }

    /// ¿Anda en una ventana (adentro o encima)? Entonces hay que ver las
    /// ventanas seguido, para irse con ella si la mueves.
    pub fn pegado(&self) -> bool {
        self.dentro.is_some() || self.sobre.is_some() || matches!(self.hace, Hace::Trepa { .. })
    }

    /// Lo dejaste encima de una ventana: se mete y ahí se queda (cae hasta
    /// el fondo de la ventana).
    pub fn entra(&mut self, ventana: String) {
        self.dentro = Some(ventana);
        self.corazon = CORAZON_DURA;
    }

    /// Arrastrándolo: su esquina de arriba a la izquierda va a (x, y). Mira
    /// hacia donde lo jalas.
    pub fn arrastra(&mut self, x: f32, y: f32) {
        if (x - self.x).abs() > 1.0 {
            self.der = x > self.x;
        }
        self.x = x;
        self.y = y;
    }

    /// Lo sueltas con esa velocidad (pixeles por segundo): sale volando.
    pub fn avienta(&mut self, vx: f32, vy: f32) {
        let v = vx.hypot(vy);
        let k = if v > AVIENTA_MAX {
            AVIENTA_MAX / v
        } else {
            1.0
        };
        self.vx = vx * k;
        self.vy = vy * k;
        if self.vx.abs() > 30.0 {
            self.der = self.vx > 0.0;
        }
        self.sobre = None;
        self.vuelve = None;
        self.hace = Hace::Vuela;
        self.cariño_espera = 2.0;
    }

    /// Anda escondido en una ventana de otro escritorio y hay que avisarte
    /// algo: se sale y cae desde arriba donde estás tú.
    pub fn sal_a_avisar(&mut self, ancho: f32) {
        self.dentro = None;
        self.oculto = false;
        self.ancla = None;
        self.sobre = None;
        self.vuelve = None;
        self.vx = 0.0;
        self.vy = 0.0;
        self.aparece(ancho);
    }

    /// Claude acabó: brinca contento con su corazón (si está libre).
    pub fn celebra(&mut self) {
        if !self.oculto && !matches!(self.hace, Hace::Vuela | Hace::Trepa { .. } | Hace::Agarrado) {
            let der = self.der;
            self.cariño(der);
        }
    }

    /// Se pone a platicar: suelta un corazón y se queda quieto (si va en el
    /// aire, al caer) hasta que `platica` vuelva a ser false.
    pub fn platica(&mut self, der: bool) {
        self.platica = true;
        self.der = der;
        self.corazon = CORAZON_DURA;
        self.falta = 1.0;
        if self.hace == Hace::Vuela {
            self.vuelve = Some(Hace::Para);
        } else if self.hace != Hace::Agarrado {
            self.hace = Hace::Para;
        }
    }

    /// Para saludarse con otro: anda suelto y no acaba de saludar.
    pub fn saludable(&self) -> Option<(f32, Option<&str>)> {
        let libre = matches!(
            self.hace,
            Hace::Camina | Hace::Para | Hace::Persigue | Hace::Sienta | Hace::Baila
        );
        (libre && !self.platica && self.cariño_espera <= 0.0)
            .then_some((self.x, self.sobre.as_deref()))
    }

    fn cae(&mut self) {
        self.sobre = None;
        self.vuelve = None;
        self.vx = 0.0;
        self.vy = 0.0;
        self.hace = Hace::Vuela;
    }

    /// Lo blandito de la baba (squash & stretch): se aplasta para agarrar
    /// vuelo, se estira en el aire, se embarra al caer y tiembla como
    /// gelatina; quieta tiembla poquito, dormida respira.
    fn forma(&self) -> Forma {
        if !self.salta {
            return Forma::Normal;
        }
        match &self.hace {
            // colgando de tu mouse: estirada
            Hace::Agarrado => Forma::Estirada,
            // rebotó en el piso: se embarra un instante
            Hace::Vuela if self.aterrizo < 0.06 => Forma::Plasta,
            Hace::Vuela => match self.vy.abs() {
                v if v > 160.0 => Forma::Estirada,
                v if v > 70.0 => Forma::Alta,
                _ => Forma::Normal,
            },
            // como gusanito: se estira y se encoge
            Hace::Trepa { .. } => {
                if (self.reloj / 0.22) as u32 % 2 == 1 {
                    Forma::Alta
                } else {
                    Forma::Ancha
                }
            }
            Hace::Baila => {
                if (self.reloj * 2.2).fract() < 0.5 {
                    Forma::Ancha
                } else {
                    Forma::Alta
                }
            }
            h => {
                let carga = matches!(h, Hace::Camina | Hace::Persigue);
                if carga && self.carga < 0.08 {
                    return Forma::Plasta;
                }
                if carga && self.carga < 0.18 {
                    return Forma::Aplastada;
                }
                // al caer: plasta → aplastada → rebota alta → normal → ancha
                match self.aterrizo {
                    t if t < 0.07 => Forma::Plasta,
                    t if t < 0.14 => Forma::Aplastada,
                    t if t < 0.23 => Forma::Alta,
                    t if t < 0.31 => Forma::Normal,
                    t if t < 0.40 => Forma::Ancha,
                    _ => {
                        let (vel, umbral) = if matches!(h, Hace::Duerme | Hace::Sienta) {
                            (2.2, 0.1)
                        } else {
                            (5.0, 0.75)
                        };
                        if (self.reloj * vel).sin() > umbral {
                            Forma::Ancha
                        } else {
                            Forma::Normal
                        }
                    }
                }
            }
        }
    }

    fn look(&self) -> Look {
        let paso = (self.reloj / 0.14) as u32 % 2 == 1;
        let pose = match &self.hace {
            Hace::Camina | Hace::VaATrepar { .. } | Hace::Persigue => Pose::Camina(paso),
            Hace::Baila => Pose::Camina((self.reloj * 4.4) as u32 % 2 == 1),
            Hace::Para => Pose::Parado,
            Hace::Sienta => Pose::Sentado,
            Hace::Duerme => Pose::Duerme,
            Hace::Trepa { pared_der, .. } => Pose::Trepa {
                paso: (self.reloj / 0.2) as u32 % 2 == 1,
                pared_der: *pared_der,
            },
            Hace::Vuela | Hace::Agarrado => Pose::Brinca,
        };
        Look {
            pose,
            forma: self.forma(),
            derecha: self.der,
            parpadeo: self.parpadeo > 0.0,
            corazon: (self.corazon > 0.0).then(|| 1.0 - self.corazon / CORAZON_DURA),
            zzz: (self.hace == Hace::Duerme).then_some(self.reloj),
            notas: (self.hace == Hace::Baila).then_some(self.reloj),
            piensa: self.piensa.then_some(self.reloj),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mundo(ventanas: Vec<Ventana>) -> Mundo {
        Mundo {
            ventanas,
            abajo: 0.0,
            borde: 1.0,
            origen: (0.0, 0.0),
            todas: vec![],
            clis: vec![],
            pids: vec![],
        }
    }

    #[test]
    fn cae_al_piso() {
        let mut b = Bicho::new(3.0, 48.0, 60.0, 42);
        b.aparece(1920.0);
        let m = mundo(vec![]);
        for _ in 0..200 {
            b.tick(0.033, &m, 1920.0, 1200.0, None);
        }
        assert_eq!(b.y, 1200.0 - 60.0);
        assert!(b.x >= 0.0 && b.x <= 1920.0 - 48.0);
    }

    #[test]
    fn se_sube_a_las_ventanas_y_no_se_sale() {
        let v = Ventana {
            id: "a".into(),
            x: 600.0,
            y: 300.0,
            w: 600.0,
            h: 899.0,
        };
        let m = mundo(vec![v]);
        let mut b = Bicho::new(3.0, 48.0, 60.0, 7);
        b.aparece(1920.0);
        let mut arriba = false;
        for _ in 0..60_000 {
            b.tick(0.033, &m, 1920.0, 1200.0, None);
            assert!(b.x >= 0.0 && b.x <= 1920.0 - 48.0, "x = {}", b.x);
            assert!(b.y <= 1200.0 - 60.0 + 0.01, "y = {}", b.y);
            arriba |= b.sobre.is_some();
        }
        assert!(arriba, "nunca trepó");
    }

    #[test]
    fn si_la_ventana_se_va_se_cae() {
        let v = Ventana {
            id: "a".into(),
            x: 0.0,
            y: 300.0,
            w: 1920.0,
            h: 899.0,
        };
        let mut b = Bicho::new(3.0, 48.0, 60.0, 9);
        b.x = 500.0;
        b.y = -60.0;
        for _ in 0..100 {
            b.tick(0.033, &mundo(vec![v.clone()]), 1920.0, 1200.0, None);
        }
        assert_eq!(b.sobre.as_deref(), Some("a"));
        for _ in 0..200 {
            b.tick(0.033, &mundo(vec![]), 1920.0, 1200.0, None);
        }
        assert_eq!(b.sobre, None);
        assert_eq!(b.y, 1200.0 - 60.0);
    }

    #[test]
    fn la_baba_anda_a_brincos() {
        let mut b = Bicho::new(3.0, 48.0, 60.0, 5);
        b.salta = true;
        b.x = 900.0;
        b.y = 1140.0;
        b.hace = Hace::Camina;
        b.falta = 5.0;
        let m = mundo(vec![]);
        let mut aire = 0;
        for _ in 0..60 {
            b.tick(0.033, &m, 1920.0, 1200.0, None);
            aire += usize::from(b.hace == Hace::Vuela);
        }
        assert!((b.x - 900.0).abs() > 20.0, "no se movió");
        assert!(aire > 10, "no brincó");
    }

    #[test]
    fn agarrado_no_se_cae_y_aventado_vuela_rebota_y_cae() {
        let mut b = Bicho::new(3.0, 48.0, 60.0, 9);
        b.salta = true;
        b.x = 900.0;
        b.y = 1140.0;
        b.hace = Hace::Camina;
        b.falta = 5.0;
        let m = mundo(vec![]);
        b.agarra();
        b.arrastra(500.0, 300.0);
        for _ in 0..30 {
            let look = b.tick(0.033, &m, 1920.0, 1200.0, None);
            assert_eq!(look.forma, Forma::Estirada);
        }
        assert_eq!((b.x, b.y), (500.0, 300.0), "agarrado no se mueve solo");
        // aventón fuerte a la izquierda y para abajo: pega en la orilla y rebota
        b.avienta(-9000.0, 1500.0);
        let mut reboto_piso = false;
        let mut minx = f32::MAX;
        for _ in 0..300 {
            b.tick(0.016, &m, 1920.0, 1200.0, None);
            minx = minx.min(b.x);
            reboto_piso |= b.hace == Hace::Vuela && b.vy < 0.0 && b.y > 1000.0;
        }
        assert!(minx >= 0.0, "se salió de la pantalla");
        assert!(reboto_piso, "no rebotó en el piso");
        assert_ne!(b.hace, Hace::Vuela, "nunca aterrizó");
        assert_eq!(b.y, 1140.0);
    }

    #[test]
    fn metido_en_una_ventana_vive_ahi_se_esconde_y_si_la_cierran_cae() {
        let v = |x: f32| Ventana {
            id: "a".into(),
            x,
            y: 200.0,
            w: 400.0,
            h: 300.0,
        };
        let mut m = mundo(vec![v(300.0)]);
        m.todas = vec!["a".into()];
        let mut b = Bicho::new(3.0, 48.0, 60.0, 21);
        b.salta = true;
        b.x = 400.0;
        b.y = 250.0;
        b.agarra();
        b.avienta(0.0, 0.0);
        b.entra("a".into());
        // (solito se sale de vez en cuando: aquí lo volvemos a meter)
        for _ in 0..2000 {
            b.dentro = Some("a".into());
            b.tick(0.033, &m, 1920.0, 1200.0, None);
            if b.dentro.is_some() {
                assert!(
                    b.x >= 300.0 && b.x <= 300.0 + 400.0 - 48.0,
                    "se salió por un lado: {}",
                    b.x
                );
                assert!(
                    b.y <= 200.0 + 300.0 - 60.0 + 0.5,
                    "se salió por abajo: {}",
                    b.y
                );
            }
        }
        // mueves la ventana: se va con ella
        m.ventanas = vec![v(900.0)];
        for _ in 0..60 {
            b.dentro = Some("a".into());
            b.tick(0.033, &m, 1920.0, 1200.0, None);
        }
        b.dentro = Some("a".into());
        assert!(b.x >= 900.0, "no se fue con la ventana: {}", b.x);
        // otro escritorio: se esconde, sin moverse
        m.ventanas.clear();
        let antes = (b.x, b.y);
        for _ in 0..60 {
            b.tick(0.033, &m, 1920.0, 1200.0, None);
        }
        assert!(b.oculto);
        assert_eq!((b.x, b.y), antes);
        // la cierran: sale y cae al piso
        m.todas.clear();
        m.todas.push("otra".into());
        for _ in 0..200 {
            b.tick(0.033, &m, 1920.0, 1200.0, None);
        }
        assert!(!b.oculto && b.dentro.is_none());
        assert_eq!(b.y, 1200.0 - 60.0);
    }

    #[test]
    fn solitos_se_meten_a_las_ventanas_y_se_salen() {
        let m = {
            let mut m = mundo(vec![Ventana {
                id: "a".into(),
                x: 0.0,
                y: 600.0,
                w: 1920.0,
                h: 600.0,
            }]);
            m.todas = vec!["a".into()];
            m
        };
        let mut entro = false;
        let mut salio = false;
        for semilla in 1..6 {
            let mut b = Bicho::new(3.0, 48.0, 60.0, semilla);
            b.trepa = false;
            b.aparece(1920.0);
            for _ in 0..20000 {
                b.tick(0.033, &m, 1920.0, 1200.0, None);
                if b.dentro.is_some() {
                    entro = true;
                } else if entro {
                    salio = true;
                }
            }
        }
        assert!(entro, "nunca se metió");
        assert!(salio, "nunca se salió");
    }

    #[test]
    fn si_mueves_su_ventana_se_va_con_ella() {
        let v = |x: f32| Ventana {
            id: "a".into(),
            x,
            y: 200.0,
            w: 400.0,
            h: 300.0,
        };
        let mut m = mundo(vec![v(300.0)]);
        m.todas = vec!["a".into()];
        let mut b = Bicho::new(3.0, 48.0, 60.0, 4);
        b.x = 450.0;
        b.y = 440.0;
        b.entra("a".into());
        b.hace = Hace::Duerme;
        b.falta = 100.0;
        for _ in 0..10 {
            b.tick(0.033, &m, 1920.0, 1200.0, None);
        }
        let antes = b.x - 300.0;
        // la arrastras de a poquito, como con el mouse
        for k in 1..=60 {
            m.ventanas = vec![v(300.0 + k as f32 * 10.0)];
            b.tick(0.033, &m, 1920.0, 1200.0, None);
            assert_eq!(b.dentro.as_deref(), Some("a"));
        }
        assert!(
            (b.x - 900.0 - antes).abs() < 1.0,
            "se quedó atrás: {} vs {}",
            b.x - 900.0,
            antes
        );
        assert_eq!(b.hace, Hace::Duerme, "ni se despertó");
    }

    #[test]
    fn si_vas_a_agarrarlo_no_brinca_pero_si_lo_acaricias_si() {
        let m = mundo(vec![]);
        let nuevo = || {
            let mut b = Bicho::new(3.0, 48.0, 60.0, 8);
            (b.x, b.y, b.hace, b.falta) = (900.0, 1140.0, Hace::Sienta, 100.0);
            b
        };
        // el cursor llega directo hasta él: no brinca
        let mut b = nuevo();
        for k in 0..20 {
            let c = (500.0 + k as f32 * 22.0, 1170.0);
            b.tick(0.033, &m, 1920.0, 1200.0, Some(c));
            assert_ne!(b.hace, Hace::Vuela, "brincó cuando ibas a agarrarlo");
        }
        // se queda quieto cerca un ratito: brinca con su corazón
        let mut b = nuevo();
        let mut brinco = false;
        for _ in 0..40 {
            b.tick(0.033, &m, 1920.0, 1200.0, Some((930.0, 1170.0)));
            brinco |= b.hace == Hace::Vuela;
        }
        assert!(brinco, "no le dio cariño");
    }

    #[test]
    fn con_musica_baila() {
        let mut b = Bicho::new(3.0, 48.0, 60.0, 11);
        b.musica = true;
        b.aparece(1920.0);
        let m = mundo(vec![]);
        let mut bailo = false;
        for _ in 0..3000 {
            b.tick(0.033, &m, 1920.0, 1200.0, None);
            bailo |= b.hace == Hace::Baila;
        }
        assert!(bailo);
    }

    #[test]
    fn ventanas_en_la_orilla_no_lo_tumban() {
        // una casi fuera de la pantalla, una angostita, una con x negativa
        let vs = vec![
            Ventana {
                id: "a".into(),
                x: 1900.0,
                y: 300.0,
                w: 300.0,
                h: 899.0,
            },
            Ventana {
                id: "b".into(),
                x: 500.0,
                y: 400.0,
                w: 4.0,
                h: 799.0,
            },
            Ventana {
                id: "c".into(),
                x: -400.0,
                y: 200.0,
                w: 380.0,
                h: 999.0,
            },
        ];
        let m = mundo(vs);
        for semilla in 1..40 {
            let mut b = Bicho::new(3.0, 48.0, 60.0, semilla);
            b.aparece(1920.0);
            for _ in 0..3000 {
                b.tick(0.033, &m, 1920.0, 1200.0, Some((1919.0, 1199.0)));
            }
        }
    }

    #[test]
    fn la_baba_se_aplasta_se_estira_y_tiembla() {
        let mut b = Bicho::new(3.0, 48.0, 60.0, 3);
        b.salta = true;
        b.x = 900.0;
        b.y = 1140.0;
        b.hace = Hace::Camina;
        b.falta = 20.0;
        let m = mundo(vec![]);
        let mut vistas = std::collections::HashSet::new();
        for _ in 0..400 {
            vistas.insert(format!(
                "{:?}",
                b.tick(0.016, &m, 1920.0, 1200.0, None).forma
            ));
        }
        for f in ["Plasta", "Aplastada", "Estirada", "Alta", "Normal", "Ancha"] {
            assert!(vistas.contains(f), "nunca hizo {f}: {vistas:?}");
        }
    }

    #[test]
    fn los_duros_no_se_deforman() {
        let mut b = Bicho::new(3.0, 48.0, 60.0, 3);
        b.aparece(1920.0);
        let m = mundo(vec![]);
        for _ in 0..2000 {
            assert_eq!(b.tick(0.033, &m, 1920.0, 1200.0, None).forma, Forma::Normal);
        }
    }
}
