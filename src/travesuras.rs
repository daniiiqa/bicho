//! Caga palos en Linux ("mischief" en la config, o en el menú): las mismas
//! travesuras que en Windows (win/travesuras.rs), por el socket de Hyprland.
//!
//! - Se te escapan: le llegas rápido con el cursor y, a veces, brinca para
//!   el otro lado con un "!".
//! - Se roban el cursor: si uno que te anda persiguiendo te alcanza, se lo
//!   lleva corriendo un ratito. Si jalas fuerte, lo suelta (y se asusta).
//!   En Wayland no se sabe si traes un botón apretado: nomás si el cursor
//!   llevaba un rato quieto (no te lo quita mientras arrastras algo).
//! - Pisotones: uno encima de una ventana flotante brinca tres veces y con
//!   cada pisotón la ventana se hunde tantito (las de mosaico no se mueven).
//!
//! En KDE nomás se escapan (KWin no deja mover el cursor ni las ventanas).

use super::*;

/// Cada cuánto, más o menos, hacen una travesura grande (s).
const ENTRE: (f32, f32) = (70.0, 200.0);
/// Lo que dura el robo del cursor (s).
const ROBO: f32 = 1.6;
/// Si el cursor quedó más lejos que esto de donde lo dejó (px), jalaste.
const JALON: f32 = 30.0;
/// Para robárselo, el cursor tenía que estar quieto tanto (s).
const QUIETO: f32 = 0.4;
/// Qué tan cerca (px) y qué tan rápido (px/s) tiene que venir el cursor
/// para que se escape.
const ESCAPA_CERCA: f32 = 90.0;
const ESCAPA_RAPIDO: f32 = 700.0;
/// Después de escaparse, cuánto tarda en volver a hacerlo (s).
const ESCAPA_OTRA: f32 = 15.0;
/// Lo que se hunde la ventana con un pisotón (px) y lo que tarda en
/// regresar (s).
const PISOTON: i32 = 9;
const REGRESA: f32 = 0.08;

pub(crate) enum Travesura {
    Roba {
        i: usize,
        t: f32,
        puso: Option<(f32, f32)>,
    },
    Pisa {
        i: usize,
        ventana: String,
        t: f32,
        cayo: u32,
        /// la ventana va hundida: lo que falta para regresarla
        hundida: Option<f32>,
    },
}

impl App {
    fn azar_t(&mut self) -> f32 {
        let mut x = self.semilla | 1;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.semilla = x;
        (x >> 40) as f32 / (1u64 << 24) as f32
    }

    pub(crate) fn tick_travesuras(&mut self, dt: f32) {
        if !self.cfg.travieso || self.arrastre.is_some() || self.dialogo.is_some() {
            if let Some(t) = self.travesura.take() {
                self.acaba_travesura(t);
            }
            return;
        }
        // el cursor quieto (para saber si lo puede robar)
        match (self.cursor, self.cursor_antes_t) {
            (Some(c), Some(a)) if (c.0 - a.0).hypot(c.1 - a.1) < 1.0 => self.cursor_quieto += dt,
            _ => self.cursor_quieto = 0.0,
        }
        self.escapan(dt);
        self.cursor_antes_t = self.cursor;
        match self.travesura.take() {
            Some(t) => {
                if let Some(t) = self.sigue_travesura(t, dt) {
                    self.travesura = Some(t);
                }
            }
            None => {
                self.cada_travesura -= dt;
                if !kde::activo() {
                    self.roba_si_alcanza();
                }
                if self.travesura.is_none() && self.cada_travesura <= 0.0 {
                    self.cada_travesura = ENTRE.0 + self.azar_t() * (ENTRE.1 - ENTRE.0);
                    if !kde::activo() {
                        self.empieza_pisotones();
                    }
                }
            }
        }
    }

    fn escapan(&mut self, dt: f32) {
        for p in &mut self.mascotas {
            p.escapa_otra = (p.escapa_otra - dt).max(0.0);
        }
        let (Some(c), Some(a)) = (self.cursor, self.cursor_antes_t) else {
            return;
        };
        if dt <= 0.0 || c == a {
            return;
        }
        // el cursor se lee cada 0.1 s: la velocidad, con eso
        let rapido = (c.0 - a.0).hypot(c.1 - a.1) / dt.max(0.1) > ESCAPA_RAPIDO;
        for i in 0..self.mascotas.len() {
            let p = &self.mascotas[i];
            if p.escapa_otra > 0.0 || !p.bicho.puede_jugar() {
                continue;
            }
            // el cursor (global) en coordenadas de su monitor
            let (ox, oy) = self.origen_de(i);
            let (c, a) = ((c.0 - ox, c.1 - oy), (a.0 - ox, a.1 - oy));
            let b = &p.bicho;
            let (bx, by) = (b.x + b.cw / 2.0, b.y + b.ch / 2.0);
            let (d, d_antes) = ((c.0 - bx).hypot(c.1 - by), (a.0 - bx).hypot(a.1 - by));
            if d < ESCAPA_CERCA && d < d_antes && rapido {
                self.mascotas[i].escapa_otra = ESCAPA_OTRA;
                if self.azar_t() < 0.4 {
                    self.mascotas[i].bicho.esquiva(c.0 < bx);
                }
            }
        }
    }

    fn roba_si_alcanza(&mut self) {
        if self.cursor.is_none() || self.cursor_quieto < QUIETO {
            return;
        }
        for i in 0..self.mascotas.len() {
            let Some((cx, cy)) = self.cursor_de(i) else {
                continue;
            };
            let b = &self.mascotas[i].bicho;
            if !b.persigue() {
                continue;
            }
            let cerca = (cx - (b.x + b.cw / 2.0)).abs() < b.cw * 0.6
                && cy > b.y - b.ch * 0.6
                && cy < b.y + b.ch;
            if cerca && self.azar_t() < 0.5 {
                let der = self.azar_t() < 0.5;
                self.mascotas[i].bicho.corre(der, ROBO + 1.0);
                self.travesura = Some(Travesura::Roba {
                    i,
                    t: 0.0,
                    puso: None,
                });
                return;
            }
        }
    }

    fn empieza_pisotones(&mut self) {
        let candidatos: Vec<(usize, String)> = self
            .mascotas
            .iter()
            .enumerate()
            .filter(|(_, p)| p.bicho.puede_jugar())
            .filter_map(|(i, p)| {
                let (_, dentro, sobre) = p.bicho.suelo();
                (dentro.is_none()).then_some((i, sobre?.to_string()))
            })
            .filter(|(_, v)| hypr::flotante(v))
            .collect();
        if candidatos.is_empty() {
            self.cada_travesura = 15.0;
            return;
        }
        let (i, ventana) = candidatos
            [(self.azar_t() * candidatos.len() as f32) as usize % candidatos.len()]
        .clone();
        let medio = self.tam_de(i).0 / 2.0;
        let b = &mut self.mascotas[i].bicho;
        let der = b.x < medio;
        b.llama(der);
        let cayo = b.aterrizajes;
        self.travesura = Some(Travesura::Pisa {
            i,
            ventana,
            t: 0.0,
            cayo,
            hundida: None,
        });
    }

    fn sigue_travesura(&mut self, t: Travesura, dt: f32) -> Option<Travesura> {
        match t {
            Travesura::Roba { i, t, puso } => {
                let t = t + dt;
                // el cursor de verdad, cada cuadro (¿lo jalaste?), global
                // como lo dejó
                let ahora = hypr::cursor((0.0, 0.0));
                let (ox, oy) = self.origen_de(i);
                if let (Some(c), Some(puso)) = (ahora, puso)
                    && (c.0 - puso.0).hypot(c.1 - puso.1) > JALON
                {
                    let der = self.mascotas.get(i)?.bicho.x < c.0 - ox;
                    self.mascotas[i].bicho.esquiva(!der);
                    return None;
                }
                if t > ROBO {
                    let der = self.azar_t() < 0.5;
                    self.mascotas.get_mut(i)?.bicho.cariño(der);
                    return None;
                }
                let b = &self.mascotas.get(i)?.bicho;
                let (x, y) = (b.x + b.cw / 2.0 + ox, b.y - b.dy + b.ch * 0.15 + oy);
                hypr::mueve_cursor(x, y);
                self.cursor = Some((x, y));
                self.cursor_antes_t = Some((x, y));
                Some(Travesura::Roba {
                    i,
                    t,
                    puso: Some((x, y)),
                })
            }
            Travesura::Pisa {
                i,
                ventana,
                t,
                cayo,
                mut hundida,
            } => {
                let t = t + dt;
                if let Some(falta) = hundida.as_mut() {
                    *falta -= dt;
                    if *falta <= 0.0 {
                        hypr::empuja(&ventana, 0, -PISOTON);
                        hundida = None;
                    }
                }
                let b = &self.mascotas.get(i)?.bicho;
                let aire = b.en_el_aire();
                let (_, _, sobre) = b.suelo();
                let encima = sobre == Some(ventana.as_str());
                let ahora_cayo = b.aterrizajes;
                if ahora_cayo != cayo && encima && hundida.is_none() {
                    hypr::empuja(&ventana, 0, PISOTON);
                    hundida = Some(REGRESA);
                }
                if (t > 5.0 || (!aire && t > 0.3)) && hundida.is_none() {
                    return None;
                }
                Some(Travesura::Pisa {
                    i,
                    ventana,
                    t,
                    cayo: ahora_cayo,
                    hundida,
                })
            }
        }
    }

    /// Se acabó a la fuerza: que la ventana no se quede hundida.
    pub(crate) fn acaba_travesura(&mut self, t: Travesura) {
        if let Travesura::Pisa {
            ventana,
            hundida: Some(_),
            ..
        } = t
        {
            hypr::empuja(&ventana, 0, -PISOTON);
        }
    }
}
